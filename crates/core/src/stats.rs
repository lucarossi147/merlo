//! Everything we know about the player, folded out of the attempt log.

use crate::drill::{Attempt, Guess};
use crate::staff::{Card, Clef};
use crate::time::{Day, Millis};
use std::collections::{BTreeMap, BTreeSet};

/// Counts and reaction times for any group of attempts.
/// Reaction times are kept for correct answers only: a fast wrong answer is a guess.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    pub seen: u32,
    pub correct: u32,
    times: Vec<u32>,
}

impl Tally {
    pub fn from_parts(seen: u32, correct: u32, times: Vec<u32>) -> Self {
        Self { seen, correct, times }
    }

    #[must_use]
    pub fn push(mut self, attempt: &Attempt) -> Self {
        self.seen += 1;
        if attempt.correct() {
            self.correct += 1;
            self.times.push(attempt.reaction.0);
        }
        self
    }

    pub fn of<'a>(attempts: impl IntoIterator<Item = &'a Attempt>) -> Self {
        attempts.into_iter().fold(Self::default(), Self::push)
    }

    pub fn accuracy(&self) -> Option<f64> {
        (self.seen > 0).then(|| f64::from(self.correct) / f64::from(self.seen))
    }

    /// The `q`-quantile (0..=1) of correct reaction times, nearest-rank.
    pub fn quantile(&self, q: f64) -> Option<Millis> {
        let mut times = self.times.clone();
        times.sort_unstable();
        let rank = (q.clamp(0.0, 1.0) * times.len() as f64).ceil() as usize;
        times.get(rank.saturating_sub(1)).copied().map(Millis)
    }

    /// Median reaction time: robust to the odd distracted answer.
    pub fn median(&self) -> Option<Millis> {
        self.quantile(0.5)
    }
}

/// Groups attempts by any key, e.g. per card, per day or per clef.
pub fn group_by<K: Ord>(log: &[Attempt], key: impl Fn(&Attempt) -> K) -> BTreeMap<K, Tally> {
    log.iter().fold(BTreeMap::new(), |mut groups, a| {
        let tally: &mut Tally = groups.entry(key(a)).or_default();
        *tally = std::mem::take(tally).push(a);
        groups
    })
}

/// Wrong answers ranked by frequency: what you read instead of what was written.
pub fn confusions(log: &[Attempt]) -> Vec<(Card, Guess, u32)> {
    let counts = log.iter().filter(|a| !a.correct()).fold(BTreeMap::new(), |mut m, a| {
        *m.entry((a.card, a.guess)).or_insert(0) += 1;
        m
    });
    let mut ranked: Vec<_> = counts.into_iter().map(|((c, g), n)| (c, g, n)).collect();
    ranked.sort_by_key(|&(.., n)| std::cmp::Reverse(n));
    ranked
}

/// Experience for one attempt: 10 for a right answer, plus a bonus for speed.
pub fn xp(attempt: &Attempt) -> u32 {
    match (attempt.correct(), attempt.reaction.0) {
        (false, _) => 0,
        (true, 0..1000) => 15,
        (true, 1000..2000) => 12,
        (true, _) => 10,
    }
}

/// Days practised in a row, ending today, or yesterday if today is still open.
pub fn current_streak(days: &BTreeSet<Day>, today: Day) -> u32 {
    let end = if days.contains(&today) { today } else { today.prev() };
    std::iter::successors(Some(end), |d| Some(d.prev())).take_while(|d| days.contains(d)).count() as u32
}

pub fn best_streak(days: &BTreeSet<Day>) -> u32 {
    days.iter()
        .scan(None::<(Day, u32)>, |run, &day| {
            let len = match *run {
                Some((last, len)) if last.0 + 1 == day.0 => len + 1,
                _ => 1,
            };
            *run = Some((day, len));
            Some(len)
        })
        .max()
        .unwrap_or(0)
}

/// Levels get progressively longer: level `n` takes `100 * n` XP to clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub level: u32,
    pub into: u32,
    pub span: u32,
}

pub fn level(xp: u32) -> Level {
    let mut level = Level { level: 1, into: xp, span: 100 };
    while level.into >= level.span {
        level = Level { level: level.level + 1, into: level.into - level.span, span: level.span + 100 };
    }
    level
}

/// XP to earn every day to keep the bird singing.
pub const DAILY_GOAL: u32 = 150;

/// A snapshot of the player, everything derived from the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub xp: u32,
    pub level: Level,
    pub attempts: u32,
    pub correct: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub today_xp: u32,
    pub bass_correct: u32,
    /// Longest run of consecutive right answers, each under a second.
    pub lightning_run: u32,
    pub busiest_day: u32,
}

impl Profile {
    pub fn from_log(log: &[Attempt], today: Day) -> Self {
        let days: BTreeSet<Day> = log.iter().map(|a| a.at.day()).collect();
        let total = Tally::of(log);
        let total_xp = log.iter().map(xp).sum();
        let lightning_run = log
            .iter()
            .scan(0, |run, a| {
                *run = if a.correct() && a.reaction < Millis(1000) { *run + 1 } else { 0 };
                Some(*run)
            })
            .max()
            .unwrap_or(0);
        Self {
            xp: total_xp,
            level: level(total_xp),
            attempts: total.seen,
            correct: total.correct,
            streak: current_streak(&days, today),
            best_streak: best_streak(&days),
            today_xp: log.iter().filter(|a| a.at.day() == today).map(xp).sum(),
            bass_correct: Tally::of(log.iter().filter(|a| a.card.clef == Clef::Bass)).correct,
            lightning_run,
            busiest_day: group_by(log, |a| a.at.day()).values().map(|t| t.seen).max().unwrap_or(0),
        }
    }

    /// Progress towards today's goal, in `0..=1`.
    pub fn goal_progress(&self) -> f64 {
        (f64::from(self.today_xp) / f64::from(DAILY_GOAL)).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Letter, Midi};
    use crate::time::Timestamp;

    const DAY: i64 = 86_400_000;

    fn attempt(day: i64, clef: Clef, note: &str, right: bool, ms: u32) -> Attempt {
        let pitch: crate::Pitch = note.parse().unwrap();
        let guess = if right { Guess::Key(pitch.midi()) } else { Guess::Key(Midi(0)) };
        Attempt { at: Timestamp(day * DAY + 1), card: Card { clef, pitch }, guess, reaction: Millis(ms) }
    }

    fn days(ds: &[i32]) -> BTreeSet<Day> {
        ds.iter().copied().map(Day).collect()
    }

    #[test]
    fn tally_accuracy_and_quantiles() {
        assert_eq!(Tally::default().accuracy(), None);
        assert_eq!(Tally::default().median(), None);
        let log: Vec<_> = [(true, 500), (true, 900), (true, 3000), (false, 100)]
            .map(|(r, ms)| attempt(0, Clef::Treble, "C4", r, ms))
            .to_vec();
        let t = Tally::of(&log);
        assert_eq!((t.seen, t.correct), (4, 3));
        assert_eq!(t.accuracy(), Some(0.75));
        assert_eq!(t.median(), Some(Millis(900)), "wrong answers don't count");
        assert_eq!(t.quantile(0.0), Some(Millis(500)));
        assert_eq!(t.quantile(1.0), Some(Millis(3000)));
    }

    #[test]
    fn grouping_and_confusions() {
        let log = vec![
            attempt(0, Clef::Treble, "C4", false, 900),
            attempt(0, Clef::Treble, "C4", false, 900),
            attempt(1, Clef::Treble, "E4", false, 900),
            attempt(1, Clef::Bass, "C4", true, 900),
        ];
        let by_clef = group_by(&log, |a| a.card.clef);
        assert_eq!(by_clef[&Clef::Treble].seen, 3);
        assert_eq!(by_clef[&Clef::Bass].correct, 1);
        assert_eq!(group_by(&log, |a| a.at.day()).len(), 2);

        let top = confusions(&log);
        assert_eq!(top.len(), 2);
        assert_eq!((top[0].0.pitch.to_string(), top[0].2), ("C4".into(), 2));
    }

    #[test]
    fn xp_rewards_speed() {
        assert_eq!(xp(&attempt(0, Clef::Treble, "C4", false, 100)), 0);
        assert_eq!(xp(&attempt(0, Clef::Treble, "C4", true, 999)), 15);
        assert_eq!(xp(&attempt(0, Clef::Treble, "C4", true, 1999)), 12);
        assert_eq!(xp(&attempt(0, Clef::Treble, "C4", true, 2000)), 10);
    }

    #[test]
    fn streaks() {
        assert_eq!(current_streak(&days(&[]), Day(10)), 0);
        assert_eq!(current_streak(&days(&[8, 9, 10]), Day(10)), 3);
        assert_eq!(current_streak(&days(&[8, 9]), Day(10)), 2, "today still open");
        assert_eq!(current_streak(&days(&[7, 8]), Day(10)), 0, "broken");
        assert_eq!(best_streak(&days(&[])), 0);
        assert_eq!(best_streak(&days(&[1, 2, 3, 7, 8, 20])), 3);
    }

    #[test]
    fn levels() {
        assert_eq!(level(0), Level { level: 1, into: 0, span: 100 });
        assert_eq!(level(99).level, 1);
        assert_eq!(level(100), Level { level: 2, into: 0, span: 200 });
        assert_eq!(level(350), Level { level: 3, into: 50, span: 300 });
    }

    #[test]
    fn profile_from_log() {
        let mut log = vec![attempt(9, Clef::Treble, "C4", true, 2500)];
        log.extend((0..4).map(|_| attempt(10, Clef::Bass, "G2", true, 800)));
        log.push(attempt(10, Clef::Treble, "C4", false, 800));
        log.push(attempt(10, Clef::Treble, "C4", true, 800));
        let p = Profile::from_log(&log, Day(10));
        assert_eq!((p.attempts, p.correct, p.bass_correct), (7, 6, 4));
        assert_eq!((p.streak, p.best_streak), (2, 2));
        assert_eq!(p.xp, 10 + 5 * 15);
        assert_eq!(p.today_xp, 5 * 15);
        assert_eq!((p.lightning_run, p.busiest_day), (4, 6));
        assert!((p.goal_progress() - 0.5).abs() < 1e-9);

        let empty = Profile::from_log(&[], Day(0));
        assert_eq!((empty.xp, empty.lightning_run, empty.busiest_day), (0, 0, 0));
        assert_eq!(Guess::Name(Letter::C), Guess::Name(Letter::C));
    }
}
