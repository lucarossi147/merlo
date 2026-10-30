//! Time as plain numbers. The app reads the clock; the core only does arithmetic.

use serde::{Deserialize, Serialize};

const DAY_MS: i64 = 86_400_000;

/// Local wall-clock milliseconds since the Unix epoch (UTC time + the local
/// offset), so that day boundaries fall at the user's midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub const fn day(self) -> Day {
        Day(self.0.div_euclid(DAY_MS) as i32)
    }

    /// Elapsed time since an earlier instant; zero if the clock went backwards.
    pub fn since(self, earlier: Self) -> Millis {
        Millis(u32::try_from(self.0 - earlier.0).unwrap_or(if self < earlier { 0 } else { u32::MAX }))
    }
}

/// A duration, typically a reaction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Millis(pub u32);

impl Millis {
    pub fn seconds(self) -> f64 {
        f64::from(self.0) / 1000.0
    }
}

/// A calendar day, counted from the epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Day(pub i32);

impl Day {
    #[must_use]
    pub const fn prev(self) -> Self {
        Self(self.0 - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_split_at_midnight() {
        assert_eq!(Timestamp(0).day(), Day(0));
        assert_eq!(Timestamp(DAY_MS - 1).day(), Day(0));
        assert_eq!(Timestamp(DAY_MS).day(), Day(1));
        assert_eq!(Timestamp(-1).day(), Day(-1));
        assert_eq!(Day(5).prev(), Day(4));
    }

    #[test]
    fn since_saturates() {
        assert_eq!(Timestamp(1500).since(Timestamp(1000)), Millis(500));
        assert_eq!(Timestamp(1000).since(Timestamp(1500)), Millis(0));
        assert_eq!(Timestamp(i64::MAX).since(Timestamp(0)), Millis(u32::MAX));
        assert!((Millis(1250).seconds() - 1.25).abs() < f64::EPSILON);
    }
}
