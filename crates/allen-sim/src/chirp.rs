//! The startup sound, ported from native/kernel/ui/chirp.[ch]: a quick "blirp", the last moment of
//! the chirp two bodies make as they spiral into each other and merge, raised into hearing and
//! timed to the night sky. Every frame is worked out from the formula (pitch as L^(-3/8), loudness
//! as pitch^(2/3), L the share of the inspiral left); no recorded sound. Integer only.
use crate::night::MERGE_MS;

pub const RATE: u32 = 48000;
pub const FROM_HZ: u32 = 200;
pub const TO_HZ: u32 = 1000;
pub const PEAK: u32 = 22937;
pub const RISE_MS: u32 = 600;
pub const IN_MS: u32 = 40;
pub const START_MS: u32 = MERGE_MS - RISE_MS;
pub const END_MS: u32 = MERGE_MS + 300;
pub const FRAMES: u32 = RATE / 1000 * END_MS;

const F0: u32 = RATE / 1000 * (START_MS - IN_MS);
const F1: u32 = RATE / 1000 * START_MS;
const F2: u32 = RATE / 1000 * MERGE_MS;
const Q30: u64 = 1 << 30;
/// exp(-1 / (30 ms * 48 kHz)) in Q30
const DECAY: u64 = 1072996429;

fn isqrt(mut x: u64) -> u64 {
    let mut r = 0u64;
    let mut bit = 1u64 << 62;
    while bit > x {
        bit >>= 2;
    }
    while bit != 0 {
        if x >= r + bit {
            x -= r + bit;
            r = (r >> 1) + bit;
        } else {
            r >>= 1;
        }
        bit >>= 2;
    }
    r
}

fn left_q30(f: u32) -> u64 {
    (((F2 - f) as u64) << 30) / (F2 - F1) as u64
}

fn root8(mut l: u64) -> u64 {
    for _ in 0..3 {
        l = isqrt(l << 30);
    }
    l
}

fn mul(a: u64, b: u64) -> u64 {
    a * b >> 30
}

fn inspiral_phase(f: u32) -> u64 {
    let r = root8(left_q30(f));
    let l58 = mul(mul(mul(mul(r, r), r), r), r);
    let p1 = ((FROM_HZ as u64 * (F1 - F0) as u64) << 32) / RATE as u64;
    let k = FROM_HZ as u64 * 8 * (F2 - F1) as u64;
    p1 + ((k * (Q30 - l58) << 2) / (5 * RATE as u64))
}

fn decay_pow(mut k: u32) -> u64 {
    let mut r = Q30;
    let mut b = DECAY;
    while k != 0 {
        if k & 1 != 0 {
            r = mul(r, b);
        }
        b = mul(b, b);
        k >>= 1;
    }
    r
}

pub struct Chirp {
    sin_t: Vec<i16>,
    cap_frame: u32,
    cap_l14: u64,
    cap_phase: u64,
}

impl Default for Chirp {
    fn default() -> Self {
        Self::new()
    }
}

impl Chirp {
    pub fn new() -> Chirp {
        const C: i64 = 1073740561;
        const S: i64 = 1647099;
        let mut sin_t = vec![0i16; 4096];
        let (mut c, mut s) = (1i64 << 30, 0i64);
        for k in 0..=1024usize {
            let v = if k == 1024 { 16384 } else { ((s + (1 << 15)) >> 16) as i16 };
            sin_t[k] = v;
            sin_t[2048 - k] = v;
            sin_t[(2048 + k) & 4095] = -v;
            sin_t[(4096 - k) & 4095] = -v;
            let nc = (c * C - s * S) >> 30;
            let ns = (s * C + c * S) >> 30;
            c = nc;
            s = ns;
        }
        let want = ((FROM_HZ as u64) << 30) / TO_HZ as u64;
        let (mut lo, mut hi) = (F1, F2);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let r = root8(left_q30(mid));
            if mul(mul(r, r), r) <= want {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let r = root8(left_q30(lo));
        Chirp { sin_t, cap_frame: lo, cap_l14: mul(r, r), cap_phase: inspiral_phase(lo) }
    }

    fn phase_at(&self, f: u32) -> u64 {
        if f < F1 {
            return ((FROM_HZ as u64 * (f - F0) as u64) << 32) / RATE as u64;
        }
        if f < self.cap_frame {
            return inspiral_phase(f);
        }
        self.cap_phase + (((TO_HZ as u64) * (f - self.cap_frame) as u64) << 32) / RATE as u64
    }

    /// The loudness (0..PEAK) at a frame.
    pub fn level(&self, f: u32) -> u32 {
        if f < F0 || f >= FRAMES {
            return 0;
        }
        let steady = PEAK as u64 * self.cap_l14 >> 30;
        if f < F1 {
            let x = (((f - F0) as u64) << 16) / (F1 - F0) as u64;
            let s = x * x * (3 * 65536 - 2 * x);
            return (steady * s >> 48) as u32;
        }
        if f < self.cap_frame {
            let r = root8(left_q30(f));
            return (PEAK as u64 * self.cap_l14 / mul(r, r)) as u32;
        }
        if f < F2 {
            return PEAK;
        }
        (PEAK as u64 * decay_pow(f - F2) >> 30) as u32
    }

    /// The pitch in Hz at a frame.
    pub fn hz(&self, f: u32) -> u32 {
        if f < F0 || f >= FRAMES {
            return 0;
        }
        if f < F1 {
            return FROM_HZ;
        }
        if f >= self.cap_frame {
            return TO_HZ;
        }
        let r = root8(left_q30(f));
        (((FROM_HZ as u64) << 30) / mul(mul(r, r), r)) as u32
    }

    /// One sample (both channels carry it) of frame f.
    pub fn sample(&self, f: u32) -> i16 {
        let a = self.level(f);
        if a == 0 {
            return 0;
        }
        let ph = self.phase_at(f) as u32;
        let k = (ph >> 20) as usize;
        let frac = (ph >> 4 & 0xffff) as i64;
        let s0 = self.sin_t[k] as i32;
        let s1 = self.sin_t[(k + 1) & 4095] as i32;
        let s = s0 + (((s1 - s0) as i64 * frac) >> 16) as i32;
        ((s as i64 * a as i64) >> 14) as i16
    }

    /// Frames [from, from + out.len()) as mono samples.
    pub fn fill(&self, out: &mut [i16], from: u32) {
        for (i, o) in out.iter_mut().enumerate() {
            *o = self.sample(from + i as u32);
        }
    }
}
