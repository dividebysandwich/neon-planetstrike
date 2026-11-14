//! Deterministic 2D noise for the terrain and the scenery: gradient noise from an integer hash,
//! summed in octaves (fBm), ridged, and domain warped. Only `+ - * floor` on f32, so the server
//! and every client (native or wasm) compute the same heights from the same seed.

/// A 32-bit hash of a lattice point and a seed (lowbias32 mix).
#[inline]
pub fn hash2(x: i32, z: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (z as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h
}

/// A value in 0..1 from a hash.
#[inline]
pub fn unit(h: u32) -> f32 {
    (h >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// 0..1 at lattice point (x, z).
#[inline]
pub fn rand2(x: i32, z: i32, seed: u32) -> f32 {
    unit(hash2(x, z, seed))
}

/// Twelve gradient directions (unit-ish, no trigonometry).
const GRADS: [(f32, f32); 12] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (0.866, 0.5),
    (-0.866, 0.5),
    (0.866, -0.5),
    (-0.866, -0.5),
    (0.5, 0.866),
    (-0.5, 0.866),
    (0.5, -0.866),
    (-0.5, -0.866),
];

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn grad(ix: i32, iz: i32, seed: u32, dx: f32, dz: f32) -> f32 {
    let g = GRADS[(hash2(ix, iz, seed) % 12) as usize];
    g.0 * dx + g.1 * dz
}

/// Gradient noise, about -1..1, features about 1 unit apart.
pub fn perlin(x: f32, z: f32, seed: u32) -> f32 {
    let x0 = x.floor();
    let z0 = z.floor();
    let (ix, iz) = (x0 as i32, z0 as i32);
    let (fx, fz) = (x - x0, z - z0);
    let a = grad(ix, iz, seed, fx, fz);
    let b = grad(ix + 1, iz, seed, fx - 1.0, fz);
    let c = grad(ix, iz + 1, seed, fx, fz - 1.0);
    let d = grad(ix + 1, iz + 1, seed, fx - 1.0, fz - 1.0);
    let u = fade(fx);
    let v = fade(fz);
    let ab = a + (b - a) * u;
    let cd = c + (d - c) * u;
    (ab + (cd - ab) * v) * 1.4
}

/// Fractal sum of `octaves` of `perlin`, about -1..1.
pub fn fbm(x: f32, z: f32, seed: u32, octaves: u32, gain: f32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += perlin(x * freq, z * freq, seed.wrapping_add(o * 1013)) * amp;
        norm += amp;
        amp *= gain;
        freq *= 2.03;
    }
    sum / norm
}

/// Ridged multifractal, 0..1: sharp crests where the noise crosses zero.
pub fn ridged(x: f32, z: f32, seed: u32, octaves: u32, gain: f32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    let mut weight = 1.0f32;
    for o in 0..octaves {
        let n = 1.0 - perlin(x * freq, z * freq, seed.wrapping_add(o * 7919)).abs();
        let n = n * n * weight;
        weight = (n * 1.6).clamp(0.0, 1.0);
        sum += n * amp;
        norm += amp;
        amp *= gain;
        freq *= 2.11;
    }
    sum / norm
}

/// Cellular (Worley) noise: distance to the nearest of one jittered point per unit cell
/// (about 0..1), and that point's hash (to colour or size what sits there).
pub fn cells(x: f32, z: f32, seed: u32) -> (f32, u32) {
    let (cx, cz) = (x.floor() as i32, z.floor() as i32);
    let mut best = (f32::MAX, 0u32);
    for dz in -1..=1 {
        for dx in -1..=1 {
            let (gx, gz) = (cx + dx, cz + dz);
            let h = hash2(gx, gz, seed);
            let px = gx as f32 + unit(h);
            let pz = gz as f32 + unit(h.rotate_left(13));
            let d2 = (px - x) * (px - x) + (pz - z) * (pz - z);
            if d2 < best.0 {
                best = (d2, h);
            }
        }
    }
    (best.0.sqrt(), best.1)
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A small deterministic random sequence (splitmix64), for generation that has to come out the
/// same everywhere without depending on a crate's algorithm staying the same.
#[derive(Clone, Debug)]
pub struct Seq(pub u64);

impl Seq {
    pub fn new(seed: u64, stream: u64) -> Seq {
        let mut s = Seq(seed ^ stream.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        s.next_u64();
        s
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// 0..1
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * (1.0 / 16_777_216.0)
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f()
    }

    /// lo..hi (exclusive)
    pub fn int(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_bounded_and_continuous() {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..2000 {
            let x = i as f32 * 0.137;
            let z = i as f32 * 0.071;
            let n = fbm(x, z, 7, 5, 0.5);
            lo = lo.min(n);
            hi = hi.max(n);
            let m = fbm(x + 0.001, z, 7, 5, 0.5);
            assert!((n - m).abs() < 0.05);
        }
        assert!(lo > -1.2 && hi < 1.2 && hi - lo > 0.6, "{lo}..{hi}");
        let r = ridged(3.3, 4.4, 1, 4, 0.5);
        assert!((0.0..=1.0).contains(&r));
    }

    #[test]
    fn seq_is_deterministic() {
        let mut a = Seq::new(42, 1);
        let mut b = Seq::new(42, 1);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Seq::new(42, 2).next_u64(), Seq::new(42, 1).next_u64());
    }
}
