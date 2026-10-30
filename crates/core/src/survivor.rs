//! Survivor: notes scroll towards a line, faster and faster. Play each one
//! before it arrives. A wrong key or a note that reaches the line costs a life;
//! the run ends after three.
//!
//! Like [`crate::Drill`], this is a pure state machine driven by timestamps:
//! the app calls [`Survivor::tick`] on every frame and [`Survivor::play`] on
//! every key press.

use crate::curriculum::{span, Filter::All, Lesson};
use crate::drill::Guess;
use crate::rng::Rng;
use crate::staff::{Card, Clef::Bass, Clef::Treble};
use crate::time::{Millis, Timestamp};
use std::collections::VecDeque;

/// Note ranges to survive in, from narrow and easy to wide and hard.
pub const RANGES: &[Lesson] = &[
    Lesson { title: "Five notes", hint: "Middle C to G", spans: &[span(Treble, -2, 2, All)] },
    Lesson { title: "Treble staff", hint: "E4 to F5, no ledgers", spans: &[span(Treble, 0, 8, All)] },
    Lesson { title: "Bass staff", hint: "G2 to A3, no ledgers", spans: &[span(Bass, 0, 8, All)] },
    Lesson { title: "Treble + ledgers", hint: "A3 to C6", spans: &[span(Treble, -4, 12, All)] },
    Lesson { title: "Bass + ledgers", hint: "C2 to E4", spans: &[span(Bass, -4, 12, All)] },
    Lesson {
        title: "Grand staff",
        hint: "Both clefs, C2 to C6",
        spans: &[span(Treble, -4, 12, All), span(Bass, -4, 12, All)],
    },
];

/// A note on its way to the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Falling {
    pub card: Card,
    pub born: Timestamp,
    /// How long it takes to cross the staff.
    pub travel: Millis,
}

impl Falling {
    /// 0 when it appears on the right, 1 when it reaches the line.
    pub fn progress(&self, now: Timestamp) -> f64 {
        f64::from(now.since(self.born).0) / f64::from(self.travel.0.max(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Right,
    Wrong,
    /// Nothing to play, or the run is over.
    Ignored,
}

#[derive(Debug, Clone)]
pub struct Survivor {
    pool: Vec<Card>,
    notes: VecDeque<Falling>,
    lives: u8,
    score: u32,
    start: Timestamp,
    next_spawn: Timestamp,
    rng: Rng,
}

impl Survivor {
    pub const LIVES: u8 = 3;
    /// Roughly how many notes are on the staff at once.
    pub const ON_SCREEN: u32 = 3;
    const FIRST_TRAVEL_MS: f64 = 6000.0;
    const FASTEST_TRAVEL_MS: f64 = 1200.0;
    /// Travel time shrinks by 1.5% every second.
    const DECAY_PER_SECOND: f64 = 0.985;

    /// Crossing time for a note that appears `elapsed` into the run.
    pub fn travel_at(elapsed: Millis) -> Millis {
        let ms = Self::FIRST_TRAVEL_MS * Self::DECAY_PER_SECOND.powf(elapsed.seconds());
        Millis(ms.max(Self::FASTEST_TRAVEL_MS) as u32)
    }

    /// # Panics
    /// If `pool` is empty.
    pub fn new(pool: Vec<Card>, now: Timestamp, seed: u64) -> Self {
        assert!(!pool.is_empty(), "survivor needs at least one card");
        Self {
            pool,
            notes: VecDeque::new(),
            lives: Self::LIVES,
            score: 0,
            start: now,
            next_spawn: now,
            rng: Rng::new(seed),
        }
    }

    /// Notes on screen, oldest (the one to play) first.
    pub fn notes(&self) -> impl Iterator<Item = &Falling> {
        self.notes.iter()
    }

    pub fn lives(&self) -> u8 {
        self.lives
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn is_over(&self) -> bool {
        self.lives == 0
    }

    /// How long the run has lasted.
    pub fn elapsed(&self, now: Timestamp) -> Millis {
        now.since(self.start)
    }

    /// Advances time: spawns due notes, and charges a life for each note that reached the line.
    pub fn tick(&mut self, now: Timestamp) {
        while !self.is_over() && self.next_spawn <= now {
            self.spawn();
        }
        while !self.is_over() && self.notes.front().is_some_and(|n| n.progress(now) >= 1.0) {
            self.notes.pop_front();
            self.lose_life();
        }
    }

    /// Plays a key or name against the oldest note on screen.
    pub fn play(&mut self, guess: Guess) -> Hit {
        let Some(target) = self.notes.front().filter(|_| !self.is_over()) else { return Hit::Ignored };
        if guess.matches(target.card.pitch) {
            self.notes.pop_front();
            self.score += 1;
            Hit::Right
        } else {
            self.lose_life();
            Hit::Wrong
        }
    }

    fn lose_life(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        if self.is_over() {
            self.notes.clear();
        }
    }

    fn spawn(&mut self) {
        let travel = Self::travel_at(self.next_spawn.since(self.start));
        let last = self.notes.back().map(|n| n.card);
        let choices: Vec<Card> =
            self.pool.iter().copied().filter(|&c| self.pool.len() == 1 || Some(c) != last).collect();
        let card = choices[(self.rng.unit() * choices.len() as f64) as usize];
        self.notes.push_back(Falling { card, born: self.next_spawn, travel });
        self.next_spawn = Timestamp(self.next_spawn.0 + i64::from(travel.0 / Self::ON_SCREEN));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Letter, Midi};

    fn card(s: &str) -> Card {
        Card { clef: Treble, pitch: s.parse().unwrap() }
    }

    fn game() -> Survivor {
        Survivor::new(["C4", "D4", "E4"].map(card).to_vec(), Timestamp(0), 7)
    }

    fn right(s: &Survivor) -> Guess {
        Guess::Key(s.notes().next().unwrap().card.pitch.midi())
    }

    #[test]
    fn speed_rises_then_levels_off() {
        let at = |s: u32| Survivor::travel_at(Millis(s * 1000)).0;
        assert_eq!(at(0), 6000);
        assert!(at(10) < at(0) && at(60) < at(10));
        assert_eq!(at(1000), 1200, "never faster than the floor");
    }

    #[test]
    fn notes_spawn_on_schedule() {
        let mut s = game();
        s.tick(Timestamp(0));
        assert_eq!(s.notes().count(), 1, "the first note appears at once");
        s.tick(Timestamp(1999));
        assert_eq!(s.notes().count(), 1);
        s.tick(Timestamp(2000));
        assert_eq!(s.notes().count(), 2, "then every travel / 3");
        let cards: Vec<_> = s.notes().map(|n| n.card).collect();
        assert_ne!(cards[0], cards[1], "no immediate repeats");
    }

    #[test]
    fn progress_runs_from_right_to_line() {
        let n = Falling { card: card("C4"), born: Timestamp(1000), travel: Millis(2000) };
        assert!(n.progress(Timestamp(1000)).abs() < f64::EPSILON);
        assert!((n.progress(Timestamp(2000)) - 0.5).abs() < f64::EPSILON);
        assert!(n.progress(Timestamp(3000)) >= 1.0);
    }

    #[test]
    fn playing_right_scores_and_wrong_costs_a_life() {
        let mut s = game();
        assert_eq!(s.play(Guess::Name(Letter::C)), Hit::Ignored, "nothing on screen yet");
        s.tick(Timestamp(0));
        assert_eq!(s.play(right(&s)), Hit::Right);
        assert_eq!((s.score(), s.notes().count()), (1, 0));

        s.tick(Timestamp(2000));
        assert_eq!(s.play(Guess::Key(Midi(0))), Hit::Wrong);
        assert_eq!((s.lives(), s.notes().count()), (2, 1), "the note stays after a wrong key");
    }

    #[test]
    fn notes_reaching_the_line_cost_lives_until_game_over() {
        let mut s = game();
        s.tick(Timestamp(0));
        s.tick(Timestamp(6000));
        assert_eq!(s.lives(), 2, "the first note crossed in 6 s");
        s.tick(Timestamp(60_000));
        assert!(s.is_over());
        assert_eq!(s.notes().count(), 0, "the staff is cleared");
        assert_eq!(s.play(Guess::Name(Letter::C)), Hit::Ignored);
        assert_eq!(s.elapsed(Timestamp(60_000)), Millis(60_000));
    }

    #[test]
    fn three_wrong_keys_end_the_run() {
        let mut s = game();
        s.tick(Timestamp(0));
        for _ in 0..Survivor::LIVES {
            s.play(Guess::Key(Midi(0)));
        }
        assert!(s.is_over() && s.score() == 0);
    }

    #[test]
    fn notes_expire_in_the_order_they_appear() {
        let mut s = game();
        s.tick(Timestamp(300_000));
        let deadlines: Vec<_> = s.notes().map(|n| n.born.0 + i64::from(n.travel.0)).collect();
        assert!(deadlines.windows(2).all(|w| w[0] <= w[1]), "{deadlines:?}");
    }

    #[test]
    fn single_card_pool_repeats() {
        let mut s = Survivor::new(vec![card("G4")], Timestamp(0), 1);
        s.tick(Timestamp(2000));
        assert!(s.notes().all(|n| n.card == card("G4")) && s.notes().count() == 2);
    }

    #[test]
    #[should_panic(expected = "at least one card")]
    fn empty_pool_panics() {
        let _ = Survivor::new(vec![], Timestamp(0), 1);
    }

    #[test]
    fn ranges_get_wider() {
        let sizes: Vec<_> = RANGES.iter().map(|r| r.pool().len()).collect();
        assert_eq!(sizes, [5, 9, 9, 17, 17, 34]);
    }
}
