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

/// Keys from `lo` to `hi`, widened to whole octaves (C to B).
/// White keys come first so black keys are drawn on top.
pub fn keyboard(lo: Midi, hi: Midi) -> Vec<KeyRect> {
    let first = lo.0 - lo.0 % 12;
    let last = hi.0 - hi.0 % 12 + 11;
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
    fn keyboard_covers_whole_octaves() {
        let keys = keyboard(Midi(62), Midi(65)); // D4..F4 → C4..B4
        assert_eq!(keys.len(), 12);
        assert_eq!(keys.iter().filter(|k| !k.is_black()).count(), 7);
        assert_eq!(keys[0], KeyRect { key: Midi(60), x: 0.0, width: 1.0 });
        let c_sharp = keys.iter().find(|k| k.key == Midi(61)).unwrap();
        assert!((c_sharp.x - (1.0 - BLACK_WIDTH / 2.0)).abs() < 1e-9);
        assert!(keys[..7].iter().all(|k| !k.is_black()), "whites first");
        assert_eq!(keyboard(Midi(55), Midi(72)).len(), 36, "G3..C5 → C3..B5");
    }

    #[test]
    fn plot_scales_and_skips_gaps() {
        let pts = plot(&[Some(0.0), None, Some(10.0)], 100.0, 50.0, (0.0, 10.0));
        assert_eq!(points(&pts), "0.0,50.0 100.0,0.0");
        assert_eq!(plot(&[Some(99.0)], 100.0, 50.0, (0.0, 10.0)), [(0.0, 0.0)], "clamped");
        assert_eq!(points(&plot(&[], 100.0, 50.0, (0.0, 1.0))), "");
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
