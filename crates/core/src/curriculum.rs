//! The path of lessons, from middle C to the grand staff.

use crate::drill::Attempt;
use crate::staff::{Card, Clef, StaffPos};
use crate::stats::Tally;
use crate::time::Millis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Lines,
    Spaces,
}

/// A stretch of one staff, e.g. "treble lines from E4 to F5".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub clef: Clef,
    pub lo: StaffPos,
    pub hi: StaffPos,
    pub filter: Filter,
}

const fn span(clef: Clef, lo: i8, hi: i8, filter: Filter) -> Span {
    Span { clef, lo: StaffPos(lo), hi: StaffPos(hi), filter }
}

impl Span {
    fn cards(self) -> impl Iterator<Item = Card> {
        (self.lo.0..=self.hi.0)
            .map(StaffPos)
            .filter(move |p| match self.filter {
                Filter::All => true,
                Filter::Lines => p.is_line(),
                Filter::Spaces => !p.is_line(),
            })
            .map(move |p| Card { clef: self.clef, pitch: self.clef.pitch_at(p) })
    }
}

/// 0 to 3 stars, earned by accuracy and speed on a lesson's recent attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stars(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lesson {
    pub title: &'static str,
    pub hint: &'static str,
    pub spans: &'static [Span],
}

impl Lesson {
    /// How many recent attempts decide the stars (at least twice the pool).
    pub const RECENT: usize = 40;
    /// Attempts needed before any star is awarded.
    pub const MIN_ATTEMPTS: usize = 20;

    pub fn pool(&self) -> Vec<Card> {
        self.spans.iter().flat_map(|s| s.cards()).collect()
    }

    pub fn stars(&self, log: &[Attempt]) -> Stars {
        let pool = self.pool();
        let window = Self::RECENT.max(2 * pool.len());
        let recent: Vec<_> = log.iter().rev().filter(|a| pool.contains(&a.card)).take(window).collect();
        let covered = pool.iter().all(|c| recent.iter().any(|a| a.card == *c));
        let tally = Tally::of(recent.iter().copied());
        let (Some(accuracy), Some(median)) = (tally.accuracy(), tally.median()) else { return Stars(0) };
        Stars(match accuracy {
            _ if recent.len() < Self::MIN_ATTEMPTS || !covered => 0,
            a if a >= 0.95 && median <= Millis(1200) => 3,
            a if a >= 0.90 && median <= Millis(2000) => 2,
            a if a >= 0.80 => 1,
            _ => 0,
        })
    }
}

/// How many lessons are open: the first, plus one per consecutive starred lesson.
pub fn unlocked(log: &[Attempt]) -> usize {
    let passed = LESSONS.iter().take_while(|l| l.stars(log) >= Stars(1)).count();
    (passed + 1).min(LESSONS.len())
}

use Clef::{Bass, Treble};
use Filter::{All, Lines, Spaces};

pub const LESSONS: &[Lesson] = &[
    Lesson { title: "Middle C", hint: "C D E F G, climbing from middle C", spans: &[span(Treble, -2, 2, All)] },
    Lesson { title: "Treble lines", hint: "Every Good Bird Does Fly: E G B D F", spans: &[span(Treble, 0, 8, Lines)] },
    Lesson { title: "Treble spaces", hint: "The spaces spell F A C E", spans: &[span(Treble, 1, 7, Spaces)] },
    Lesson { title: "Treble staff", hint: "Lines and spaces together", spans: &[span(Treble, -1, 9, All)] },
    Lesson { title: "Treble ledgers", hint: "Above and below the staff", spans: &[span(Treble, -4, 12, All)] },
    Lesson { title: "Bass clef", hint: "F to middle C: the F clef's dots hug F", spans: &[span(Bass, 6, 10, All)] },
    Lesson { title: "Bass lines", hint: "Good Birds Don't Fly Away: G B D F A", spans: &[span(Bass, 0, 8, Lines)] },
    Lesson { title: "Bass spaces", hint: "All Cows Eat Grass: A C E G", spans: &[span(Bass, 1, 7, Spaces)] },
    Lesson { title: "Bass staff", hint: "Lines and spaces together", spans: &[span(Bass, -1, 9, All)] },
    Lesson { title: "Bass ledgers", hint: "Above and below the staff", spans: &[span(Bass, -4, 12, All)] },
    Lesson {
        title: "Grand staff",
        hint: "Both clefs, the whole range",
        spans: &[span(Treble, -4, 12, All), span(Bass, -4, 12, All)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drill::Guess;
    use crate::pitch::Midi;
    use crate::time::Timestamp;

    fn names(lesson: &Lesson) -> Vec<String> {
        lesson.pool().iter().map(|c| c.pitch.to_string()).collect()
    }

    fn practise(lesson: &Lesson, n: usize, right_every: usize, ms: u32) -> Vec<Attempt> {
        let pool = lesson.pool();
        (0..n)
            .map(|i| {
                let card = pool[i % pool.len()];
                let guess = if i % right_every == 0 { Guess::Key(Midi(0)) } else { Guess::Key(card.pitch.midi()) };
                Attempt { at: Timestamp(0), card, guess, reaction: Millis(ms) }
            })
            .collect()
    }

    #[test]
    fn pools_spell_the_mnemonics() {
        assert_eq!(names(&LESSONS[0]), ["C4", "D4", "E4", "F4", "G4"]);
        assert_eq!(names(&LESSONS[1]), ["E4", "G4", "B4", "D5", "F5"]);
        assert_eq!(names(&LESSONS[2]), ["F4", "A4", "C5", "E5"]);
        assert_eq!(names(&LESSONS[6]), ["G2", "B2", "D3", "F3", "A3"]);
        assert_eq!(names(&LESSONS[7]), ["A2", "C3", "E3", "G3"]);
        assert_eq!(names(&LESSONS[5]), ["F3", "G3", "A3", "B3", "C4"]);
    }

    #[test]
    fn every_lesson_has_cards_and_grand_staff_mixes_clefs() {
        assert!(LESSONS.iter().all(|l| !l.pool().is_empty()));
        let grand = LESSONS.last().unwrap().pool();
        assert!(grand.iter().any(|c| c.clef == Treble) && grand.iter().any(|c| c.clef == Bass));
    }

    #[test]
    fn stars_follow_accuracy_and_speed() {
        let lesson = &LESSONS[0];
        assert_eq!(lesson.stars(&[]), Stars(0));
        assert_eq!(lesson.stars(&practise(lesson, 10, usize::MAX, 500)), Stars(0), "too few attempts");
        assert_eq!(lesson.stars(&practise(lesson, 40, usize::MAX, 800)), Stars(3));
        assert_eq!(lesson.stars(&practise(lesson, 40, 12, 1500)), Stars(2));
        assert_eq!(lesson.stars(&practise(lesson, 40, 6, 3000)), Stars(1));
        assert_eq!(lesson.stars(&practise(lesson, 40, 2, 800)), Stars(0));
        assert_eq!(LESSONS[1].stars(&practise(lesson, 40, usize::MAX, 800)), Stars(0), "only E4 and G4 overlap");
        assert_eq!(LESSONS[3].stars(&practise(lesson, 40, usize::MAX, 800)), Stars(0), "D4..G5 not all practised");
    }

    #[test]
    fn unlocking_is_sequential() {
        assert_eq!(unlocked(&[]), 1);
        let log = practise(&LESSONS[0], 40, usize::MAX, 800);
        assert_eq!(unlocked(&log), 2);
        let everything: Vec<_> = LESSONS.iter().flat_map(|l| practise(l, 40, usize::MAX, 800)).collect();
        assert_eq!(unlocked(&everything), LESSONS.len());
    }
}
