//! Survivor screens: choose a range, then keep up with the scrolling notes.

use crate::components::{Mascot, Mood, NamePad, Note, Piano, Staff};
use crate::geometry::scroll_x;
use crate::platform;
use crate::screens::{commit, letter_of, Screen, Tile};
use crate::store::{Input, Saved};
use dioxus::prelude::*;
use merlo_core::survivor::RANGES;
use merlo_core::{Clef, Guess, Hit, Midi, Survivor, Timestamp};

/// Time between animation frames.
const FRAME_MS: u64 = 33;
/// Staff geometry: notes enter at `END` and must be played before `LINE`.
const WIDTH: f64 = 440.0;
const LINE: f64 = 84.0;
const END: f64 = WIDTH - 18.0;

fn best(saved: &Saved, range: usize) -> u32 {
    saved.best.get(RANGES[range].title).copied().unwrap_or(0)
}

/// The notes of one clef as drawn now; the one to play next is highlighted.
fn staff_notes(game: &Survivor, clef: Clef, now: Timestamp) -> Vec<Note> {
    game.notes()
        .enumerate()
        .filter(|(_, n)| n.card.clef == clef)
        .map(|(i, n)| Note {
            pos: n.card.position(),
            x: scroll_x(n.progress(now), LINE, END),
            color: if i == 0 { "var(--beak-dark)" } else { "var(--ink)" },
            filled: false,
            title: String::new(),
        })
        .collect()
}

#[component]
pub fn Pick() -> Element {
    let saved = use_context::<Signal<Saved>>();
    let mut screen = use_context::<Signal<Screen>>();
    let data = saved.read();
    rsx! {
        header { class: "topbar",
            button { class: "pill ghost", onclick: move |_| screen.set(Screen::Home), "← Back" }
            h2 { "⚡ Survivor" }
        }
        p { class: "hint",
            "Notes scroll towards the line, faster and faster. Play each one before it arrives. A wrong key or a missed note costs a ♥; three and you're out."
        }
        ol { class: "path ranges",
            for (i, range) in RANGES.iter().enumerate() {
                li { key: "{i}", class: "lesson",
                    button { onclick: move |_| screen.set(Screen::Survivor(i)),
                        span { class: "node", "{i + 1}" }
                        span { class: "lesson-text",
                            strong { "{range.title}" }
                            small { "{range.hint} · {range.pool().len()} notes" }
                        }
                        span { class: "best", title: "Best score",
                            match best(&data, i) { 0 => "–".to_owned(), b => format!("🏆 {b}") }
                        }
                    }
                }
            }
        }
    }
}

/// A run in progress, or its result.
#[component]
pub fn Run(range: usize) -> Element {
    let saved = use_context::<Signal<Saved>>();
    let mut screen = use_context::<Signal<Screen>>();
    let new_game = move || Survivor::new(RANGES[range].pool(), platform::now(), platform::seed());
    let mut game = use_signal(new_game);
    let mut record_before = use_signal(|| best(&saved.read(), range));
    // Each hit re-keys the flash overlay, restarting its animation.
    let mut flash = use_signal(|| (0u32, Hit::Ignored));

    let record = move || {
        let score = game.read().score();
        commit(saved, |d| {
            let best = d.best.entry(RANGES[range].title.to_owned()).or_default();
            *best = (*best).max(score);
        });
    };

    use_future(move || async move {
        loop {
            platform::sleep(FRAME_MS).await;
            if !game.read().is_over() {
                game.write().tick(platform::now());
                if game.read().is_over() {
                    record();
                }
            }
        }
    });

    let mut play = move |guess: Guess| {
        let hit = game.write().play(guess);
        if hit != Hit::Ignored {
            let count = flash.read().0;
            flash.set((count + 1, hit));
            if game.read().is_over() {
                record();
            }
        }
    };

    let mut again = move || {
        record_before.set(best(&saved.read(), range));
        flash.set((0, Hit::Ignored));
        game.set(new_game());
    };

    let settings = saved.read().settings;
    let now = platform::now();
    let state = game.read();
    let pool = RANGES[range].pool();
    let clefs: Vec<Clef> =
        [Clef::Treble, Clef::Bass].into_iter().filter(|c| pool.iter().any(|p| p.clef == *c)).collect();
    let keys: Vec<Midi> = pool.iter().map(|c| c.pitch.midi()).collect();
    let (lo, hi) = (keys.iter().min().copied().unwrap_or(Midi(60)), keys.iter().max().copied().unwrap_or(Midi(60)));
    let speed = 6000.0 / f64::from(Survivor::travel_at(state.elapsed(now)).0);
    let (count, last_hit) = *flash.read();

    rsx! {
        div {
            class: "survivor",
            tabindex: 0,
            onmounted: move |e| async move { _ = e.set_focus(true).await; },
            onkeydown: move |e| {
                if let Key::Character(k) = e.key() {
                    if let Some(letter) = letter_of(&k) { play(Guess::Name(letter)); }
                } else if e.key() == Key::Escape {
                    screen.set(Screen::SurvivorPick);
                }
            },
            header { class: "topbar",
                button { class: "pill ghost", title: "Back", onclick: move |_| screen.set(Screen::SurvivorPick), "✕" }
                span { class: "pill lives", "aria-label": "{state.lives()} lives",
                    for i in 0..Survivor::LIVES {
                        span { class: if i < state.lives() { "heart" } else { "heart lost" }, "♥" }
                    }
                }
                span { class: "pill score", "{state.score()}" }
                span { class: "pill ghost speed", "×{speed:.1}" }
            }
            div { class: "stage",
                if count > 0 {
                    div { key: "{count}", class: if last_hit == Hit::Right { "flash right" } else { "flash wrong" } }
                }
                for clef in clefs {
                    Staff {
                        key: "{clef:?}",
                        clef,
                        width: WIDTH,
                        hit_line: LINE,
                        notes: staff_notes(&state, clef, now),
                    }
                }
                if state.is_over() {
                    GameOver {
                        score: state.score(),
                        record: record_before(),
                        onagain: move |()| again(),
                        onback: move |()| screen.set(Screen::SurvivorPick),
                    }
                }
            }
            match settings.input {
                Input::Names => rsx! { NamePad { naming: settings.naming, marks: vec![], onpress: move |l| play(Guess::Name(l)) } },
                Input::Piano => rsx! { Piano { lo, hi, marks: vec![], onpress: move |k| play(Guess::Key(k)) } },
            }
        }
    }
}

#[component]
pub fn GameOver(score: u32, record: u32, onagain: EventHandler, onback: EventHandler) -> Element {
    let beaten = score > record;
    rsx! {
        div { class: "game-over",
            Mascot { mood: if beaten { Mood::Happy } else { Mood::Sad }, size: 72.0 }
            h2 { if beaten { "New record!" } else { "Out of lives" } }
            div { class: "tiles",
                Tile { value: score.to_string(), label: "score" }
                Tile { value: score.max(record).to_string(), label: "best" }
            }
            div { class: "actions",
                button { class: "btn", onclick: move |_| onback.call(()), "Ranges" }
                button { class: "btn primary", onclick: move |_| onagain.call(()), "Again" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_scroll_left_and_the_next_one_is_highlighted() {
        let mut game = Survivor::new(RANGES[5].pool(), Timestamp(0), 3);
        game.tick(Timestamp(4500));
        let at = |t| {
            [Clef::Treble, Clef::Bass].into_iter().flat_map(|c| staff_notes(&game, c, Timestamp(t))).collect::<Vec<_>>()
        };
        let notes = at(4500);
        assert_eq!(notes.len(), 3);
        assert_eq!(notes.iter().filter(|n| n.color == "var(--beak-dark)").count(), 1);
        assert!(notes.iter().all(|n| (LINE..=END).contains(&n.x)));
        let later = at(5500);
        assert!(later.iter().zip(&notes).all(|(l, n)| l.x <= n.x), "everything moves towards the line");
    }
}
