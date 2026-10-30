//! Presentational building blocks: they take data and emit events, nothing else.

use crate::geometry::{keyboard, staff_lines, staff_y, KeyRect, HALF_GAP};
use crate::glyphs;
use dioxus::prelude::*;
use merlo_core::{Clef, Letter, Midi, Naming, StaffPos};

/// Feedback colouring for a note, key or button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    Plain,
    Right,
    Wrong,
}

impl Mark {
    pub const fn class(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::Right => "right",
            Self::Wrong => "wrong",
        }
    }
}

fn mark_of<T: PartialEq>(marks: &[(T, Mark)], item: &T) -> Mark {
    marks.iter().find(|(t, _)| t == item).map(|&(_, m)| m).unwrap_or_default()
}

/// A note to draw on a staff.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub pos: StaffPos,
    pub x: f64,
    /// A CSS colour.
    pub color: &'static str,
    /// Filled heads for overviews, whole notes for reading.
    pub filled: bool,
    pub title: String,
}

const GLYPH_SCALE: f64 = 2.0 * HALF_GAP / glyphs::UNITS_PER_SPACE;

#[component]
pub fn Staff(clef: Clef, notes: Vec<Note>, #[props(default = 260.0)] width: f64) -> Element {
    let (clef_path, clef_line) = match clef {
        Clef::Treble => (glyphs::G_CLEF, StaffPos(2)),
        Clef::Bass => (glyphs::F_CLEF, StaffPos(6)),
    };
    let clef_y = staff_y(clef_line);
    rsx! {
        svg {
            class: "staff",
            view_box: "0 22 {width} 156",
            role: "img",
            "aria-label": "{clef.name()} staff",
            for y in staff_lines() {
                line { x1: 4, x2: width - 4.0, y1: y, y2: y, class: "staff-line" }
            }
            path {
                d: clef_path,
                class: "clef",
                transform: "translate(12 {clef_y}) scale({GLYPH_SCALE})",
            }
            for note in notes {
                NoteHead { note }
            }
        }
    }
}

#[component]
fn NoteHead(note: Note) -> Element {
    let (glyph, units) = if note.filled {
        (glyphs::BLACK_NOTE, glyphs::BLACK_NOTE_WIDTH)
    } else {
        (glyphs::WHOLE_NOTE, glyphs::WHOLE_NOTE_WIDTH)
    };
    let half = units * GLYPH_SCALE / 2.0;
    let (x, y) = (note.x, staff_y(note.pos));
    rsx! {
        g { class: "note", style: "color: {note.color}",
            title { "{note.title}" }
            for ledger in note.pos.ledger_lines() {
                line {
                    x1: x - half - 5.0,
                    x2: x + half + 5.0,
                    y1: staff_y(ledger),
                    y2: staff_y(ledger),
                    class: "ledger",
                }
            }
            path {
                d: glyph,
                fill: "currentColor",
                transform: "translate({x - half} {y}) scale({GLYPH_SCALE})",
            }
        }
    }
}

const KEY_W: f64 = 24.0;
const KEY_H: f64 = 120.0;

/// An on-screen piano spanning whole octaves around `lo..=hi`.
#[component]
pub fn Piano(lo: Midi, hi: Midi, marks: Vec<(Midi, Mark)>, onpress: EventHandler<Midi>) -> Element {
    let keys = keyboard(lo, hi);
    let width = keys.iter().filter(|k| !k.is_black()).count() as f64 * KEY_W;
    rsx! {
        svg { class: "piano", view_box: "0 0 {width} {KEY_H}",
            for k in keys {
                g {
                    key: "{k.key.0}",
                    class: key_class(k, mark_of(&marks, &k.key)),
                    onpointerdown: move |_| onpress.call(k.key),
                    rect {
                        x: k.x * KEY_W,
                        y: 0,
                        width: k.width * KEY_W,
                        height: if k.is_black() { KEY_H * 0.62 } else { KEY_H },
                        rx: if k.is_black() { 2 } else { 4 },
                    }
                    if k.key.0 % 12 == 0 {
                        text { x: (k.x + 0.5) * KEY_W, y: KEY_H - 8.0, class: "key-label", "C{k.key.pitch().octave}" }
                    }
                }
            }
        }
    }
}

fn key_class(k: KeyRect, mark: Mark) -> String {
    format!("key {} {}", if k.is_black() { "black" } else { "white" }, mark.class())
}

/// Seven big buttons, one per note name.
#[component]
pub fn NamePad(naming: Naming, marks: Vec<(Letter, Mark)>, onpress: EventHandler<Letter>) -> Element {
    rsx! {
        div { class: "namepad",
            for letter in Letter::ALL {
                button {
                    key: "{letter:?}",
                    class: "name {mark_of(&marks, &letter).class()}",
                    onpointerdown: move |_| onpress.call(letter),
                    "{letter.name(naming)}"
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mood {
    #[default]
    Idle,
    Happy,
    Sad,
}

/// Merlo the blackbird: yellow eye-ring, orange beak, opinions about your reading.
#[component]
pub fn Mascot(#[props(default)] mood: Mood, #[props(default = 96.0)] size: f64) -> Element {
    let mood_class = match mood {
        Mood::Idle => "idle",
        Mood::Happy => "happy",
        Mood::Sad => "sad",
    };
    rsx! {
        svg { class: "mascot {mood_class}", width: size, height: size, view_box: "0 0 100 100", role: "img", "aria-label": "Merlo the blackbird",
            g { class: "bird",
                path { d: "M22 60 L3 76 L10 81 L30 66 Z", class: "plumage" }
                ellipse { cx: 47, cy: 60, rx: 29, ry: 20, class: "plumage" }
                ellipse { cx: 42, cy: 60, rx: 18, ry: 10, transform: "rotate(-14 42 60)", class: "wing" }
                circle { cx: 70, cy: 39, r: 16, class: "plumage" }
                path { d: "M84 35 L99 41 L84 46 Z", class: "beak" }
                circle { cx: 75, cy: 35, r: 4.5, class: "beak" }
                circle { cx: 75, cy: 35, r: 2.4, class: "pupil" }
                circle { cx: 76, cy: 34, r: 0.9, fill: "white" }
                path { d: "M44 78 L42 91 M53 78 L55 91 M38 91 L46 91 M51 91 L59 91", class: "legs" }
            }
            if mood == Mood::Happy {
                text { x: 86, y: 22, class: "song", "♪" }
            }
        }
    }
}

#[component]
pub fn StarRow(stars: u8) -> Element {
    rsx! {
        span { class: "stars", "aria-label": "{stars} of 3 stars",
            for i in 0..3u8 {
                span { class: if i < stars { "star on" } else { "star" }, "★" }
            }
        }
    }
}

/// A rounded progress bar, `value` in `0..=1`.
#[component]
pub fn Bar(value: f64, #[props(default)] class: String) -> Element {
    let pct = (value.clamp(0.0, 1.0) * 100.0).round();
    rsx! {
        div { class: "bar {class}", role: "progressbar", "aria-valuenow": pct,
            div { class: "fill", style: "width: {pct}%" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders inside a real virtual DOM, so event handlers have a runtime.
    fn render_element(app: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    fn note(pos: i8) -> Note {
        Note { pos: StaffPos(pos), x: 150.0, color: "red", filled: false, title: "C4".into() }
    }

    #[test]
    fn staff_draws_five_lines_clef_and_ledgers() {
        let html = render_element(|| rsx! { Staff { clef: Clef::Treble, notes: vec![note(-2)] } });
        assert_eq!(html.matches("staff-line").count(), 5);
        assert!(html.contains(glyphs::G_CLEF));
        assert_eq!(html.matches("class=\"ledger\"").count(), 1, "middle C needs one ledger line");
        assert!(html.contains(glyphs::WHOLE_NOTE) && html.contains("<title>C4</title>"));

        let bass =
            render_element(|| rsx! { Staff { clef: Clef::Bass, notes: vec![Note { filled: true, ..note(4) }] } });
        assert!(bass.contains(glyphs::F_CLEF) && bass.contains(glyphs::BLACK_NOTE));
        assert!(!bass.contains("class=\"ledger\""));
    }

    #[test]
    fn piano_marks_and_labels() {
        let html = render_element(|| {
            rsx! {
                Piano { lo: Midi(60), hi: Midi(64), marks: vec![(Midi(62), Mark::Right), (Midi(61), Mark::Wrong)], onpress: |_| {} }
            }
        });
        assert_eq!(html.matches("<rect").count(), 7, "B3..F4");
        assert_eq!(html.matches("key white").count(), 5);
        assert!(html.contains("key white right") && html.contains("key black wrong"));
        assert!(html.contains(">C4<"));
    }

    #[test]
    fn namepad_in_both_namings() {
        let letters = render_element(|| rsx! { NamePad { naming: Naming::Letters, marks: vec![], onpress: |_| {} } });
        assert_eq!(letters.matches("<button").count(), 7);
        let solfege = render_element(|| {
            rsx! {
                NamePad { naming: Naming::Solfege, marks: vec![(Letter::G, Mark::Right)], onpress: |_| {} }
            }
        });
        assert!(solfege.contains(">Sol<") && solfege.contains("name right"));
    }

    #[test]
    fn mascot_moods() {
        let happy = render_element(|| rsx! { Mascot { mood: Mood::Happy } });
        assert!(happy.contains("mascot happy") && happy.contains("♪"));
        let sad = render_element(|| rsx! { Mascot { mood: Mood::Sad, size: 40.0 } });
        assert!(sad.contains("mascot sad") && !sad.contains("♪"));
        assert!(render_element(|| rsx! { Mascot {} }).contains("mascot idle"));
    }

    #[test]
    fn stars_and_bar() {
        let stars = render_element(|| rsx! { StarRow { stars: 2 } });
        assert_eq!(stars.matches("star on").count(), 2);
        let bar = render_element(|| rsx! { Bar { value: 1.7 } });
        assert!(bar.contains("width: 100%"));
    }

    #[test]
    fn marks_default_to_plain() {
        assert_eq!(mark_of(&[(1, Mark::Wrong)], &2), Mark::Plain);
        assert_eq!(Mark::Plain.class(), "");
    }
}
