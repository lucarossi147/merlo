//! Pure layout math for the SVG views: staff, keyboard, charts, colours.

use merlo_core::{Midi, StaffPos};

/// Staff drawing: half the distance between two lines, and the y of the middle line.
pub const HALF_GAP: f64 = 8.0;
pub const STAFF_MID: f64 = 100.0;

/// Vertical coordinate of a staff position (higher notes, smaller y).
pub fn staff_y(pos: StaffPos) -> f64 {
    STAFF_MID - f64::from(pos.0 - StaffPos::TOP.0 / 2) * HALF_GAP
}

/// The five staff lines' y coordinates, bottom to top.
pub fn staff_lines() -> impl Iterator<Item = f64> {
    (0..=StaffPos::TOP.0).step_by(2).map(|p| staff_y(StaffPos(p)))
}

/// Horizontal position of a scrolling note: from `end` (just appeared) to `line` (too late).
pub fn scroll_x(progress: f64, line: f64, end: f64) -> f64 {
    line + (1.0 - progress.clamp(0.0, 1.0)) * (end - line)
}

/// One piano key in keyboard units, where a white key is 1 wide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyRect {
    pub key: Midi,
    pub x: f64,
    pub width: f64,
}

impl KeyRect {
    pub fn is_black(self) -> bool {
        self.key.is_black()
    }
}

pub const BLACK_WIDTH: f64 = 0.6;

/// The nearest white key strictly beyond `key`, one step in `dir` (±1).
fn white_beyond(key: Midi, dir: i16) -> Midi {
    let next = |k: i16| Midi((k + dir).clamp(0, 127) as u8);
    let step = next(i16::from(key.0));
    if step.is_black() {
        next(i16::from(step.0))
    } else {
        step
    }
}

/// Keys from `lo` to `hi` with one white key of margin on each side, so the
/// edges of the keyboard don't give the answer away.
/// White keys come first so black keys are drawn on top.
pub fn keyboard(lo: Midi, hi: Midi) -> Vec<KeyRect> {
    let first = white_beyond(lo, -1).0;
    let last = white_beyond(hi, 1).0;
    let (whites, blacks): (Vec<_>, Vec<_>) = (first..=last)
        .map(Midi)
        .scan(0.0, |white_x, key| {
            let rect = if key.is_black() {
                KeyRect { key, x: *white_x - BLACK_WIDTH / 2.0, width: BLACK_WIDTH }
            } else {
                *white_x += 1.0;
                KeyRect { key, x: *white_x - 1.0, width: 1.0 }
            };
            Some(rect)
        })
        .partition(|k| !k.is_black());
    whites.into_iter().chain(blacks).collect()
}

/// Maps a series into a `w`×`h` box with `lo..hi` on the y axis (clamped).
/// `None` gaps are skipped; x keeps the series' index spacing.
pub fn plot(values: &[Option<f64>], w: f64, h: f64, (lo, hi): (f64, f64)) -> Vec<(f64, f64)> {
    let step = w / (values.len().max(2) - 1) as f64;
    let span = (hi - lo).max(f64::EPSILON);
    values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|v| (i as f64 * step, h - ((v - lo) / span).clamp(0.0, 1.0) * h)))
        .collect()
}

/// An SVG `points` attribute.
pub fn points(pts: &[(f64, f64)]) -> String {
    pts.iter().map(|(x, y)| format!("{x:.1},{y:.1}")).collect::<Vec<_>>().join(" ")
}

/// A colour from fast (green) to slow (red) for a reaction time; grey if unseen.
pub fn heat(median_ms: Option<u32>) -> &'static str {
    match median_ms {
        None => "var(--muted)",
        Some(0..800) => "#2fb36d",
        Some(800..1300) => "#8cc63f",
        Some(1300..2000) => "#f5b82e",
        Some(2000..3000) => "#f08a24",
        Some(_) => "#e5484d",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staff_is_centred_and_spaced() {
        assert!((staff_y(StaffPos(4)) - STAFF_MID).abs() < f64::EPSILON);
        assert!(staff_y(StaffPos::TOP) < staff_y(StaffPos::BOTTOM));
        let lines: Vec<_> = staff_lines().collect();
        assert_eq!(lines, [132.0, 116.0, 100.0, 84.0, 68.0]);
    }

    #[test]
    fn keyboard_has_one_white_key_of_margin() {
        let keys = keyboard(Midi(62), Midi(65)); // D4..F4 → C4..G4
        assert_eq!(keys.len(), 8);
        assert_eq!(keys.iter().filter(|k| !k.is_black()).count(), 5);
        assert_eq!(keys[0], KeyRect { key: Midi(60), x: 0.0, width: 1.0 });
        let c_sharp = keys.iter().find(|k| k.key == Midi(61)).unwrap();
        assert!((c_sharp.x - (1.0 - BLACK_WIDTH / 2.0)).abs() < 1e-9);
        assert!(keys[..5].iter().all(|k| !k.is_black()), "whites first");
        let wide = keyboard(Midi(57), Midi(84)); // A3..C6 → G3..D6
        assert_eq!((wide[0].key, wide.iter().map(|k| k.key).max()), (Midi(55), Some(Midi(86))));
        assert_eq!(keyboard(Midi(0), Midi(127)).len(), 128, "clamped to the MIDI range");
    }

    #[test]
    fn plot_scales_and_skips_gaps() {
        let pts = plot(&[Some(0.0), None, Some(10.0)], 100.0, 50.0, (0.0, 10.0));
        assert_eq!(points(&pts), "0.0,50.0 100.0,0.0");
        assert_eq!(plot(&[Some(99.0)], 100.0, 50.0, (0.0, 10.0)), [(0.0, 0.0)], "clamped");
        assert_eq!(points(&plot(&[], 100.0, 50.0, (0.0, 1.0))), "");
    }

    #[test]
    fn scrolling_runs_from_the_end_to_the_line() {
        assert!((scroll_x(0.0, 80.0, 580.0) - 580.0).abs() < f64::EPSILON);
        assert!((scroll_x(0.5, 80.0, 580.0) - 330.0).abs() < f64::EPSILON);
        assert!((scroll_x(7.0, 80.0, 580.0) - 80.0).abs() < f64::EPSILON, "clamped at the line");
    }

    #[test]
    fn heat_buckets() {
        assert_eq!(heat(None), "var(--muted)");
        let colours: Vec<_> = [500, 1000, 1500, 2500, 5000].map(|ms| heat(Some(ms))).to_vec();
        let mut unique = colours.clone();
        unique.dedup();
        assert_eq!(unique.len(), 5);
    }
}
