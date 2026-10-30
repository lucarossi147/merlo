//! Pitches, piano keys and note names.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A note letter. The discriminant is its diatonic index within the octave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Letter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Letter {
    pub const ALL: [Self; 7] = [Self::C, Self::D, Self::E, Self::F, Self::G, Self::A, Self::B];

    /// Letter at a diatonic step, wrapping around the octave.
    pub const fn from_step(step: i16) -> Self {
        Self::ALL[step.rem_euclid(7) as usize]
    }

    const fn semitone(self) -> i16 {
        match self {
            Self::C => 0,
            Self::D => 2,
            Self::E => 4,
            Self::F => 5,
            Self::G => 7,
            Self::A => 9,
            Self::B => 11,
        }
    }

    /// The name under a given naming convention.
    pub const fn name(self, naming: Naming) -> &'static str {
        const LETTERS: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
        const SOLFEGE: [&str; 7] = ["Do", "Re", "Mi", "Fa", "Sol", "La", "Si"];
        match naming {
            Naming::Letters => LETTERS[self as usize],
            Naming::Solfege => SOLFEGE[self as usize],
        }
    }
}

/// How note names are displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Naming {
    #[default]
    Letters,
    Solfege,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub enum Accidental {
    Flat,
    #[default]
    Natural,
    Sharp,
}

impl Accidental {
    const fn offset(self) -> i16 {
        self as i16 - 1
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Flat => "♭",
            Self::Natural => "",
            Self::Sharp => "♯",
        }
    }
}

/// A written pitch in scientific notation (middle C is `C4`).
///
/// Field order makes the derived `Ord` follow the staff from low to high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pitch {
    pub octave: i8,
    pub letter: Letter,
    pub accidental: Accidental,
}

impl Pitch {
    pub const fn natural(letter: Letter, octave: i8) -> Self {
        Self { octave, letter, accidental: Accidental::Natural }
    }

    /// Diatonic steps from C0: one per staff position.
    pub const fn step(self) -> i16 {
        self.octave as i16 * 7 + self.letter as i16
    }

    /// The natural pitch at a diatonic step.
    pub const fn from_step(step: i16) -> Self {
        Self::natural(Letter::from_step(step), step.div_euclid(7) as i8)
    }

    /// The piano key that sounds this pitch.
    pub const fn midi(self) -> Midi {
        Midi((12 * (self.octave as i16 + 1) + self.letter.semitone() + self.accidental.offset()) as u8)
    }

    pub fn name(self, naming: Naming) -> String {
        format!("{}{}", self.letter.name(naming), self.accidental.symbol())
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.name(Naming::Letters), self.octave)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePitchError(pub String);

impl FromStr for Pitch {
    type Err = ParsePitchError;

    /// Parses `C4`, `F#3`, `Bb2`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParsePitchError(s.to_owned());
        let mut chars = s.chars();
        let letter = chars
            .next()
            .and_then(|c| Letter::ALL.into_iter().find(|l| l.name(Naming::Letters).starts_with(c)))
            .ok_or_else(err)?;
        let rest = chars.as_str();
        let (accidental, octave) = match rest.chars().next() {
            Some('#') => (Accidental::Sharp, &rest[1..]),
            Some('b') => (Accidental::Flat, &rest[1..]),
            _ => (Accidental::Natural, rest),
        };
        let octave = octave.parse().map_err(|_| err())?;
        Ok(Self { octave, letter, accidental })
    }
}

/// A piano key as a MIDI note number (middle C is 60).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Midi(pub u8);

impl Midi {
    pub const fn is_black(self) -> bool {
        matches!(self.0 % 12, 1 | 3 | 6 | 8 | 10)
    }

    /// The key's pitch, spelled with sharps.
    pub const fn pitch(self) -> Pitch {
        const SPELL: [(Letter, Accidental); 12] = {
            use Accidental::{Natural as N, Sharp as S};
            use Letter::{A, B, C, D, E, F, G};
            [(C, N), (C, S), (D, N), (D, S), (E, N), (F, N), (F, S), (G, N), (G, S), (A, N), (A, S), (B, N)]
        };
        let (letter, accidental) = SPELL[(self.0 % 12) as usize];
        Pitch { octave: (self.0 / 12) as i8 - 1, letter, accidental }
    }

    /// Frequency in Hz with A4 = 440.
    pub fn frequency(self) -> f64 {
        440.0 * 2f64.powf((f64::from(self.0) - 69.0) / 12.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pitch {
        s.parse().unwrap()
    }

    #[test]
    fn midi_of_landmarks() {
        assert_eq!(p("C4").midi(), Midi(60));
        assert_eq!(p("A4").midi(), Midi(69));
        assert_eq!(p("A0").midi(), Midi(21));
        assert_eq!(p("C8").midi(), Midi(108));
        assert_eq!(p("F#3").midi(), Midi(54));
        assert_eq!(p("Bb2").midi(), Midi(46));
        assert_eq!(p("Cb4").midi(), p("B3").midi(), "enharmonics share a key");
    }

    #[test]
    fn midi_pitch_roundtrip_over_the_whole_piano() {
        for key in (21..=108).map(Midi) {
            assert_eq!(key.pitch().midi(), key);
            assert_eq!(key.is_black(), key.pitch().accidental == Accidental::Sharp);
        }
    }

    #[test]
    fn step_roundtrip_and_order() {
        for step in -10..80 {
            assert_eq!(Pitch::from_step(step).step(), step);
        }
        assert!(p("B3") < p("C4") && p("C4") < p("C#4") && p("C#4") < p("D4"));
    }

    #[test]
    fn names_and_display() {
        assert_eq!(p("G4").name(Naming::Solfege), "Sol");
        assert_eq!(p("F#5").name(Naming::Letters), "F♯");
        assert_eq!(p("Bb2").name(Naming::Solfege), "Si♭");
        assert_eq!(p("Eb3").to_string(), "E♭3");
        assert_eq!(p("C-1").midi(), Midi(0));
    }

    #[test]
    fn parse_errors() {
        for bad in ["", "H4", "C", "C#", "Cx4"] {
            assert_eq!(bad.parse::<Pitch>(), Err(ParsePitchError(bad.into())), "{bad}");
        }
    }

    #[test]
    fn frequency_of_a4_and_octaves() {
        assert!((Midi(69).frequency() - 440.0).abs() < 1e-9);
        assert!((Midi(57).frequency() - 220.0).abs() < 1e-9);
    }
}
