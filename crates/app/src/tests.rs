//! Whole-screen rendering tests: the shell with real state, rendered to HTML.

use crate::screens::{Screen, Shell, ShellProps};
use crate::store::{Input, Saved, Settings};
use dioxus::prelude::*;
use merlo_core::{Attempt, Card, Clef, Guess, Letter, Midi, Millis, Naming, LESSONS};

fn render(saved: Saved, screen: Screen) -> String {
    let mut dom = VirtualDom::new_with_props(Shell, ShellProps::builder().saved(saved).screen(screen).build());
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    html.split_once("</style>").expect("the shell inlines its CSS").1.to_owned()
}

fn attempt(clef: Clef, note: &str, right: bool, ms: u32) -> Attempt {
    let pitch: merlo_core::Pitch = note.parse().unwrap();
    let guess = if right { Guess::Key(pitch.midi()) } else { Guess::Name(Letter::B) };
    Attempt { at: crate::platform::now(), card: Card { clef, pitch }, guess, reaction: Millis(ms) }
}

/// Forty quick right answers on the first lesson, plus a few bass mistakes.
fn practised() -> Saved {
    let mut attempts: Vec<_> =
        LESSONS[0].pool().iter().cycle().take(40).map(|c| attempt(c.clef, &c.pitch.to_string(), true, 700)).collect();
    attempts.extend([attempt(Clef::Bass, "C3", false, 900), attempt(Clef::Bass, "D3", true, 2500)]);
    Saved { attempts, ..Saved::default() }
}

#[test]
fn home_for_a_newcomer() {
    let html = render(Saved::default(), Screen::Home);
    assert!(html.contains("Hi, I&#39;m Merlo"), "{html}");
    assert_eq!(html.matches("lesson locked").count(), LESSONS.len() - 1);
    assert!(html.contains("Middle C") && html.contains("Grand staff"));
    assert!(html.contains("C D E") && html.contains("Do Re Mi"));
}

#[test]
fn home_after_practice_unlocks_the_next_lesson() {
    let html = render(practised(), Screen::Home);
    assert_eq!(html.matches("lesson locked").count(), LESSONS.len() - 2);
    assert!(html.contains("star on"));
    assert!(html.contains("🔥 1"));
}

#[test]
fn drill_with_names_and_with_piano() {
    let names = render(Saved::default(), Screen::Drill(0));
    assert!(names.contains("class=\"staff\"") && names.contains("namepad"));
    assert!(!names.contains("class=\"piano\""));

    let saved = Saved { settings: Settings { naming: Naming::Solfege, input: Input::Piano }, ..Saved::default() };
    let piano = render(saved, Screen::Drill(10));
    assert!(piano.contains("class=\"piano\"") && !piano.contains("namepad"));
}

#[test]
fn summary_shows_session_results_and_new_badges() {
    let saved = practised();
    let answers = saved.attempts[..20].to_vec();
    let html = render(Saved { attempts: answers.clone(), ..saved }, Screen::Summary { lesson: 0, answers });
    assert!(html.contains("100%") && html.contains("+300"));
    assert!(html.contains("New badge: Hatchling"));
    assert!(html.contains("Perfect pitch!"));
}

#[test]
fn summary_of_a_rough_session() {
    let answers = vec![attempt(Clef::Treble, "C4", false, 900)];
    let html = render(Saved { attempts: answers.clone(), ..Saved::default() }, Screen::Summary { lesson: 0, answers });
    assert!(html.contains("Keep practising!") && html.contains("0%") && html.contains("–"));
}

#[test]
fn stats_overview_heatmap_confusions_and_badges() {
    let html = render(practised(), Screen::Stats);
    assert!(html.contains("notes read") && html.contains(">42<"));
    assert_eq!(html.matches("class=\"staff\"").count(), 2);
    assert!(html.contains("C3 (Bass) read as") || html.contains("C3, Bass) read as"));
    assert!(html.contains("badge earned"));
    assert!(html.contains("not practised yet"));
    assert!(html.contains("class=\"series\""));
}

#[test]
fn stats_when_empty() {
    let html = render(Saved::default(), Screen::Stats);
    assert!(html.contains("No data yet"));
    assert!(!html.contains("badge earned"));
}

#[test]
fn unused_helpers_stay_honest() {
    assert_eq!(Midi(60).pitch().to_string(), "C4");
}

#[test]
fn home_shows_the_brand_and_every_top_pill_is_tappable() {
    let html = render(practised(), Screen::Home);
    assert!(html.contains("class=\"logo\"") && html.contains(">Merlo<"));
    let topbar = html.split("class=\"topbar\"").nth(1).unwrap().split("</header>").next().unwrap();
    assert_eq!(topbar.matches("<button").count(), 3, "streak, level and stats");
    assert!(!topbar.contains("<span class=\"pill"));
}

#[test]
fn home_offers_survivor() {
    let html = render(Saved::default(), Screen::Home);
    assert!(html.contains("mode-card") && html.contains("Survivor"));
    assert!(html.contains("class=\"app\""), "home keeps the narrow column");
}

#[test]
fn survivor_ranges_show_best_scores() {
    let saved = Saved { best: [("Bass staff".to_owned(), 42)].into(), ..Saved::default() };
    let html = render(saved, Screen::SurvivorPick);
    assert_eq!(html.matches("class=\"lesson\"").count(), merlo_core::survivor::RANGES.len());
    assert!(html.contains("🏆 42") && html.contains("34 notes"));
}

#[test]
fn survivor_run_starts_with_full_lives() {
    let html = render(Saved::default(), Screen::Survivor(0));
    assert!(html.contains("class=\"app wide\""));
    assert_eq!(html.matches("class=\"heart\"").count(), 3);
    assert_eq!(html.matches("class=\"staff\"").count(), 1);
    assert!(html.contains("hit-line") && html.contains("namepad") && !html.contains("game-over"));

    let piano = Saved { settings: Settings { naming: Naming::Letters, input: Input::Piano }, ..Saved::default() };
    let grand = render(piano, Screen::Survivor(5));
    assert_eq!(grand.matches("class=\"staff\"").count(), 2, "grand staff: one staff per clef");
    assert!(grand.contains("class=\"piano\""));
}

#[test]
fn game_over_card() {
    fn ssr(app: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }
    let record =
        ssr(|| rsx! { crate::survivor::GameOver { score: 30, record: 12, onagain: |()| {}, onback: |()| {} } });
    assert!(record.contains("New record!") && record.contains(">30<"));
    let plain = ssr(|| rsx! { crate::survivor::GameOver { score: 5, record: 12, onagain: |()| {}, onback: |()| {} } });
    assert!(plain.contains("Out of lives") && plain.contains(">12<"));
}

/// Set `MERLO_SNAPSHOTS=<dir>` to write every screen as a static page for visual review.
#[test]
fn snapshots() {
    let Ok(dir) = std::env::var("MERLO_SNAPSHOTS") else { return };
    let piano = Saved { settings: Settings { naming: Naming::Solfege, input: Input::Piano }, ..practised() };
    let answers = practised().attempts[..20].to_vec();
    let pages = [
        ("home-new", Saved::default(), Screen::Home),
        ("home", practised(), Screen::Home),
        ("drill-names", practised(), Screen::Drill(0)),
        ("drill-piano", piano, Screen::Drill(4)),
        ("summary", practised(), Screen::Summary { lesson: 0, answers }),
        ("stats", practised(), Screen::Stats),
        ("survivor-pick", practised(), Screen::SurvivorPick),
        (
            "survivor-grand",
            Saved { settings: Settings { naming: Naming::Letters, input: Input::Piano }, ..Saved::default() },
            Screen::Survivor(5),
        ),
        ("survivor-five", Saved::default(), Screen::Survivor(0)),
    ];
    std::fs::create_dir_all(&dir).unwrap();
    for (name, saved, screen) in pages {
        let mut dom = VirtualDom::new_with_props(Shell, ShellProps::builder().saved(saved).screen(screen).build());
        dom.rebuild_in_place();
        let page = format!(
            "<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'><body>{}</body>",
            dioxus_ssr::render(&dom)
        );
        std::fs::write(format!("{dir}/{name}.html"), page).unwrap();
    }
}
