//! Ziplines (in the canyon caves, `crate::caves`): a cable between two anchors, each with a
//! spot to land on. On foot within reach of one, the use key hangs on it; it runs along it
//! the way one looks (driven, faster downhill), puts one down on the far end's landing, and
//! the jump key (or use again) lets go wherever one is. Riders can shoot and be shot.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// How far under the cable a rider's feet hang, how near one grabs it (from the hands held up:
/// `ZIP_HANDS` over the eyes), and how fast it runs (level; slowest uphill, fastest downhill).
pub const ZIP_HANG: f32 = 2.1;
pub const ZIP_REACH: f32 = 2.6;
pub const ZIP_HANDS: f32 = 0.8;
pub const ZIP_SPEED: f32 = 14.0;
pub const ZIP_MIN: f32 = 7.0;
pub const ZIP_MAX: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zipline {
    /// The cable's ends.
    pub a: Vec3,
    pub b: Vec3,
    /// Where a rider is put down at each end (on the floor).
    pub land_a: Vec3,
    pub land_b: Vec3,
}

impl Zipline {
    pub fn len(&self) -> f32 {
        (self.b - self.a).length()
    }

    /// Unit direction from `a` to `b`.
    pub fn dir(&self) -> Vec3 {
        (self.b - self.a).normalize_or_zero()
    }

    /// The point `s` metres along from `a` (clamped to the cable).
    pub fn at(&self, s: f32) -> Vec3 {
        self.a + self.dir() * s.clamp(0.0, self.len())
    }

    /// The point of the cable nearest `p`: how far along, and how far off.
    pub fn closest(&self, p: Vec3) -> (f32, f32) {
        let s = (p - self.a).dot(self.dir()).clamp(0.0, self.len());
        (s, (self.at(s) - p).length())
    }

    /// How fast a rider goes heading `sign` (+1 towards `b`).
    pub fn speed(&self, sign: f32) -> f32 {
        let climb = self.dir().y * sign;
        (ZIP_SPEED - climb * 20.0).clamp(ZIP_MIN, ZIP_MAX)
    }
}

/// A player hanging on a zipline: which, how far along, which way.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZipRide {
    pub line: u16,
    pub s: f32,
    pub sign: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cables_measure_and_run_faster_downhill() {
        let z = Zipline {
            a: Vec3::new(0.0, 30.0, 0.0),
            b: Vec3::new(100.0, 10.0, 0.0),
            land_a: Vec3::ZERO,
            land_b: Vec3::ZERO,
        };
        let (s, d) = z.closest(Vec3::new(50.0, 21.0, 0.0));
        assert!((s - z.len() / 2.0).abs() < 1.0 && d < 1.5, "{s} {d}");
        assert_eq!(z.closest(Vec3::new(-50.0, 30.0, 0.0)).0, 0.0);
        assert!(z.speed(1.0) > ZIP_SPEED && z.speed(-1.0) < ZIP_SPEED);
    }
}
