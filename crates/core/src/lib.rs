//! Pure domain logic for **Merlo**, a game for learning to read music.
//!
//! No IO, no clocks, no randomness from the outside world: everything here is
//! a function of its inputs. The app feeds in timestamps, seeds and guesses and
//! renders what comes out.
//!
//! The single source of truth is the append-only [`Attempt`] log. Every stat,
//! streak, level and badge is *derived* from it, so adding a new stat never
//! needs a data migration.

pub mod badge;
pub mod curriculum;
pub mod drill;
pub mod pitch;
pub mod rng;
pub mod staff;
pub mod stats;
pub mod time;

pub use badge::Badge;
pub use curriculum::{Lesson, Span, Stars, LESSONS};
pub use drill::{Attempt, Drill, Guess, Phase};
pub use pitch::{Accidental, Letter, Midi, Naming, Pitch};
pub use rng::Rng;
pub use staff::{Card, Clef, StaffPos};
pub use stats::{Profile, Tally};
pub use time::{Day, Millis, Timestamp};
