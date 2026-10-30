//! A drill session: show a card, take a guess, give feedback, repeat.
//!
//! The drill is a small state machine whose [`Phase`] makes illegal states
//! unrepresentable: you can only answer while a card is shown, and feedback
//! always carries the attempt it refers to.

use crate::pitch::{Letter, Midi, Pitch};
use crate::rng::Rng;
use crate::staff::Card;
use crate::stats::{self, Tally};
use crate::time::{Millis, Timestamp};
use serde::{Deserialize, Serialize};

/// What the player answered: a piano key, or just the note's letter name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Guess {
    Key(Midi),
    Name(Letter),
}

impl Guess {
    /// A key must be exact (octave included, enharmonics allowed);
    /// a name only has to match the letter.
    pub fn matches(self, target: Pitch) -> bool {
        match self {
            Self::Key(key) => key == target.midi(),
            Self::Name(letter) => letter == target.letter,
        }
    }
}

/// One answered card: the atom of the whole history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub at: Timestamp,
    pub card: Card,
    pub guess: Guess,
    pub reaction: Millis,
}

impl Attempt {
    pub fn correct(&self) -> bool {
        self.guess.matches(self.card.pitch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Asking { card: Card, shown_at: Timestamp },
    Feedback(Attempt),
    Done,
}

#[derive(Debug, Clone)]
pub struct Drill {
    pool: Vec<Card>,
    window: Vec<Attempt>,
    answers: Vec<Attempt>,
    length: usize,
    phase: Phase,
    rng: Rng,
}

impl Drill {
    /// How many past attempts steer the choice of the next card.
    pub const WINDOW: usize = 300;

    /// # Panics
    /// If `pool` is empty.
    pub fn new(pool: Vec<Card>, history: &[Attempt], length: usize, now: Timestamp, seed: u64) -> Self {
        assert!(!pool.is_empty(), "a drill needs at least one card");
        let window = history[history.len().saturating_sub(Self::WINDOW)..].to_vec();
        let mut drill = Self { pool, window, answers: Vec::new(), length, phase: Phase::Done, rng: Rng::new(seed) };
        drill.phase = drill.ask(now);
        drill
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn answers(&self) -> &[Attempt] {
        &self.answers
    }

    /// `(answered, total)`.
    pub fn progress(&self) -> (usize, usize) {
        (self.answers.len(), self.length)
    }

    /// Answer the shown card. Ignored (returns `None`) in any other phase.
    pub fn answer(&mut self, guess: Guess, now: Timestamp) -> Option<&Attempt> {
        let Phase::Asking { card, shown_at } = self.phase else { return None };
        let attempt = Attempt { at: now, card, guess, reaction: now.since(shown_at) };
        self.answers.push(attempt.clone());
        self.window.push(attempt.clone());
        self.phase = Phase::Feedback(attempt);
        match &self.phase {
            Phase::Feedback(a) => Some(a),
            _ => None,
        }
    }

    /// Leave the feedback and show the next card, or finish.
    pub fn advance(&mut self, now: Timestamp) {
        if matches!(self.phase, Phase::Feedback(_)) {
            self.phase = if self.answers.len() >= self.length { Phase::Done } else { self.ask(now) };
        }
    }

    fn ask(&mut self, now: Timestamp) -> Phase {
        let tallies = stats::group_by(&self.window, |a| a.card);
        let last = self.answers.last().map(|a| a.card);
        let weighted: Vec<_> = self
            .pool
            .iter()
            .filter(|&&c| self.pool.len() == 1 || Some(c) != last)
            .map(|&c| (c, need(tallies.get(&c))))
            .collect();
        Phase::Asking { card: weighted_pick(&weighted, self.rng.unit()), shown_at: now }
    }
}

/// How much a card needs practice: unseen, wrong and slow cards come up more.
pub fn need(tally: Option<&Tally>) -> f64 {
    tally
        .and_then(|t| Some((t.accuracy()?, t.median().unwrap_or(Millis(4000)))))
        .map_or(4.0, |(accuracy, median)| 1.0 + 4.0 * (1.0 - accuracy) + median.seconds().min(4.0))
}

/// Picks an item with probability proportional to its weight; `r` is in `[0, 1)`.
fn weighted_pick<T: Copy>(items: &[(T, f64)], r: f64) -> T {
    let total: f64 = items.iter().map(|(_, w)| w).sum();
    let mut target = r * total;
    items
        .iter()
        .find(|(_, w)| {
            target -= w;
            target < 0.0
        })
        .or(items.last())
        .map(|&(item, _)| item)
        .expect("non-empty pool")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::staff::Clef;

    fn card(s: &str) -> Card {
        Card { clef: Clef::Treble, pitch: s.parse().unwrap() }
    }

    fn pool() -> Vec<Card> {
        ["C4", "D4", "E4"].map(card).to_vec()
    }

    fn shown(drill: &Drill) -> Card {
        match drill.phase() {
            Phase::Asking { card, .. } => *card,
            other => panic!("not asking: {other:?}"),
        }
    }

    #[test]
    fn guesses() {
        let c4 = "C4".parse().unwrap();
        assert!(Guess::Key(Midi(60)).matches(c4));
        assert!(!Guess::Key(Midi(72)).matches(c4), "octave matters on keys");
        assert!(Guess::Name(Letter::C).matches(c4));
        assert!(!Guess::Name(Letter::D).matches(c4));
    }

    #[test]
    fn full_session_flow() {
        let mut drill = Drill::new(pool(), &[], 3, Timestamp(1000), 1);
        assert_eq!(drill.progress(), (0, 3));
        drill.advance(Timestamp(1100)); // no-op while asking
        let target = shown(&drill);

        let attempt = drill.answer(Guess::Key(target.pitch.midi()), Timestamp(1800)).unwrap();
        assert!(attempt.correct());
        assert_eq!(attempt.reaction, Millis(800));
        assert!(drill.answer(Guess::Name(Letter::C), Timestamp(1900)).is_none(), "one answer per card");

        drill.advance(Timestamp(2000));
        assert_ne!(shown(&drill), target, "never the same card twice in a row");
        for t in [3000, 4000] {
            drill.answer(Guess::Name(Letter::B), Timestamp(t));
            drill.advance(Timestamp(t + 100));
        }
        assert_eq!(drill.phase(), &Phase::Done);
        assert_eq!(drill.answers().len(), 3);
        assert!(drill.answer(Guess::Name(Letter::B), Timestamp(9000)).is_none());
    }

    #[test]
    fn single_card_pool_may_repeat() {
        let mut drill = Drill::new(vec![card("G4")], &[], 2, Timestamp(0), 9);
        drill.answer(Guess::Name(Letter::G), Timestamp(500));
        drill.advance(Timestamp(600));
        assert_eq!(shown(&drill), card("G4"));
    }

    #[test]
    #[should_panic(expected = "at least one card")]
    fn empty_pool_panics() {
        let _ = Drill::new(vec![], &[], 5, Timestamp(0), 1);
    }

    #[test]
    fn weak_cards_come_up_more_often() {
        let wrong = |c: &str| Attempt {
            at: Timestamp(0),
            card: card(c),
            guess: Guess::Name(Letter::B),
            reaction: Millis(3000),
        };
        let right = |c: &str| Attempt {
            at: Timestamp(0),
            card: card(c),
            guess: Guess::Key(card(c).pitch.midi()),
            reaction: Millis(400),
        };
        let history: Vec<_> = (0..20).flat_map(|_| [wrong("C4"), right("D4"), right("E4")]).collect();
        let picks =
            (0..300).filter(|&seed| shown(&Drill::new(pool(), &history, 1, Timestamp(0), seed)) == card("C4")).count();
        assert!(picks > 180, "C4 should dominate, got {picks}/300");
    }

    #[test]
    fn need_ranks_unseen_wrong_and_slow() {
        let t = |seen, correct, ms: &[u32]| Tally::from_parts(seen, correct, ms.to_vec());
        let unseen = need(None);
        let fast = need(Some(&t(5, 5, &[400; 5])));
        let slow = need(Some(&t(5, 5, &[3000; 5])));
        let wrong = need(Some(&t(5, 0, &[])));
        assert!(fast < slow && slow < wrong && fast < unseen);
    }

    #[test]
    fn weighted_pick_edges() {
        let items = [(1, 1.0), (2, 0.0), (3, 1.0)];
        assert_eq!(weighted_pick(&items, 0.0), 1);
        assert_eq!(weighted_pick(&items, 0.75), 3);
        assert_eq!(weighted_pick(&items, 1.0), 3, "r at the top falls back to the last item");
    }

    #[test]
    fn attempts_roundtrip_through_json() {
        let a = Attempt { at: Timestamp(5), card: card("A4"), guess: Guess::Key(Midi(69)), reaction: Millis(900) };
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Attempt>(&json).unwrap(), a);
    }
}
