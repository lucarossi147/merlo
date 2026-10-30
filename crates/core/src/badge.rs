//! Badges: named predicates over the [`Profile`].

use crate::stats::Profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Badge {
    FirstNote,
    Century,
    Thousand,
    ThreeDays,
    Week,
    Month,
    Lightning,
    LowNotes,
    Marathon,
    Level5,
}

impl Badge {
    pub const ALL: [Self; 10] = [
        Self::FirstNote,
        Self::Century,
        Self::Thousand,
        Self::ThreeDays,
        Self::Week,
        Self::Month,
        Self::Lightning,
        Self::LowNotes,
        Self::Marathon,
        Self::Level5,
    ];

    /// `(icon, title, how to earn it)`.
    pub const fn info(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::FirstNote => ("🥚", "Hatchling", "Read your first note"),
            Self::Century => ("🪶", "Fledgling", "100 notes read right"),
            Self::Thousand => ("🐦‍⬛", "Songbird", "1000 notes read right"),
            Self::ThreeDays => ("🔥", "Warm nest", "A 3-day streak"),
            Self::Week => ("🌿", "Full week", "A 7-day streak"),
            Self::Month => ("🌳", "Old tree", "A 30-day streak"),
            Self::Lightning => ("⚡", "Lightning", "10 right in a row, each under a second"),
            Self::LowNotes => ("🎻", "Low notes", "100 bass-clef notes read right"),
            Self::Marathon => ("🏃", "Marathon", "200 notes in one day"),
            Self::Level5 => ("⭐", "Level 5", "Reach level 5"),
        }
    }

    pub fn earned(self, p: &Profile) -> bool {
        match self {
            Self::FirstNote => p.correct >= 1,
            Self::Century => p.correct >= 100,
            Self::Thousand => p.correct >= 1000,
            Self::ThreeDays => p.best_streak >= 3,
            Self::Week => p.best_streak >= 7,
            Self::Month => p.best_streak >= 30,
            Self::Lightning => p.lightning_run >= 10,
            Self::LowNotes => p.bass_correct >= 100,
            Self::Marathon => p.busiest_day >= 200,
            Self::Level5 => p.level.level >= 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::level;

    fn profile(correct: u32, streak: u32, lightning: u32, bass: u32, day: u32, xp: u32) -> Profile {
        Profile {
            xp,
            level: level(xp),
            attempts: correct,
            correct,
            streak,
            best_streak: streak,
            today_xp: 0,
            bass_correct: bass,
            lightning_run: lightning,
            busiest_day: day,
        }
    }

    fn earned(p: &Profile) -> Vec<Badge> {
        Badge::ALL.into_iter().filter(|b| b.earned(p)).collect()
    }

    #[test]
    fn nothing_for_a_newcomer() {
        assert!(earned(&profile(0, 0, 0, 0, 0, 0)).is_empty());
    }

    #[test]
    fn everything_for_a_veteran() {
        assert_eq!(earned(&profile(1000, 30, 10, 100, 200, 1000)), Badge::ALL);
    }

    #[test]
    fn thresholds() {
        assert_eq!(earned(&profile(1, 0, 0, 0, 1, 10)), [Badge::FirstNote]);
        assert_eq!(earned(&profile(100, 3, 9, 99, 199, 999)), [Badge::FirstNote, Badge::Century, Badge::ThreeDays]);
    }

    #[test]
    fn every_badge_is_described() {
        for b in Badge::ALL {
            let (icon, title, how) = b.info();
            assert!(!icon.is_empty() && !title.is_empty() && !how.is_empty());
        }
    }
}
