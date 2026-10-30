//! What survives a restart: settings and the attempt log, as one JSON document.
//! On the web it lives in `localStorage`; elsewhere in the user's data folder.

use merlo_core::{Attempt, Naming};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How the player answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Input {
    /// Seven big name buttons (or the letter keys).
    #[default]
    Names,
    /// An on-screen piano: the octave counts.
    Piano,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    pub naming: Naming,
    pub input: Input,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Saved {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    /// Best Survivor score per range title.
    #[serde(default)]
    pub best: BTreeMap<String, u32>,
}

impl Saved {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("plain data always serialises")
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

const KEY: &str = "merlo";

/// Loads the saved state. Unreadable data is set aside under a backup key
/// instead of being silently overwritten.
pub fn load() -> Saved {
    load_with(&backend::read, &backend::write)
}

pub fn save(saved: &Saved) {
    backend::write(KEY, &saved.to_json());
}

fn load_with(read: &dyn Fn(&str) -> Option<String>, write: &dyn Fn(&str, &str)) -> Saved {
    let Some(json) = read(KEY) else { return Saved::default() };
    Saved::from_json(&json).unwrap_or_else(|_| {
        write(&format!("{KEY}.corrupt.{}", crate::platform::now().0), &json);
        Saved::default()
    })
}

#[cfg(target_arch = "wasm32")]
mod backend {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn read(key: &str) -> Option<String> {
        storage()?.get_item(key).ok()?
    }

    pub fn write(key: &str, value: &str) {
        if let Some(s) = storage() {
            let _ = s.set_item(key, value);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::PathBuf;

    /// Android gives each app a private folder named after its bundle
    /// identifier (set in `Dioxus.toml`); elsewhere we use the OS data folder.
    pub const ANDROID_DIR: &str = "/data/data/app.merlo/files";

    fn dir() -> PathBuf {
        if cfg!(target_os = "android") {
            PathBuf::from(ANDROID_DIR)
        } else {
            dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("merlo")
        }
    }

    fn path(key: &str) -> PathBuf {
        dir().join(format!("{key}.json"))
    }

    pub fn read(key: &str) -> Option<String> {
        std::fs::read_to_string(path(key)).ok()
    }

    /// Writes atomically: a crash mid-write never leaves half a history.
    pub fn write(key: &str, value: &str) {
        let target = path(key);
        let tmp = target.with_extension("tmp");
        let _ = target
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .and_then(|_| std::fs::write(&tmp, value))
            .and_then(|()| std::fs::rename(&tmp, &target));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlo_core::{Card, Clef, Guess, Letter, Millis, Timestamp};
    use std::cell::RefCell;

    fn sample() -> Saved {
        let card = Card { clef: Clef::Bass, pitch: "D3".parse().unwrap() };
        Saved {
            settings: Settings { naming: Naming::Solfege, input: Input::Piano },
            attempts: vec![Attempt { at: Timestamp(1), card, guess: Guess::Name(Letter::D), reaction: Millis(700) }],
            best: BTreeMap::from([("Five notes".to_owned(), 12)]),
        }
    }

    #[test]
    fn json_roundtrip() {
        assert_eq!(Saved::from_json(&sample().to_json()).unwrap(), sample());
    }

    #[test]
    fn missing_fields_default() {
        assert_eq!(Saved::from_json("{}").unwrap(), Saved::default());
    }

    #[test]
    fn load_paths() {
        let written = RefCell::new(Vec::new());
        let write = |k: &str, v: &str| written.borrow_mut().push((k.to_owned(), v.to_owned()));

        assert_eq!(load_with(&|_| None, &write), Saved::default());
        assert_eq!(load_with(&|_| Some(sample().to_json()), &write), sample());
        assert_eq!(*written.borrow(), Vec::<(String, String)>::new());

        assert_eq!(load_with(&|_| Some("{oops".into()), &write), Saved::default());
        let backups = written.borrow();
        assert_eq!(backups.len(), 1);
        assert!(backups[0].0.starts_with("merlo.corrupt.") && backups[0].1 == "{oops");
    }

    #[test]
    fn android_folder_matches_the_bundle_identifier() {
        let config = include_str!("../Dioxus.toml");
        let id = backend::ANDROID_DIR.split('/').nth(3).unwrap();
        assert!(config.contains(&format!("identifier = \"{id}\"")), "update ANDROID_DIR when the identifier changes");
    }

    #[test]
    fn file_backend_roundtrip() {
        let key = format!("test-{}", std::process::id());
        backend::write(&key, "hello");
        assert_eq!(backend::read(&key).as_deref(), Some("hello"));
        backend::write(&key, "again");
        assert_eq!(backend::read(&key).as_deref(), Some("again"));
        assert!(backend::read("definitely-missing-key").is_none());
        let _ = std::fs::remove_file(dirs::data_dir().unwrap_or_default().join("merlo").join(format!("{key}.json")));
    }
}
