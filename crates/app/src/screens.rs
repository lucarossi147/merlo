//! Screens. Each reads the shared state from context and renders core data.

use crate::components::{Bar, Logo, Mark, Mascot, Mood, NamePad, Note, Piano, Staff, StarRow};
use crate::geometry::{heat, plot, points};
use crate::platform;
use crate::store::{self, Input, Saved, Settings};
use crate::survivor;
use dioxus::prelude::*;
use merlo_core::curriculum::unlocked;
use merlo_core::stats::{self, confusions, group_by, DAILY_GOAL};
use merlo_core::{
    Attempt, Badge, Card, Clef, Day, Drill, Guess, Letter, Midi, Naming, Phase, Profile, StaffPos, Tally, LESSONS,
};

pub const CSS: &str = include_str!("../assets/main.css");

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Drill(usize),
    Summary { lesson: usize, answers: Vec<Attempt> },
    Stats,
    SurvivorPick,
    Survivor(usize),
}

impl Screen {
    /// Playing screens use the whole width, e.g. a phone in landscape.
    fn is_wide(&self) -> bool {
        matches!(self, Self::Drill(_) | Self::Survivor(_))
    }
}

/// Notes per drill session.
pub const SESSION: usize = 20;

/// Applies a change to the saved state and persists it.
pub(crate) fn commit(mut saved: Signal<Saved>, change: impl FnOnce(&mut Saved)) {
    change(&mut saved.write());
    store::save(&saved.read());
}

/// The root: owns the state and switches screens.
#[component]
pub fn Shell(saved: Saved, screen: Screen) -> Element {
    use_context_provider(|| Signal::new(saved));
    let screen = use_context_provider(|| Signal::new(screen));
    rsx! {
        style { {CSS} }
        main { class: if screen.read().is_wide() { "app wide" } else { "app" },
            match screen() {
                Screen::Home => rsx! { Home {} },
                Screen::Drill(lesson) => rsx! { DrillScreen { key: "{lesson}", lesson } },
                Screen::Summary { lesson, answers } => rsx! { Summary { lesson, answers } },
                Screen::Stats => rsx! { Stats {} },
                Screen::SurvivorPick => rsx! { survivor::Pick {} },
                Screen::Survivor(range) => rsx! { survivor::Run { key: "{range}", range } },
            }
        }
    }
}

#[component]
fn Home() -> Element {
    let saved = use_context::<Signal<Saved>>();
    let mut screen = use_context::<Signal<Screen>>();
    let data = saved.read();
    let profile = Profile::from_log(&data.attempts, platform::now().day());
    let open = unlocked(&data.attempts);
    let greeting = match (profile.attempts, profile.today_xp >= DAILY_GOAL) {
        (0, _) => "Hi, I'm Merlo! Let's read our first notes.",
        (_, true) => "Goal reached! I'm singing all day.",
        _ => "Ready for today's song?",
    };
    rsx! {
        div { class: "brand",
            Logo { size: 34.0 }
            span { class: "wordmark", "Merlo" }
        }
        header { class: "topbar",
            button { class: "pill streak", title: "Day streak", onclick: move |_| screen.set(Screen::Stats), "🔥 {profile.streak}" }
            button { class: "pill", title: "Level", onclick: move |_| screen.set(Screen::Stats), "Lv {profile.level.level}" }
            Bar { value: f64::from(profile.level.into) / f64::from(profile.level.span), class: "xp" }
            button { class: "pill ghost", onclick: move |_| screen.set(Screen::Stats), "📈 Stats" }
        }
        section { class: "hero",
            Mascot { mood: if profile.today_xp >= DAILY_GOAL { Mood::Happy } else { Mood::Idle } }
            div {
                p { class: "bubble", "{greeting}" }
                p { class: "goal-label", "Today {profile.today_xp.min(DAILY_GOAL)} / {DAILY_GOAL} XP" }
                Bar { value: profile.goal_progress(), class: "goal" }
            }
        }
        button { class: "mode-card", onclick: move |_| screen.set(Screen::SurvivorPick),
            span { class: "mode-icon", "⚡" }
            span { class: "lesson-text",
                strong { "Survivor" }
                small { "Notes rush in faster and faster. How long can you last?" }
            }
        }
        ol { class: "path",
            for (i, lesson) in LESSONS.iter().enumerate() {
                li { key: "{i}", class: if i < open { "lesson" } else { "lesson locked" },
                    button {
                        disabled: i >= open,
                        onclick: move |_| screen.set(Screen::Drill(i)),
                        span { class: "node", if i < open { "{i + 1}" } else { "🔒" } }
                        span { class: "lesson-text",
                            strong { "{lesson.title}" }
                            small { "{lesson.hint}" }
                        }
                        if i < open {
                            StarRow { stars: lesson.stars(&data.attempts).0 }
                        }
                    }
                }
            }
        }
        SettingsBar { settings: data.settings }
    }
}

#[component]
fn SettingsBar(settings: Settings) -> Element {
    let saved = use_context::<Signal<Saved>>();
    let set = move |s: Settings| commit(saved, |d| d.settings = s);
    let toggle = |on: bool| if on { "seg on" } else { "seg" };
    rsx! {
        footer { class: "settings",
            div { class: "segmented",
                button { class: toggle(settings.naming == Naming::Letters),
                    onclick: move |_| set(Settings { naming: Naming::Letters, ..settings }), "C D E" }
                button { class: toggle(settings.naming == Naming::Solfege),
                    onclick: move |_| set(Settings { naming: Naming::Solfege, ..settings }), "Do Re Mi" }
            }
            div { class: "segmented",
                button { class: toggle(settings.input == Input::Names),
                    onclick: move |_| set(Settings { input: Input::Names, ..settings }), "Names" }
                button { class: toggle(settings.input == Input::Piano),
                    onclick: move |_| set(Settings { input: Input::Piano, ..settings }), "Piano" }
            }
        }
    }
}

/// Right answers at the end of the session so far.
fn combo(answers: &[Attempt]) -> usize {
    answers.iter().rev().take_while(|a| a.correct()).count()
}

/// Marks for the answer pads: the right answer green, a wrong guess red.
fn marks<T: PartialEq + Copy>(
    phase: &Phase,
    of: impl Fn(Guess) -> Option<T>,
    target: impl Fn(Card) -> T,
) -> Vec<(T, Mark)> {
    match phase {
        Phase::Feedback(a) if a.correct() => vec![(target(a.card), Mark::Right)],
        Phase::Feedback(a) => {
            of(a.guess).map(|g| (g, Mark::Wrong)).into_iter().chain([(target(a.card), Mark::Right)]).collect()
        }
        _ => vec![],
    }
}

pub(crate) fn letter_of(key: &str) -> Option<Letter> {
    let mut chars = key.chars().map(|c| c.to_ascii_uppercase());
    match (chars.next(), chars.next()) {
        (Some(c), None) => Letter::ALL.into_iter().find(|l| l.name(Naming::Letters).starts_with(c)),
        _ => None,
    }
}

#[component]
fn DrillScreen(lesson: usize) -> Element {
    let saved = use_context::<Signal<Saved>>();
    let mut screen = use_context::<Signal<Screen>>();
    let mut drill = use_signal(|| {
        Drill::new(LESSONS[lesson].pool(), &saved.read().attempts, SESSION, platform::now(), platform::seed())
    });
    let settings = saved.read().settings;

    let mut answer = move |guess: Guess| {
        let Some(attempt) = drill.write().answer(guess, platform::now()).cloned() else { return };
        commit(saved, |d| d.attempts.push(attempt.clone()));
        spawn(async move {
            platform::sleep(if attempt.correct() { 450 } else { 1400 }).await;
            drill.write().advance(platform::now());
            if *drill.read().phase() == Phase::Done {
                screen.set(Screen::Summary { lesson, answers: drill.read().answers().to_vec() });
            }
        });
    };

    let state = drill.read();
    let phase = state.phase().clone();
    let (done, total) = state.progress();
    let streak = combo(state.answers());
    let (card, color, mood) = match &phase {
        Phase::Asking { card, .. } => (Some(*card), "var(--ink)", Mood::Idle),
        Phase::Feedback(a) if a.correct() => (Some(a.card), "var(--good)", Mood::Happy),
        Phase::Feedback(a) => (Some(a.card), "var(--bad)", Mood::Sad),
        Phase::Done => (None, "var(--ink)", Mood::Idle),
    };
    let keys: Vec<Midi> = LESSONS[lesson].pool().iter().map(|c| c.pitch.midi()).collect();
    let (lo, hi) = (keys.iter().min().copied().unwrap_or(Midi(60)), keys.iter().max().copied().unwrap_or(Midi(60)));

    rsx! {
        div {
            class: "drill",
            tabindex: 0,
            onmounted: move |e| async move { _ = e.set_focus(true).await; },
            onkeydown: move |e| {
                if let Key::Character(k) = e.key() {
                    if let Some(letter) = letter_of(&k) { answer(Guess::Name(letter)); }
                } else if e.key() == Key::Escape {
                    screen.set(Screen::Home);
                }
            },
            header { class: "topbar",
                button { class: "pill ghost", title: "Back", onclick: move |_| screen.set(Screen::Home), "✕" }
                Bar { value: done as f64 / total as f64, class: "session" }
                span { class: if streak >= 3 { "pill combo hot" } else { "pill combo" }, "🔥 {streak}" }
            }
            div { class: "stage",
                Mascot { mood, size: 64.0 }
                if let Some(card) = card {
                    Staff {
                        clef: card.clef,
                        notes: vec![Note { pos: card.position(), x: 160.0, color, filled: false, title: String::new() }],
                    }
                }
                p { class: "feedback",
                    if let Phase::Feedback(a) = &phase {
                        if a.correct() { "{a.reaction.seconds():.2} s" } else { "That was {a.card.pitch.name(settings.naming)} ({a.card.pitch})" }
                    }
                }
            }
            match settings.input {
                Input::Names => rsx! {
                    NamePad {
                        naming: settings.naming,
                        marks: marks(&phase, |g| match g { Guess::Name(l) => Some(l), Guess::Key(k) => Some(k.pitch().letter) }, |c| c.pitch.letter),
                        onpress: move |l| answer(Guess::Name(l)),
                    }
                },
                Input::Piano => rsx! {
                    Piano {
                        lo, hi,
                        marks: marks(&phase, |g| match g { Guess::Key(k) => Some(k), Guess::Name(_) => None }, |c| c.pitch.midi()),
                        onpress: move |k| answer(Guess::Key(k)),
                    }
                },
            }
        }
    }
}

#[component]
fn Summary(lesson: usize, answers: Vec<Attempt>) -> Element {
    let saved = use_context::<Signal<Saved>>();
    let mut screen = use_context::<Signal<Screen>>();
    let data = saved.read();
    let today = platform::now().day();
    let before = &data.attempts[..data.attempts.len().saturating_sub(answers.len())];
    let (old, new) = (Profile::from_log(before, today), Profile::from_log(&data.attempts, today));
    let fresh: Vec<_> = Badge::ALL.into_iter().filter(|b| b.earned(&new) && !b.earned(&old)).collect();
    let session = Tally::of(&answers);
    let accuracy = session.accuracy().unwrap_or(0.0);
    let gained: u32 = answers.iter().map(stats::xp).sum();
    rsx! {
        section { class: "summary",
            Mascot { mood: if accuracy >= 0.8 { Mood::Happy } else { Mood::Sad }, size: 120.0 }
            h2 { if accuracy >= 0.95 { "Perfect pitch!" } else if accuracy >= 0.8 { "Lovely song!" } else { "Keep practising!" } }
            StarRow { stars: LESSONS[lesson].stars(&data.attempts).0 }
            div { class: "tiles",
                Tile { value: format!("{:.0}%", accuracy * 100.0), label: "accuracy" }
                Tile { value: session.median().map_or("–".into(), |m| format!("{:.2} s", m.seconds())), label: "median time" }
                Tile { value: format!("+{gained}"), label: "XP" }
            }
            for badge in fresh {
                p { class: "new-badge", "{badge.info().0} New badge: {badge.info().1}" }
            }
            div { class: "actions",
                button { class: "btn", onclick: move |_| screen.set(Screen::Drill(lesson)), "Again" }
                button { class: "btn primary", onclick: move |_| screen.set(Screen::Home), "Continue" }
            }
        }
    }
}

#[component]
pub(crate) fn Tile(value: String, label: &'static str) -> Element {
    rsx! {
        div { class: "tile", strong { "{value}" } small { "{label}" } }
    }
}

/// Daily series over the last `days` days, ending today.
fn daily(log: &[Attempt], today: Day, days: i32, metric: impl Fn(&Tally) -> Option<f64>) -> Vec<Option<f64>> {
    let by_day = group_by(log, |a| a.at.day());
    (today.0 - days + 1..=today.0).map(|d| by_day.get(&Day(d)).and_then(&metric)).collect()
}

/// One staff per clef, every note coloured by your median reaction time.
fn heat_notes(log: &[Attempt], clef: Clef, naming: Naming) -> Vec<Note> {
    let tallies = group_by(log, |a| a.card);
    (-4..=12)
        .map(|p| {
            let card = Card { clef, pitch: clef.pitch_at(StaffPos(p)) };
            let tally = tallies.get(&card);
            let median = tally.and_then(Tally::median);
            let detail = tally.map_or("not practised yet".into(), |t| {
                format!(
                    "{} · {:.0}% right",
                    median.map_or("–".into(), |m| format!("{:.2} s", m.seconds())),
                    t.accuracy().unwrap_or(0.0) * 100.0
                )
            });
            Note {
                pos: StaffPos(p),
                x: 72.0 + f64::from(p + 4) * 18.5,
                color: heat(median.map(|m| m.0)),
                filled: true,
                title: format!("{} ({}): {detail}", card.pitch.name(naming), card.pitch),
            }
        })
        .collect()
}

#[component]
fn Stats() -> Element {
    let saved = use_context::<Signal<Saved>>();
    let mut screen = use_context::<Signal<Screen>>();
    let data = saved.read();
    let log = &data.attempts;
    let today = platform::now().day();
    let profile = Profile::from_log(log, today);
    let all = Tally::of(log);
    let speed = daily(log, today, 14, |t| t.median().map(merlo_core::Millis::seconds));
    let accuracy = daily(log, today, 14, |t| t.accuracy().map(|a| a * 100.0));
    let naming = data.settings.naming;
    rsx! {
        header { class: "topbar",
            button { class: "pill ghost", onclick: move |_| screen.set(Screen::Home), "← Back" }
            h2 { "Your reading" }
        }
        div { class: "tiles",
            Tile { value: all.seen.to_string(), label: "notes read" }
            Tile { value: all.accuracy().map_or("–".into(), |a| format!("{:.0}%", a * 100.0)), label: "accuracy" }
            Tile { value: all.median().map_or("–".into(), |m| format!("{:.2} s", m.seconds())), label: "median time" }
            Tile { value: format!("{} d", profile.best_streak), label: "best streak" }
        }
        Chart { title: "Median reaction time (s), last 14 days", values: speed, range: (0.0, 4.0), class: "speed" }
        Chart { title: "Accuracy (%), last 14 days", values: accuracy, range: (0.0, 100.0), class: "accuracy" }
        h3 { "Where you're slow" }
        p { class: "hint", "Each note is coloured by your median time: green is fluent, red needs work. Tap or hover for details." }
        for clef in [Clef::Treble, Clef::Bass] {
            Staff { key: "{clef:?}", clef, notes: heat_notes(log, clef, naming), width: 380.0 }
        }
        h3 { "Most confused" }
        ul { class: "confusions",
            for (card, guess, n) in confusions(log).into_iter().take(5) {
                li { key: "{card:?}{guess:?}",
                    "{card.pitch.name(naming)} ({card.pitch}, {card.clef.name()}) read as "
                    strong { {guess_name(guess, naming)} }
                    " × {n}"
                }
            }
        }
        h3 { "Badges" }
        div { class: "badges",
            for badge in Badge::ALL {
                div { key: "{badge:?}", class: if badge.earned(&profile) { "badge earned" } else { "badge" }, title: "{badge.info().2}",
                    span { class: "icon", "{badge.info().0}" }
                    small { "{badge.info().1}" }
                }
            }
        }
    }
}

fn guess_name(guess: Guess, naming: Naming) -> String {
    match guess {
        Guess::Name(l) => l.name(naming).to_owned(),
        Guess::Key(k) => format!("{} ({})", k.pitch().name(naming), k.pitch()),
    }
}

#[component]
fn Chart(title: &'static str, values: Vec<Option<f64>>, range: (f64, f64), class: &'static str) -> Element {
    let pts = plot(&values, 300.0, 80.0, range);
    let line = points(&pts);
    rsx! {
        figure { class: "chart {class}",
            figcaption { "{title}" }
            svg { view_box: "-4 -4 308 88",
                line { x1: 0, x2: 300, y1: 80, y2: 80, class: "axis" }
                polyline { points: line, class: "series" }
                for (x, y) in pts {
                    circle { cx: x, cy: y, r: 3, class: "dot" }
                }
            }
            if values.iter().all(Option::is_none) {
                p { class: "hint", "No data yet: play a lesson!" }
            }
        }
    }
}
