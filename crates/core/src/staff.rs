//! Clefs and positions on the five-line staff.

use crate::pitch::{Letter, Pitch};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Clef {
    Treble,
    Bass,
}

impl Clef {
    /// The pitch sitting on the bottom line.
    const fn bottom_line(self) -> Pitch {
        match self {
            Self::Treble => Pitch::natural(Letter::E, 4),
            Self::Bass => Pitch::natural(Letter::G, 2),
        }
    }

    pub const fn position(self, pitch: Pitch) -> StaffPos {
        StaffPos((pitch.step() - self.bottom_line().step()) as i8)
    }

    pub const fn pitch_at(self, pos: StaffPos) -> Pitch {
        Pitch::from_step(self.bottom_line().step() + pos.0 as i16)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Treble => "Treble",
            Self::Bass => "Bass",
        }
    }
}

/// Half-steps above the bottom line: 0 is the bottom line, 1 the first space,
/// 8 the top line. Negative and >8 positions need ledger lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StaffPos(pub i8);

impl StaffPos {
    pub const BOTTOM: Self = Self(0);
    pub const TOP: Self = Self(8);

    pub const fn is_line(self) -> bool {
        self.0 % 2 == 0
    }

    /// The ledger lines needed to reach this position.
    pub fn ledger_lines(self) -> impl Iterator<Item = Self> {
        let below = self.0..=-2;
        let above = (Self::TOP.0 + 2)..=self.0;
        below.chain(above).filter(|p| p % 2 == 0).map(Self)
    }
}

/// The unit of reading practice: a pitch written in a clef.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Card {
    pub clef: Clef,
    pub pitch: Pitch,
}

impl Card {
    pub const fn position(self) -> StaffPos {
        self.clef.position(self.pitch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pitch {
        s.parse().unwrap()
    }

    #[test]
    fn landmark_positions() {
        assert_eq!(Clef::Treble.position(p("E4")), StaffPos::BOTTOM);
        assert_eq!(Clef::Treble.position(p("F5")), StaffPos::TOP);
        assert_eq!(Clef::Treble.position(p("C4")), StaffPos(-2));
        assert_eq!(Clef::Bass.position(p("G2")), StaffPos::BOTTOM);
        assert_eq!(Clef::Bass.position(p("A3")), StaffPos::TOP);
        assert_eq!(Clef::Bass.position(p("C4")), StaffPos(10));
        assert_eq!(Clef::Bass.position(p("F3")), StaffPos(6), "the F clef's line");
    }

    #[test]
    fn accidentals_share_their_letter_position() {
        assert_eq!(Clef::Treble.position(p("F#4")), Clef::Treble.position(p("F4")));
    }

    #[test]
    fn position_roundtrip() {
        for clef in [Clef::Treble, Clef::Bass] {
            for pos in (-8..16).map(StaffPos) {
                assert_eq!(clef.position(clef.pitch_at(pos)), pos);
            }
        }
    }

    #[test]
    fn ledger_lines() {
        let lines = |pos| StaffPos(pos).ledger_lines().map(|p| p.0).collect::<Vec<_>>();
        assert!(lines(-1).is_empty());
        assert!(lines(4).is_empty());
        assert!(lines(9).is_empty());
        assert_eq!(lines(-2), [-2]);
        assert_eq!(lines(-5), [-4, -2]);
        assert_eq!(lines(10), [10]);
        assert_eq!(lines(13), [10, 12]);
    }

    #[test]
    fn lines_and_spaces() {
        assert!(StaffPos(-2).is_line() && StaffPos(8).is_line());
        assert!(!StaffPos(-1).is_line() && !StaffPos(7).is_line());
    }

    #[test]
    fn card_position_and_clef_name() {
        let card = Card { clef: Clef::Bass, pitch: p("B2") };
        assert_eq!(card.position(), StaffPos(2));
        assert_eq!(Clef::Bass.name(), "Bass");
        assert_eq!(Clef::Treble.name(), "Treble");
    }
}
