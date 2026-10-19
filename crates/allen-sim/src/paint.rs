//! The drawing kit, ported from native/kernel/ui/paint.[ch]: rectangles, colour arithmetic, soft
//! round lights, anti-aliased threads, rounded shapes and text into a RAM canvas of 0x00RRGGBB
//! pixels, plus the integer mathematics they share. Integer only. Angles are 65536ths of a turn.
//! Positions handed to the soft shapes are in 16ths of a pixel.
use crate::font::{isqrt, Face};
use std::sync::OnceLock;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub const fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

pub struct Canvas {
    pub px: Vec<u32>,
    pub w: i32,
    pub h: i32,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas { px: vec![0; (w as usize) * (h as usize)], w, h }
    }
}

pub const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r << 16 | g << 8 | b
}

pub fn min(a: i32, b: i32) -> i32 {
    if a < b { a } else { b }
}
pub fn max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}
pub fn abs(a: i32) -> i32 {
    if a < 0 { -a } else { a }
}
pub fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

/// r becomes its overlap with c; false when they do not meet.
pub fn clip(r: &mut Rect, c: &Rect) -> bool {
    let x0 = max(r.x, c.x);
    let y0 = max(r.y, c.y);
    let x1 = min(r.x + r.w, c.x + c.w);
    let y1 = min(r.y + r.h, c.y + c.h);
    if x1 <= x0 || y1 <= y0 {
        return false;
    }
    *r = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    true
}

pub fn meets(a: Rect, b: &Rect) -> bool {
    let mut a = a;
    clip(&mut a, b)
}

pub fn grow(r: Rect, m: i32) -> Rect {
    Rect { x: r.x - m, y: r.y - m, w: r.w + 2 * m, h: r.h + 2 * m }
}

/// The smallest rectangle holding both (an empty one adds nothing).
pub fn union(a: Rect, b: Rect) -> Rect {
    if a.w <= 0 || a.h <= 0 {
        return b;
    }
    if b.w <= 0 || b.h <= 0 {
        return a;
    }
    let x0 = min(a.x, b.x);
    let y0 = min(a.y, b.y);
    Rect { x: x0, y: y0, w: max(a.x + a.w, b.x + b.w) - x0, h: max(a.y + a.h, b.y + b.h) - y0 }
}

// ---- colour: channels 0..255, weights 0..256 ----

pub fn ch(c: u32, s: u32) -> u32 {
    c >> s & 255
}

/// c scaled by f/256 (f <= 256), all three channels at once
pub fn scale(c: u32, f: u32) -> u32 {
    ((c & 0xff00ff).wrapping_mul(f) >> 8 & 0xff00ff) | ((c & 0x00ff00).wrapping_mul(f) >> 8 & 0x00ff00)
}

/// d + c * a / 256 per channel, held at 255
pub fn add(d: u32, c: u32, a: u32) -> u32 {
    let r = ch(d, 16) + (ch(c, 16).wrapping_mul(a) >> 8);
    let g = ch(d, 8) + (ch(c, 8).wrapping_mul(a) >> 8);
    let b = ch(d, 0) + (ch(c, 0).wrapping_mul(a) >> 8);
    (if r > 255 { 255 } else { r }) << 16 | (if g > 255 { 255 } else { g }) << 8 | (if b > 255 { 255 } else { b })
}

/// d toward c by a/256
pub fn mix(d: u32, c: u32, a: u32) -> u32 {
    if a >= 256 {
        return c;
    }
    scale(d, 256 - a).wrapping_add(scale(c, a))
}

// ---- easing: t and the result in 0..256 ----

pub fn smooth(t: i32) -> u32 {
    let t = clamp(t, 0, 256);
    (t * t * (768 - 2 * t) >> 16) as u32
}

pub fn ease_out(t: i32) -> u32 {
    let t = 256 - clamp(t, 0, 256);
    256 - (t * t * t >> 16) as u32
}

/// 0..256 for how far `now` is from `from` to `to` (ms)
pub fn prog(now: u32, from: u32, to: u32) -> i32 {
    if now <= from {
        return 0;
    }
    if now >= to {
        return 256;
    }
    ((now - from) as u64 * 256 / (to - from) as u64) as i32
}

// ---- mathematics ----

struct Tables {
    sin_t: [i16; 4097],
    exp_t: [u32; 4096],
    log_t: [u32; 257],
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut sin_t = [0i16; 4097];
        let (cc, ss): (i64, i64) = (1073740561, 1647099);
        let (mut c, mut s): (i64, i64) = (1 << 30, 0);
        for k in 0..=1024usize {
            let v: i16 = if k == 1024 { 16384 } else { ((s + (1 << 15)) >> 16) as i16 };
            sin_t[k] = v;
            sin_t[2048 - k] = v;
            sin_t[(2048 + k) & 4095] = -v;
            sin_t[(4096 - k) & 4095] = -v;
            let nc = (c * cc - s * ss) >> 30;
            let ns = (s * cc + c * ss) >> 30;
            c = nc;
            s = ns;
        }
        sin_t[4096] = sin_t[0];
        let mut exp_t = [0u32; 4096];
        let mut e: u64 = 1 << 30;
        for k in 0..4096 {
            exp_t[k] = ((e + (1 << 13)) >> 14) as u32;
            e = e * 1069555701 >> 30;
        }
        let mut log_t = [0u32; 257];
        for k in 0..=256usize {
            let mut y: u64 = ((256 + k) << 8) as u64;
            let mut r: u32 = 0;
            if k == 256 {
                log_t[k] = 65536;
                continue;
            }
            for b in (0..=15).rev() {
                y = y * y >> 16;
                if y >= 2 << 16 {
                    y >>= 1;
                    r |= 1 << b;
                }
            }
            log_t[k] = r;
        }
        Tables { sin_t, exp_t, log_t }
    })
}

/// sine of a turn in 65536ths, scaled by 16384
pub fn sin(a: u32) -> i32 {
    let t = tables();
    let a = a & 65535;
    let i = (a >> 4) as usize;
    let f = (a & 15) as i32;
    t.sin_t[i] as i32 + (((t.sin_t[i + 1] as i32 - t.sin_t[i] as i32) * f) >> 4)
}

pub fn cos(a: u32) -> i32 {
    sin(a.wrapping_add(16384))
}

/// the angle of (x, y), 0..65535
pub fn atan2(y: i32, x: i32) -> u32 {
    static STEP: [u16; 14] = [8192, 4836, 2555, 1297, 651, 326, 163, 81, 41, 20, 10, 5, 3, 1];
    let (mut xx, mut yy) = (x as i64, y as i64);
    let mut ang: u32 = 0;
    if xx == 0 && yy == 0 {
        return 0;
    }
    if xx < 0 {
        xx = -xx;
        yy = -yy;
        ang = 32768;
    }
    let ay = if yy < 0 { -yy } else { yy };
    let mut m = if xx > ay { xx } else { ay };
    while m < (1 << 20) {
        xx <<= 1;
        yy <<= 1;
        m <<= 1;
    }
    while m >= (1 << 22) {
        xx >>= 1;
        yy >>= 1;
        m >>= 1;
    }
    let mut z: i32 = 0;
    for i in 0..14 {
        let nx;
        if yy > 0 {
            nx = xx + (yy >> i);
            yy -= xx >> i;
            z += STEP[i] as i32;
        } else {
            nx = xx - (yy >> i);
            yy += xx >> i;
            z -= STEP[i] as i32;
        }
        xx = nx;
    }
    ang.wrapping_add(z as u32) & 65535
}

/// exp(-x/256), scaled by 65536
pub fn exp_neg(x: u32) -> u32 {
    if x < 4096 { tables().exp_t[x as usize] } else { 0 }
}

/// log2(v) * 65536, v >= 1
pub fn log2_q16(v: u64) -> u32 {
    let t = tables();
    if v == 0 {
        return 0;
    }
    let n = 63 - v.leading_zeros() as i32;
    let m: u64 = if n >= 16 { v >> (n - 16) } else { v << (16 - n) };
    let k = (m >> 8 & 255) as usize;
    let f = (m & 255) as u32;
    (n as u32) * 65536 + t.log_t[k] + ((t.log_t[k + 1] - t.log_t[k]) * f >> 8)
}

pub fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841) ^ seed.wrapping_mul(0xcb1ab31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h = h.wrapping_mul(0x27d4eb2d);
    h ^= h >> 16;
    h
}

/// smooth value noise, 0..65535
pub fn noise(x: i32, y: i32, seed: u32) -> u32 {
    let ix = x >> 8;
    let iy = y >> 8;
    let fx = smooth(x & 255) as u64;
    let fy = smooth(y & 255) as u64;
    let v00 = (hash(ix, iy, seed) >> 16) as u64;
    let v10 = (hash(ix + 1, iy, seed) >> 16) as u64;
    let v01 = (hash(ix, iy + 1, seed) >> 16) as u64;
    let v11 = (hash(ix + 1, iy + 1, seed) >> 16) as u64;
    let a = v00 * (256 - fx) + v10 * fx;
    let b = v01 * (256 - fx) + v11 * fx;
    ((a * (256 - fy) + b * fy) >> 16) as u32
}

/// 0..65535
pub fn fbm(x: i32, y: i32, seed: u32, octaves: i32) -> u32 {
    let (mut sum, mut total): (u64, u64) = (0, 0);
    for o in 0..octaves {
        let w = 1u32 << (octaves - 1 - o);
        sum += noise(x << o, y << o, seed.wrapping_add((o as u32).wrapping_mul(7919))) as u64 * w as u64;
        total += w as u64;
    }
    (sum / total) as u32
}

// ---- shapes; each draws only inside the clip rectangle ----

/// d becomes the part of d inside both the clip and the canvas
fn fit(cv: &Canvas, clipr: &Rect, d: &mut Rect) -> bool {
    let all = Rect { x: 0, y: 0, w: cv.w, h: cv.h };
    clip(d, clipr) && clip(d, &all)
}

/// A soft round light at (cx, cy) with radius r (16ths of a pixel): colour added with weight
/// a * (1 - d^2/r^2)^3.
pub fn glow(cv: &mut Canvas, clipr: &Rect, cx: i32, cy: i32, r: i32, color: u32, a: u32) {
    if r <= 0 || a == 0 {
        return;
    }
    let mut d = Rect { x: (cx - r) >> 4, y: (cy - r) >> 4, w: 0, h: 0 };
    d.w = ((cx + r) >> 4) - d.x + 1;
    d.h = ((cy + r) >> 4) - d.y + 1;
    if !fit(cv, clipr, &mut d) {
        return;
    }
    let r2 = r as i64 * r as i64;
    let inv = (1u64 << 40) / r2 as u64;
    for y in d.y..d.y + d.h {
        let dy = y as i64 * 16 + 8 - cy as i64;
        let dy2 = dy * dy;
        if dy2 >= r2 {
            continue;
        }
        let half = isqrt((r2 - dy2) as u64) as i32;
        let x0 = max(d.x, (cx - half) >> 4);
        let x1 = min(d.x + d.w - 1, (cx + half) >> 4);
        let row = y as usize * cv.w as usize;
        for x in x0..=x1 {
            let dx = x as i64 * 16 + 8 - cx as i64;
            let d2 = dx * dx + dy2;
            if d2 >= r2 {
                continue;
            }
            let f = 256u32 - ((d2 as u64 * inv) >> 32) as u32;
            let w = (f * f >> 8) * f >> 8;
            let p = &mut cv.px[row + x as usize];
            *p = add(*p, color, w * a >> 8);
        }
    }
}

fn plot(cv: &mut Canvas, clipr: &Rect, x: i32, y: i32, color: u32, a: u32) {
    if a == 0 || x < clipr.x || y < clipr.y || x >= clipr.x + clipr.w || y >= clipr.y + clipr.h || x < 0 || y < 0 || x >= cv.w || y >= cv.h {
        return;
    }
    let i = y as usize * cv.w as usize + x as usize;
    cv.px[i] = add(cv.px[i], color, a);
}

/// A thread one pixel wide from (x0, y0) to (x1, y1) (16ths of a pixel), anti-aliased, added as
/// light whose weight runs from a0 to a1 along it.
pub fn thread(cv: &mut Canvas, clipr: &Rect, mut x0: i32, mut y0: i32, mut x1: i32, mut y1: i32, color: u32, mut a0: u32, mut a1: u32) {
    let mut bx = Rect { x: (min(x0, x1) >> 4) - 1, y: (min(y0, y1) >> 4) - 1, w: 0, h: 0 };
    bx.w = (max(x0, x1) >> 4) - bx.x + 3;
    bx.h = (max(y0, y1) >> 4) - bx.y + 3;
    if !meets(bx, clipr) || (a0 == 0 && a1 == 0) {
        return;
    }
    let steep = abs(y1 - y0) > abs(x1 - x0);
    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
        std::mem::swap(&mut a0, &mut a1);
    }
    let mut len = x1 - x0;
    if len < 1 {
        len = 1;
    }
    let grad = (y1 - y0) as i64 * 65536 / len as i64;
    let slant = (isqrt((65536i64 * 65536 + grad * grad) as u64) >> 8) as u32;
    let first = (x0 + 7) >> 4;
    let last = (x1 - 8) >> 4;
    let mut xx = first;
    while xx <= last {
        let along = xx * 16 + 8 - x0;
        let yc = y0 + ((along as i64 * grad) >> 16) as i32 - 8;
        let yy = yc >> 4;
        let f = yc & 15;
        let a = ((a0 as i32 + (a1 as i32 - a0 as i32) * along / len) as u32).wrapping_mul(slant) >> 8;
        if steep {
            plot(cv, clipr, yy, xx, color, a * (16 - f) as u32 >> 4);
            plot(cv, clipr, yy + 1, xx, color, a * f as u32 >> 4);
        } else {
            plot(cv, clipr, xx, yy, color, a * (16 - f) as u32 >> 4);
            plot(cv, clipr, xx, yy + 1, color, a * f as u32 >> 4);
        }
        xx += 1;
    }
}

/// Signed distance from a pixel's centre to a rounded rectangle, 16ths of a pixel, negative inside.
pub fn sd_rrect(x: i32, y: i32, r: &Rect, rad: i32) -> i32 {
    let px = x as i64 * 16 + 8;
    let py = y as i64 * 16 + 8;
    let cx = r.x as i64 * 16 + r.w as i64 * 8;
    let cy = r.y as i64 * 16 + r.h as i64 * 8;
    let qx = (if px > cx { px - cx } else { cx - px }) - (r.w as i64 * 8 - rad as i64 * 16);
    let qy = (if py > cy { py - cy } else { cy - py }) - (r.h as i64 * 8 - rad as i64 * 16);
    let out = if qx > 0 && qy > 0 { isqrt((qx * qx + qy * qy) as u64) as i64 } else if qx > qy { qx } else { qy };
    (out - rad as i64 * 16) as i32
}

/// the coverage of a pixel at such a distance
pub fn cover(d16: i32) -> u32 {
    let v = 8 - d16;
    if v <= 0 { 0 } else if v >= 16 { 256 } else { (v * 16) as u32 }
}

/// One letter at pen (1/256 px) on baseline y, mixed in with weight a (additive light when add is
/// set); returns the pen after it.
pub fn letter(cv: &mut Canvas, clipr: &Rect, f: &mut Face, pen: u64, y: i32, c: u8, color: u32, a: u32, additive: bool) -> u64 {
    if a != 0 {
        let g = f.glyph(c);
        if !g.alpha.is_empty() {
            let gx = ((pen + 128) / 256) as i32 + g.ox as i32;
            let gy = y + g.oy as i32;
            let mut d = Rect { x: gx, y: gy, w: g.w as i32, h: g.h as i32 };
            if fit(cv, clipr, &mut d) {
                for yy in d.y..d.y + d.h {
                    let row = yy as usize * cv.w as usize;
                    let m = (yy - gy) as usize * g.w as usize;
                    for xx in d.x..d.x + d.w {
                        let cov = g.alpha[m + (xx - gx) as usize] as u32;
                        if cov != 0 {
                            let w = (cov + (cov >> 7)) * a >> 8;
                            let p = &mut cv.px[row + xx as usize];
                            *p = if additive { add(*p, color, w) } else { mix(*p, color, w) };
                        }
                    }
                }
            }
        }
    }
    pen + f.advance(c) as u64
}
