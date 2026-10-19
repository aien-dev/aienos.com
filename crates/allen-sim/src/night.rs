//! The night sky and the merging black holes, ported from native/kernel/ui/night.[ch].
//! Every pixel is a pure function of its position and the time. Integer only.
use crate::font::isqrt;
use crate::paint::*;
use std::sync::OnceLock;

pub const FADE_END_MS: u32 = 6800;
pub const HOLES_MS: u32 = 2600;
pub const SPIRAL_MS: u32 = 6000;
pub const MERGE_MS: u32 = 12000;
pub const RIPPLE_MS: u32 = 3600;
pub const GONE_MS: u32 = 15200;

pub const GALAXIES: usize = 6;
pub const TWINKLES: usize = 256;
pub const LEVELS: usize = 6;
const Q30: u64 = 1 << 30;

/// How the dusk stands at a frame. All distances in 16ths of a pixel.
#[derive(Clone, Copy, Default)]
pub struct Dusk {
    pub fallen: i64,
    pub swirl: u32,
    pub calm: i32,
    pub crowd: u32,
    pub e: i32,
    pub rs: i32,
    pub disk: u32,
    pub beam: u32,
    pub keep: u32,
    pub near: i32,
    pub far: i32,
    pub shade: u32,
    pub fine: i32,
}

#[derive(Clone, Default)]
pub struct Galaxy {
    pub bx: Rect,
    pub light: Vec<u8>,
    pub phase: Vec<u8>,
    pub tint: u32,
    pub period: u16,
}

#[derive(Clone, Copy, Default)]
pub struct Star {
    pub x: i32,
    pub y: i32,
    pub r: i32,
    pub color: u32,
    pub period: u16,
    pub phase: u16,
}

#[derive(Clone, Copy, Default)]
pub struct Hole {
    pub x: i32,
    pub y: i32,
    pub e: i32,
    pub rs: i32,
}

pub struct Night {
    pub w: i32,
    pub h: i32,
    pub k: u32,
    pub cx: i32,
    pub cy: i32,
    pub sky: Vec<u32>,
    pub appear: Vec<u8>,
    pub aw: i32,
    pub gal: Vec<Galaxy>,
    pub tw: Vec<Star>,
    pub t: u32,
    pub holes: usize,
    pub hole: [Hole; 2],
    pub lens: Rect,
    pub ripple_r: i32,
    pub ripple_w: i32,
    ripple_disp: [i32; 1024],
    ripple_light: [u8; 1024],
    ripple_c2: i32,
    ripple_s2: i32,
    pub falling: bool,
    pub dusk: Dusk,
    disk_a: [u8; 1024],
    disk_c: [u32; 1024],
    disk_far: i32,
}

/// The picture kept for the dusk: the screen as it was, with smaller copies of it and of the bare sky.
pub struct Picture {
    pub level: Vec<Vec<u32>>,
    pub sky: Vec<Vec<u32>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Spiral,
    Round,
    Edge,
}

struct GDef {
    fx: u32,
    fy: u32,
    radius: u32,
    tilt: i32,
    turn: i32,
    kind: Kind,
    bright: u32,
    tint: u32,
    period: u16,
}

static GDEF: [GDef; GALAXIES] = [
    GDef { fx: 150, fy: 205, radius: 112, tilt: 38, turn: 30, kind: Kind::Spiral, bright: 160, tint: rgb(190, 205, 255), period: 3400 },
    GDef { fx: 805, fy: 135, radius: 76, tilt: 62, turn: -25, kind: Kind::Spiral, bright: 140, tint: rgb(200, 212, 255), period: 2900 },
    GDef { fx: 265, fy: 705, radius: 42, tilt: 45, turn: 60, kind: Kind::Round, bright: 130, tint: rgb(255, 228, 196), period: 2600 },
    GDef { fx: 870, fy: 750, radius: 88, tilt: 84, turn: 15, kind: Kind::Edge, bright: 150, tint: rgb(210, 216, 255), period: 3800 },
    GDef { fx: 60, fy: 545, radius: 40, tilt: 22, turn: 0, kind: Kind::Spiral, bright: 120, tint: rgb(190, 205, 255), period: 3100 },
    GDef { fx: 600, fy: 95, radius: 28, tilt: 52, turn: 70, kind: Kind::Spiral, bright: 110, tint: rgb(200, 212, 255), period: 2700 },
];

struct Tabs {
    band_t: [u8; 256],
    pow2_t: [u32; 257],
}

fn tabs() -> &'static Tabs {
    static T: OnceLock<Tabs> = OnceLock::new();
    T.get_or_init(|| {
        let mut band_t = [0u8; 256];
        for i in 0..256u32 {
            let c = (16384 + cos(i * 256)) as u64 >> 1;
            let c2 = c * c >> 14;
            let c6 = (c2 * c2 >> 14) * c2 >> 14;
            band_t[i as usize] = (c6 * 255 >> 14) as u8;
        }
        let mut pow2_t = [0u32; 257];
        let mut e2: u64 = 1 << 30;
        for i in 0..256 {
            pow2_t[i] = ((e2 + (1 << 13)) >> 14) as u32;
            e2 = e2 * 1076652777 >> 30;
        }
        pow2_t[256] = 131072;
        Tabs { band_t, pow2_t }
    })
}

/// 2^(x / 65536), Q16; x below 40 << 16
fn pow2_q16(x: u32) -> u64 {
    let t = &tabs().pow2_t;
    let n = x >> 16;
    let f = x & 65535;
    let k = (f >> 8) as usize;
    let g = (f & 255) as u64;
    let m = t[k] as u64 + ((t[k + 1] - t[k]) as u64 * g >> 8);
    m << n
}

fn half_box(k: u32, i: usize) -> i32 {
    ((GDEF[i].radius * 13 / 10 * k + 128) / 256) as i32 + 1
}

fn galaxy_box(w: u32, h: u32, k: u32, i: usize) -> Rect {
    let hb = half_box(k, i);
    let gx = (w * GDEF[i].fx / 1000) as i32;
    let gy = (h * GDEF[i].fy / 1000) as i32;
    let all = rect(0, 0, w as i32, h as i32);
    let mut b = rect(gx - hb, gy - hb, 2 * hb + 1, 2 * hb + 1);
    if !clip(&mut b, &all) {
        b = rect(0, 0, 0, 0);
    }
    b
}

// ---- the Milky Way's geometry ----

struct Band {
    x0: i64,
    y0: i64,
    dx: i64,
    dy: i64,
    len: i64,
    half: i64,
}

fn band_of(n: &Night) -> Band {
    let (w, h) = (n.w as i64, n.h as i64);
    let mut b = Band { x0: -w / 20, y0: h * 90 / 100, dx: w * 110 / 100, dy: -h * 92 / 100, len: 0, half: h * 16 / 100 };
    b.len = isqrt((b.dx * b.dx + b.dy * b.dy) as u64) as i64;
    b
}

fn band_at(b: &Band, x: i64, y: i64) -> (i64, i64) {
    let across = ((x - b.x0) * b.dy - (y - b.y0) * b.dx) / b.len;
    let along = ((x - b.x0) * b.dx + (y - b.y0) * b.dy) / b.len;
    (across, along)
}

/// exp(-(v/width)^2), Q16
fn gauss(v: i64, width: i64) -> u32 {
    let q = v * 256 / if width != 0 { width } else { 1 };
    if q > 4095 || q < -4095 {
        return 0;
    }
    exp_neg((q * q >> 8) as u32)
}

impl Night {
    fn s(&self, v: i64) -> i32 {
        ((v * self.k as i64 + 128) / 256) as i32
    }

    // ---- the soft layer ----
    fn clouds(&self, q: &mut [u16], qw: i32, qh: i32) {
        let b = band_of(self);
        let hh = self.h as i64;
        for qy in 0..qh {
            for qx in 0..qw {
                let x = qx as i64 * 4;
                let y = qy as i64 * 4;
                let (across, along) = band_at(&b, x, y);
                let u3 = (x * 768 / hh) as i32;
                let v3 = (y * 768 / hh) as i32;
                let core = gauss(across, b.half) as u64;
                let wide = gauss(across, b.half * 2) as u64;
                let n1 = fbm(u3, v3, 101, 5) as u64;
                let n2 = fbm((x * 1792 / hh) as i32, (y * 1792 / hh) as i32, 202, 4) as u64;
                let lane = gauss(across - hh * 2 / 100, hh * 5 / 100) as u64;
                let dust = lane * smooth((((n2 as i64) - 22000) * 256 / 26000) as i32) as u64 >> 8;
                let bright = gauss(along - b.len * 40 / 100, b.len * 38 / 100) as u64;
                let mut mw = core * (32768 + n1 / 2) >> 16;
                mw = mw * (65536 - dust * 52428 / 65536) >> 16;
                mw = mw * (29491 + bright * 36045 / 65536) >> 16;
                let halo = wide * (39321 + n1 * 26214 / 65536) >> 16;
                let n4 = fbm((x * 512 / hh) as i32, (y * 512 / hh) as i32, 404, 4);
                let n5 = fbm((x * 666 / hh) as i32, (y * 666 / hh) as i32, 505, 4);
                let n6 = fbm((x * 1024 / hh) as i32, (y * 1024 / hh) as i32, 606, 4);
                let mut vi = smooth((((n4 as i64) - 36000) * 256 / 29535) as i32) as u64;
                let mut bl = smooth((((n5 as i64) - 35000) * 256 / 30535) as i32) as u64;
                let ro = smooth((((n6 as i64) - 33000) * 256 / 32535) as i32) as u64 * wide >> 16;
                vi = vi * vi >> 8;
                bl = bl * bl >> 8;
                let top = 256 - y * 256 / hh;
                let r = 26 + top / 32 + (mw * 44 * 16 >> 16) as i64 + (halo * 6 * 16 >> 16) as i64 + (vi * 22 * 16 >> 8) as i64 + (bl * 5 * 16 >> 8) as i64 + (ro * 22 * 16 >> 8) as i64;
                let g = 34 + top / 32 + (mw * 40 * 16 >> 16) as i64 + (halo * 8 * 16 >> 16) as i64 + (vi * 8 * 16 >> 8) as i64 + (bl * 12 * 16 >> 8) as i64 + (ro * 7 * 16 >> 8) as i64;
                let bb = 92 + top / 16 + (mw * 46 * 16 >> 16) as i64 + (halo * 16 * 16 >> 16) as i64 + (vi * 28 * 16 >> 8) as i64 + (bl * 30 * 16 >> 8) as i64 + (ro * 15 * 16 >> 8) as i64;
                let c = (qy as usize * qw as usize + qx as usize) * 3;
                q[c] = clamp(r as i32, 0, 4080) as u16;
                q[c + 1] = clamp(g as i32, 0, 4080) as u16;
                q[c + 2] = clamp(bb as i32, 0, 4080) as u16;
            }
        }
    }

    fn scale_up(&mut self, q: &[u16], qw: i32) {
        static BAYER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
        for y in 0..self.h {
            let qy = (y >> 2) as usize;
            let fy = (y & 3) as u32;
            let r0 = qy * qw as usize * 3;
            let r1 = r0 + qw as usize * 3;
            for x in 0..self.w {
                let qx = (x >> 2) as usize;
                let fx = (x & 3) as u32;
                let d = BAYER[((y & 3) * 4 + (x & 3)) as usize] as u32;
                let mut c: u32 = 0;
                for k in 0..3usize {
                    let a = q[r0 + qx * 3 + k] as u32 * (4 - fx) + q[r0 + qx * 3 + 3 + k] as u32 * fx;
                    let b = q[r1 + qx * 3 + k] as u32 * (4 - fx) + q[r1 + qx * 3 + 3 + k] as u32 * fx;
                    let v = ((a * (4 - fy) + b * fy) / 16 + d) >> 4;
                    c = c << 8 | if v > 255 { 255 } else { v };
                }
                self.sky[y as usize * self.w as usize + x as usize] = c;
            }
        }
    }

    // ---- stars ----
    fn stars(&mut self) {
        let (w, h) = (self.w, self.h);
        let all = rect(0, 0, w, h);
        let b = band_of(self);
        let k = self.k;
        let area = w as u32 * h as u32;
        let mut cv = Canvas { px: std::mem::take(&mut self.sky), w, h };
        for i in 0..area / 800 {
            let ii = i as i32;
            let x = (hash(ii, 1, 77) % (w as u32 * 16)) as i32;
            let y = (hash(ii, 2, 77) % (h as u32 * 16)) as i32;
            let (across, _along) = band_at(&b, (x >> 4) as i64, (y >> 4) as i64);
            if (hash(ii, 3, 77) & 65535) > 36000 + gauss(across, b.half) * 29535 / 65536 {
                continue;
            }
            let u = (hash(ii, 4, 77) & 255) as u64;
            let lv = 14 + (241 * (u * u * u * u) >> 32) as u32;
            let r = ((20 + 16 * lv / 255) * k / 256) as i32;
            glow(&mut cv, &all, x, y, r, star_color(hash(ii, 5, 77)), lv);
        }
        for i in 0..area / 170 {
            let ii = i as i32;
            let x = (hash(ii, 6, 78) % (w as u32 * 16)) as i32;
            let y = (hash(ii, 7, 78) % (h as u32 * 16)) as i32;
            let (across, _along) = band_at(&b, (x >> 4) as i64, (y >> 4) as i64);
            let c = gauss(across, b.half * 8 / 10) as u64;
            if (hash(ii, 8, 78) & 65535) as u64 > (c * c >> 16) {
                continue;
            }
            glow(&mut cv, &all, x, y, (20 * k / 256) as i32, rgb(226, 230, 255), 10 + hash(ii, 9, 78) % 44);
        }
        let mut nb = area / 50000;
        nb = if nb < 24 { 24 } else if nb > TWINKLES as u32 { TWINKLES as u32 } else { nb };
        for i in 0..nb {
            let ii = i as i32;
            let x = (hash(ii, 1, 91) % (w as u32 * 16)) as i32;
            let y = (hash(ii, 2, 91) % (h as u32 * 16)) as i32;
            let color = star_color(hash(ii, 3, 91) | 1);
            let lv = 190 + hash(ii, 4, 91) % 66;
            let r = (34 * k / 256) as i32;
            glow(&mut cv, &all, x, y, r, color, lv);
            glow(&mut cv, &all, x, y, r * 5, color, 22);
            let period = (2600 + hash(ii, 5, 91) % 4200) as u16;
            let phase = (hash(ii, 6, 91) % period as u32) as u16;
            self.tw.push(Star { x, y, r: r * 2, color, period, phase });
        }
        self.sky = cv.px;
    }

    // ---- galaxies ----
    fn galaxy(&mut self, i: usize) {
        let bx = galaxy_box(self.w as u32, self.h as u32, self.k, i);
        let cnt = (bx.w as usize) * (bx.h as usize);
        let mut light = vec![0u8; cnt];
        let mut phase = vec![0u8; cnt];
        let gd = &GDEF[i];
        let gx = self.w as i64 * gd.fx as i64 / 1000 * 16 + 8;
        let gy = self.h as i64 * gd.fy as i64 / 1000 * 16 + 8;
        let rg = self.s(gd.radius as i64) as i64 * 16;
        let turn = (gd.turn * 65536 / 360) as u32;
        let ct = cos(turn) as i64;
        let st = sin(turn) as i64;
        let mut ci = cos((gd.tilt * 65536 / 360) as u32) as i64;
        if ci < 600 {
            ci = 600;
        }
        let (kind, bright) = (gd.kind, gd.bright);
        for y in bx.y..bx.y + bx.h {
            for x in bx.x..bx.x + bx.w {
                let at = (y - bx.y) as usize * bx.w as usize + (x - bx.x) as usize;
                let dx = x as i64 * 16 + 8 - gx;
                let dy = y as i64 * 16 + 8 - gy;
                let u = (dx * ct + dy * st) >> 14;
                let v = (dy * ct - dx * st) >> 14;
                let vd = v * 16384 / ci;
                let rq = isqrt((u * u + vd * vd) as u64) as i64 * 256 / rg;
                if rq > 330 {
                    continue;
                }
                let th = atan2(vd as i32, u as i32);
                let nz = fbm((u * 1536 / rg) as i32, (vd * 1536 / rg) as i32, 700 + i as u32, 3);
                let fine = fbm((u * 5120 / rg) as i32, (vd * 5120 / rg) as i32, 750 + i as u32, 2);
                let vb = v * 32768 / (16384 + ci);
                let rb = isqrt((u * u + vb * vb) as u64) as i64 * 256 / rg;
                let (mut lb, mut ld, mut knot): (i64, i64, i64) = (0, 0, 0);
                let ph: u32;
                if kind == Kind::Round {
                    lb = gauss(rb, 30) as i64 * 8 / 10;
                    ld = exp_neg((rb * 1075 / 256) as u32) as i64 * 6 / 10;
                    ph = (rb * 512) as u32;
                } else {
                    let lr = log2_q16((if rq < 4 { 4 } else { rq }) as u64) as i64 - 8 * 65536;
                    let mut p = 2u32.wrapping_mul(th).wrapping_add((lr * 30000 >> 16) as u32);
                    let mut arm = ((16384 + cos(p)) >> 1) as i64;
                    arm = arm * arm >> 14;
                    let from = smooth((rq - 34) as i32 * 256 / 90) as i64;
                    arm = 16384 - ((16384 - arm) * from >> 8);
                    let clump = 32768 + nz as i64 * 22000 / 65536 + fine as i64 * 10768 / 65536;
                    lb = gauss(rb, 58) as i64 * 9 / 10;
                    ld = (exp_neg((rq * 12 / 5) as u32) as i64 * (8192 + (arm * 57344 >> 14) * clump / 65536)) >> 16;
                    if arm > 11468 && nz > 44000 && rq > 50 {
                        knot = ld * (nz as i64 - 44000) / 21535;
                    }
                    if kind == Kind::Edge {
                        let lane = gauss(v, rg * 5 / 100) as i64;
                        if rq < 240 {
                            ld = ld * (65536 - lane * 56000 / 65536) >> 16;
                            lb = lb * (65536 - lane * 40000 / 65536) >> 16;
                        }
                        p = (u * 32768 / rg) as u32;
                    }
                    ph = p;
                }
                let edge = 65536 - smooth((rq - 250) as i32 * 256 / 80) as i64 * 256;
                lb = lb * edge >> 16;
                ld = ld * edge >> 16;
                knot = knot * edge >> 16;
                let ab = (lb * bright as i64 >> 16) as u32;
                let ad = (ld * bright as i64 >> 16) as u32;
                let ak = (knot * bright as i64 >> 16) as u32;
                let pi = y as usize * self.w as usize + x as usize;
                let mut p = self.sky[pi];
                p = add(p, rgb(255, 222, 178), ab);
                p = add(p, rgb(150, 170, 255), ad);
                p = add(p, rgb(255, 140, 190), ak / 2);
                self.sky[pi] = p;
                let l = ab + ad;
                light[at] = if l > 255 { 255 } else { l as u8 };
                phase[at] = (ph >> 8) as u8;
            }
        }
        self.gal.push(Galaxy { bx, light, phase, tint: gd.tint, period: gd.period });
    }

    fn corners_far(&self) -> i64 {
        let mut far: i64 = 0;
        let cs = [(0, 0), (self.w, 0), (0, self.h), (self.w, self.h)];
        for (cx, cy) in cs {
            let dx = (cx - self.cx) as i64;
            let dy = (cy - self.cy) as i64;
            let d = isqrt((dx * dx + dy * dy) as u64) as i64;
            far = if d > far { d } else { far };
        }
        far
    }

    fn appear_map(&mut self) {
        let ah = self.h / 4 + 1;
        let far = self.corners_far();
        for y in 0..ah {
            for x in 0..self.aw {
                let px = x as i64 * 4;
                let py = y as i64 * 4;
                let dx = px - self.cx as i64;
                let dy = py - self.cy as i64;
                let a = fbm((px * 640 / self.h as i64) as i32, (py * 640 / self.h as i64) as i32, 909, 4) >> 8;
                let r = (isqrt((dx * dx + dy * dy) as u64) as i64 * 255 / if far != 0 { far } else { 1 }) as u32;
                self.appear[y as usize * self.aw as usize + x as usize] = ((a * 6 + r * 4) / 10) as u8;
            }
        }
    }

    /// How far the sky has come out of the dark at (x, y) for light of level l (0..255): 0..256.
    pub fn faded(&self, x: i32, y: i32, l: u32) -> u32 {
        if self.t >= FADE_END_MS {
            return 256;
        }
        let x = clamp(x, 0, self.w - 1);
        let y = clamp(y, 0, self.h - 1);
        let from = 300 + self.appear[(y >> 2) as usize * self.aw as usize + (x >> 2) as usize] as i32 * 12 + (255 - l as i32) * 8;
        let dt = self.t as i32 - from;
        if dt <= 0 { 0 } else if dt >= 1365 { 256 } else { smooth(dt * 48 >> 8) }
    }

    fn fade_px(&self, c: u32, x: i32, y: i32) -> u32 {
        let (r, g, b) = (c >> 16 & 255, c >> 8 & 255, c & 255);
        let mut l = if r > g { r } else { g };
        l = if l > b { l } else { b };
        let f = self.faded(x, y, l);
        if f >= 256 { c } else { scale(c, f) }
    }

    /// Makes the sky. (cx, cy): where the holes merge.
    pub fn new(w: u32, h: u32, k: u32, cx: i32, cy: i32) -> Option<Night> {
        if w < 64 || h < 64 || w > 8192 {
            return None;
        }
        let _ = tabs();
        let mut n = Night {
            w: w as i32,
            h: h as i32,
            k,
            cx,
            cy,
            sky: vec![0; (w * h) as usize],
            appear: vec![0; ((w / 4 + 1) * (h / 4 + 1)) as usize],
            aw: (w / 4 + 1) as i32,
            gal: Vec::new(),
            tw: Vec::new(),
            t: 0,
            holes: 0,
            hole: [Hole::default(); 2],
            lens: Rect::default(),
            ripple_r: 0,
            ripple_w: 0,
            ripple_disp: [0; 1024],
            ripple_light: [0; 1024],
            ripple_c2: 0,
            ripple_s2: 0,
            falling: false,
            dusk: Dusk::default(),
            disk_a: [0; 1024],
            disk_c: [0; 1024],
            disk_far: 0,
        };
        let qw = (w / 4 + 2) as i32;
        let qh = (h / 4 + 2) as i32;
        let mut q = vec![0u16; qw as usize * qh as usize * 3];
        n.clouds(&mut q, qw, qh);
        n.scale_up(&q, qw);
        n.stars();
        for i in 0..GALAXIES {
            n.galaxy(i);
        }
        n.appear_map();
        n.at(0);
        Some(n)
    }

    // ---- the black holes and the ripple at a time ----

    /// Moves the night to t ms: the holes, the lens region, the ripple. Ends any dusk.
    pub fn at(&mut self, t: u32) {
        self.t = t;
        self.holes = 0;
        self.falling = false;
        self.lens = rect(0, 0, 0, 0);
        self.ripple_w = 0;
        let e_full = self.s(60) * 16;
        let rs_full = self.s(20) * 16;
        let a0 = self.s(270) * 16;
        let cx = self.cx * 16 + 8;
        let cy = self.cy * 16 + 8;
        let (_s, last) = orbit_angle(MERGE_MS - 1, a0);
        if t >= HOLES_MS && t < MERGE_MS {
            let (sep, ang) = orbit_angle(t, a0);
            let e = (e_full as i64 * smooth(prog(t, HOLES_MS, HOLES_MS + 2600)) as i64 >> 8) as i32;
            let rs = (rs_full as i64 * smooth(prog(t, HOLES_MS + 1400, HOLES_MS + 3400)) as i64 >> 8) as i32;
            let hx = ((sep / 2) as i64 * cos(ang) as i64 >> 14) as i32;
            let hy = ((sep / 2) as i64 * sin(ang) as i64 >> 15) as i32;
            self.hole[0] = Hole { x: cx + hx, y: cy + hy, e, rs };
            self.hole[1] = Hole { x: cx - hx, y: cy - hy, e, rs };
            self.holes = 2;
        } else if t >= MERGE_MS && t < GONE_MS {
            let tm = t - MERGE_MS;
            let e = e_full * 362 / 256;
            let rs0 = rs_full * 323 / 256;
            let rs1 = rs_full * 2;
            let mut rs = rs0 + ((rs1 - rs0) as i64 * smooth(prog(tm, 0, 180)) as i64 >> 8) as i32;
            let wob = (sin(tm * 65536 / 95) as i64 * exp_neg(tm * 256 / 260) as i64) >> 16;
            rs += (rs as i64 * wob * 7 / 100 >> 14) as i32;
            let keep = 256 - smooth(prog(t, MERGE_MS + 900, GONE_MS));
            self.hole[0] = Hole { x: cx, y: cy, e: (e as i64 * keep as i64 >> 8) as i32, rs: (rs as i64 * (keep * keep >> 8) as i64 >> 8) as i32 };
            self.holes = 1;
        }
        for i in 0..self.holes {
            let mut reach = (self.hole[i].e * 44 / 10 >> 4) + 2;
            if self.hole[i].e <= 0 && self.hole[i].rs <= 0 {
                continue;
            }
            reach = max(reach, (self.hole[i].rs * 16 / 10 >> 4) + 2);
            let b = rect((self.hole[i].x >> 4) - reach, (self.hole[i].y >> 4) - reach, 2 * reach + 1, 2 * reach + 1);
            self.lens = union(self.lens, b);
        }
        let all = rect(0, 0, self.w, self.h);
        if self.lens.w != 0 && !clip(&mut self.lens, &all) {
            self.lens = rect(0, 0, 0, 0);
        }
        if t >= MERGE_MS && t < MERGE_MS + RIPPLE_MS {
            let tr = t - MERGE_MS;
            let w = clamp(self.s(60), 4, 200);
            let reach = w * 22 / 10;
            let far = self.corners_far() + reach as i64;
            self.ripple_r = (far * tr as i64 / RIPPLE_MS as i64) as i32;
            self.ripple_w = w;
            let env = smooth(prog(tr, 0, 220)) * (256 - smooth(prog(tr, RIPPLE_MS * 6 / 10, RIPPLE_MS))) >> 8;
            let mut amp = (self.s(6) as i64 * 16 * (256 - 150 * self.ripple_r as i64 / far)) >> 8;
            amp = amp * env as i64 >> 8;
            let lambda = max(self.s(110), 8);
            let mut i = 0;
            while i <= 2 * reach && i < 1024 {
                let o = i - reach;
                let s = sin((o as i64 * 65536 / lambda as i64) as u32) as i64;
                self.ripple_disp[i as usize] = ((amp * s >> 14) * gauss(o as i64, w as i64) as i64 >> 16) as i32;
                self.ripple_light[i as usize] = ((gauss(o as i64, (w * 6 / 10) as i64) as u64 * env as u64 * 46) >> 24) as u8;
                i += 1;
            }
            self.ripple_c2 = cos(last.wrapping_mul(2));
            self.ripple_s2 = sin(last.wrapping_mul(2));
        }
    }

    // ---- drawing the base ----

    /// the sky at (sx, sy), 16ths of a pixel (pixel i's centre at 16 i + 8)
    fn sample(&self, sx: i32, sy: i32) -> u32 {
        let sx = clamp(sx - 8, 0, (self.w - 1) * 16);
        let sy = clamp(sy - 8, 0, (self.h - 1) * 16);
        let ix = (sx >> 4) as usize;
        let iy = (sy >> 4) as usize;
        let ix1 = min((ix + 1) as i32, self.w - 1) as usize;
        let iy1 = min((iy + 1) as i32, self.h - 1) as usize;
        let w = self.w as usize;
        let fx = (sx & 15) as u32;
        let fy = (sy & 15) as u32;
        lerp16(lerp16(self.sky[iy * w + ix], self.sky[iy * w + ix1], fx), lerp16(self.sky[iy1 * w + ix], self.sky[iy1 * w + ix1], fx), fy)
    }

    /// the holes' pull on the light at (X, Y) (16ths of a pixel): added to (sx, sy)
    fn lens_at(&self, xx: i64, yy: i64, sx: &mut i64, sy: &mut i64) {
        for i in 0..self.holes {
            let e = self.hole[i].e as i64;
            let dx = xx - self.hole[i].x as i64;
            let dy = yy - self.hole[i].y as i64;
            let mut d2 = dx * dx + dy * dy;
            let out = e * e * 1936 / 100;
            let inn = e * e * 484 / 100;
            if e <= 0 || d2 >= out {
                continue;
            }
            if d2 < 64 {
                d2 = 64;
            }
            let mut f = e * e * 65536 / d2;
            if d2 > inn {
                let w = (out - d2) * 65536 / (out - inn);
                f = f * (w * w >> 16) >> 16;
            }
            *sx -= dx * f >> 16;
            *sy -= dy * f >> 16;
        }
    }

    /// where (X, Y) looks in the sky: offset (ax, ay), and the ripple's own faint light
    fn warp(&self, xx: i64, yy: i64) -> (i32, i32, u32) {
        let (mut sx, mut sy) = (0i64, 0i64);
        let mut light = 0u32;
        self.lens_at(xx, yy, &mut sx, &mut sy);
        if self.ripple_w != 0 {
            let dx = xx - (self.cx as i64 * 16 + 8);
            let dy = yy - (self.cy as i64 * 16 + 8);
            let d2 = dx * dx + dy * dy;
            let r16 = isqrt(d2 as u64) as i64;
            let reach = self.ripple_w as i64 * 22 / 10;
            let i = (r16 >> 4) - self.ripple_r as i64 + reach;
            if r16 > 0 && i >= 0 && i <= 2 * reach && i < 1024 {
                let q = ((dx * dx - dy * dy) * self.ripple_c2 as i64 + 2 * dx * dy * self.ripple_s2 as i64) / d2;
                let m = (self.ripple_disp[i as usize] as i64 * (5734 + (q * 10650 >> 14))) >> 14;
                sx -= m * dx / r16;
                sy -= m * dy / r16;
                light = self.ripple_light[i as usize] as u32;
            }
        }
        (sx as i32, sy as i32, light)
    }

    fn finish(&self, c: u32, x: i32, y: i32, light: u32) -> u32 {
        let c = if self.t < FADE_END_MS { self.fade_px(c, x, y) } else { c };
        if light != 0 { add(c, rgb(86, 108, 176), light) } else { c }
    }

    /// a pixel close to a hole: the pull worked out exactly, then the shadow and its thin ring of light
    fn near_px(&self, x: i32, y: i32) -> u32 {
        let xx = x as i64 * 16 + 8;
        let yy = y as i64 * 16 + 8;
        let (ax, ay, light) = self.warp(xx, yy);
        let mut c = self.finish(self.sample(xx as i32 + ax, yy as i32 + ay), x, y, light);
        for i in 0..self.holes {
            let rs = self.hole[i].rs as i64;
            let dx = xx - self.hole[i].x as i64;
            let dy = yy - self.hole[i].y as i64;
            let d2 = dx * dx + dy * dy;
            if rs <= 0 || d2 >= rs * rs * 9 / 4 {
                continue;
            }
            let r = isqrt(d2 as u64) as i32;
            c = mix(c, 0, cover(r - rs as i32));
            let ring = abs(r - (rs * 104 / 100) as i32);
            let mut a = if ring < 22 { (22 - ring) as u32 * 6 } else { 0 };
            if r as i64 > rs && (r as i64) < rs * 3 / 2 {
                a = a.wrapping_add(((rs * 3 / 2 - r as i64) * 46 / (rs / 2 + 1)) as u32);
            }
            c = add(c, rgb(255, 228, 196), a);
        }
        c
    }

    fn near_hole(&self, x: i32, y: i32) -> bool {
        let xx = x as i64 * 16 + 8;
        let yy = y as i64 * 16 + 8;
        for i in 0..self.holes {
            let reach = max(self.hole[i].e, self.hole[i].rs * 3 / 2) as i64 + 8 * 16;
            let dx = xx - self.hole[i].x as i64;
            let dy = yy - self.hole[i].y as i64;
            if dx * dx + dy * dy < reach * reach {
                return true;
            }
        }
        false
    }

    /// does the 8 x 8 block at (bx, by) meet the band the ripple bends?
    fn in_ripple(&self, bx: i32, by: i32) -> bool {
        if self.ripple_w == 0 {
            return false;
        }
        let reach = self.ripple_w as i64 * 22 / 10 + 1;
        let lo = max(0, self.ripple_r - reach as i32) as i64;
        let hi = self.ripple_r as i64 + reach;
        let nx = (clamp(self.cx, bx, bx + 8) - self.cx) as i64;
        let ny = (clamp(self.cy, by, by + 8) - self.cy) as i64;
        let fx = max(abs(bx - self.cx), abs(bx + 8 - self.cx)) as i64;
        let fy = max(abs(by - self.cy), abs(by + 8 - self.cy)) as i64;
        nx * nx + ny * ny <= hi * hi && fx * fx + fy * fy >= lo * lo
    }

    /// The sky as seen at the current time (faded in, bent, rippled) into the rectangle r of out.
    pub fn base(&self, out: &mut Canvas, r: &Rect) {
        let mut d = *r;
        let all = rect(0, 0, self.w, self.h);
        if !clip(&mut d, &all) {
            return;
        }
        let fading = self.t < FADE_END_MS;
        let w = self.w as usize;
        for y in d.y..d.y + d.h {
            let src = y as usize * w;
            let dst = y as usize * out.w as usize;
            if fading {
                for x in d.x..d.x + d.w {
                    out.px[dst + x as usize] = self.fade_px(self.sky[src + x as usize], x, y);
                }
            } else {
                for x in d.x..d.x + d.w {
                    out.px[dst + x as usize] = self.sky[src + x as usize];
                }
            }
        }
        if self.lens.w == 0 && self.ripple_w == 0 {
            return;
        }
        let mut by = d.y & !7;
        while by < d.y + d.h {
            let mut bx = d.x & !7;
            while bx < d.x + d.w {
                let mut b = rect(bx, by, 8, 8);
                let lens = self.lens.w != 0 && meets(b, &self.lens);
                if (lens || self.in_ripple(bx, by)) && clip(&mut b, &d) {
                    if lens && self.near_hole(bx + 4, by + 4) {
                        for y in b.y..b.y + b.h {
                            for x in b.x..b.x + b.w {
                                out.px[y as usize * out.w as usize + x as usize] = self.near_px(x, y);
                            }
                        }
                    } else {
                        let mut ax = [0i32; 4];
                        let mut ay = [0i32; 4];
                        let mut li = [0u32; 4];
                        for c in 0..4i32 {
                            let (a, b2, l) = self.warp((bx + (c & 1) * 8) as i64 * 16, (by + (c >> 1) * 8) as i64 * 16);
                            ax[c as usize] = a;
                            ay[c as usize] = b2;
                            li[c as usize] = l;
                        }
                        for y in b.y..b.y + b.h {
                            let wy = 2 * (y - by) + 1;
                            for x in b.x..b.x + b.w {
                                let wx = 2 * (x - bx) + 1;
                                let w00 = (16 - wx) * (16 - wy);
                                let w10 = wx * (16 - wy);
                                let w01 = (16 - wx) * wy;
                                let w11 = wx * wy;
                                let sx = (ax[0] * w00 + ax[1] * w10 + ax[2] * w01 + ax[3] * w11) >> 8;
                                let sy = (ay[0] * w00 + ay[1] * w10 + ay[2] * w01 + ay[3] * w11) >> 8;
                                let l = ((li[0] as i32 * w00 + li[1] as i32 * w10 + li[2] as i32 * w01 + li[3] as i32 * w11) as u32) >> 8;
                                let c = self.sample(x * 16 + 8 + sx, y * 16 + 8 + sy);
                                out.px[y as usize * out.w as usize + x as usize] = self.finish(c, x, y, l);
                            }
                        }
                    }
                }
                bx += 8;
            }
            by += 8;
        }
    }

    // ---- the dusk ----

    /// The picture kept for the dusk, and its smaller copies (and the bare sky's).
    pub fn picture_take(&self, from: &[u32]) -> Picture {
        let (w, h) = (self.w as u32, self.h as u32);
        let count = (w * h) as usize;
        let mut level = vec![from[..count].to_vec()];
        let mut sky = vec![self.sky.clone()];
        for l in 1..LEVELS {
            let sw = w >> (l - 1);
            let sh = h >> (l - 1);
            level.push(halve(&level[l - 1], sw, sh));
            sky.push(halve(&sky[l - 1], sw, sh));
        }
        Picture { level, sky }
    }

    /// The dusk, after at(): one black hole at (cx, cy) with everything falling into it as d says.
    pub fn fall(&mut self, d: &Dusk) {
        self.falling = true;
        self.dusk = *d;
        let cx = self.cx * 16 + 8;
        let cy = self.cy * 16 + 8;
        let rs = d.rs;
        self.hole[0] = Hole { x: cx, y: cy, e: d.e, rs };
        self.holes = 1;
        self.ripple_w = 0;
        self.lens = rect(0, 0, self.w, self.h);
        self.disk_far = rs * 33 / 10 + 16;
        let inn = rs * 115 / 100;
        let peak = rs * 150 / 100;
        let ring = rs * 106 / 100;
        let hw = 20 + rs / 64;
        for i in 0..1024usize {
            let r = (self.disk_far as i64 * i as i64 / 1024) as i32;
            let mut a: u32 = 0;
            let mut c = rgb(255, 222, 180);
            if r >= inn && peak > inn {
                if r < peak {
                    let u = ((r - inn) as i64 * 256 / (peak - inn) as i64) as u32;
                    a = smooth(u as i32) * 220 >> 8;
                    c = mix(rgb(255, 222, 180), rgb(255, 212, 150), u);
                } else {
                    let v = ((r - peak) as i64 * 256 / (rs / 2 + 1) as i64) as u32;
                    a = (exp_neg(v) >> 8) * 220 >> 8;
                    c = mix(rgb(255, 212, 150), rgb(150, 170, 255), smooth((v * 256 / 360) as i32));
                }
            }
            let off = abs(r - ring);
            if off < hw {
                let mut ar = (hw - off) as u32 * 255 / hw as u32;
                ar = ar * ar >> 8;
                c = if a + ar != 0 { mix(c, rgb(255, 244, 228), ar * 256 / (a + ar)) } else { c };
                a += ar;
            }
            a = a * d.disk >> 8;
            self.disk_a[i] = if a > 255 { 255 } else { a as u8 };
            self.disk_c[i] = c;
        }
    }

    fn corner_at(&self, x: i32, y: i32) -> Corner {
        let d = &self.dusk;
        let xx = x as i64 * 16;
        let yy = y as i64 * 16;
        let dx = xx - self.hole[0].x as i64;
        let dy = yy - self.hole[0].y as i64;
        let mut d2 = dx * dx + dy * dy;
        if d2 < 256 {
            d2 = 256;
        }
        let l0 = log2_q16(d2 as u64);
        let l1 = log2_q16((d2 + d.fallen) as u64);
        let h = l1.wrapping_sub(l0);
        let crowd = pow2_q16(h) >> 8;
        let ccrowd = if crowd > d.crowd as u64 { d.crowd } else { crowd as u32 };
        let r = (pow2_q16(l0 >> 1) >> 16) as i64;
        let mut ang = (d.swirl as u64 * (h >> 1) as u64 >> 16) as u32;
        ang = (ang as i64 * d.calm as i64 / (d.calm as i64 + r)) as u32;
        let stretch = pow2_q16(h >> 1) as i64;
        let cs = cos(ang) as i64;
        let sn = sin(ang) as i64;
        let ux = (dx * cs - dy * sn) >> 14;
        let uy = (dx * sn + dy * cs) >> 14;
        let mut sx = (ux * stretch >> 16) - dx;
        let mut sy = (uy * stretch >> 16) - dy;
        self.lens_at(xx, yy, &mut sx, &mut sy);
        let lim = 1i64 << 22;
        sx = if sx > lim { lim } else if sx < -lim { -lim } else { sx };
        sy = if sy > lim { lim } else if sy < -lim { -lim } else { sy };
        let ax = sx as i32;
        let ay = sy as i32;
        Corner {
            ax,
            ay,
            sx: xx as i32 + ax,
            sy: yy as i32 + ay,
            crowd: ccrowd,
            h,
            r: r as i32,
            inv: ((1i64 << 28) / (if r > 0 { r } else { 1 })) as i32,
            vig: 256 - (d.shade * smooth(prog(r as u32, d.near as u32, d.far as u32)) >> 8),
        }
    }

    /// The kept picture as the dusk shows it now, into the rectangle r of out.
    pub fn base_from(&self, kept: &Picture, out: &mut Canvas, r: &Rect) {
        let mut d = *r;
        let all = rect(0, 0, self.w, self.h);
        if !clip(&mut d, &all) {
            return;
        }
        if !self.falling {
            self.base(out, &d);
            return;
        }
        const CHUNK: i32 = 32;
        let bx0 = d.x & !7;
        let bx1 = d.x + d.w;
        let by0 = d.y & !7;
        let by1 = d.y + d.h;
        let mut cx0 = bx0;
        while cx0 < bx1 {
            let ncol = min(CHUNK, (bx1 - cx0 + 7) / 8);
            let mut top: Vec<Corner> = (0..=ncol).map(|i| self.corner_at(cx0 + 8 * i, by0)).collect();
            let mut by = by0;
            while by < by1 {
                let bottom: Vec<Corner> = (0..=ncol).map(|i| self.corner_at(cx0 + 8 * i, by + 8)).collect();
                for i in 0..ncol as usize {
                    self.fall_block(kept, out, &d, cx0 + 8 * i as i32, by, &top[i], &top[i + 1], &bottom[i], &bottom[i + 1]);
                }
                top = bottom;
                by += 8;
            }
            cx0 += CHUNK * 8;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fall_block(&self, kept: &Picture, out: &mut Canvas, d: &Rect, bx: i32, by: i32, c00: &Corner, c10: &Corner, c01: &Corner, c11: &Corner) {
        let mut b = rect(bx, by, 8, 8);
        if !clip(&mut b, d) {
            return;
        }
        let k = &self.dusk;
        let rs = k.rs;
        let hx = self.hole[0].x;
        let hy = self.hole[0].y;
        let rmin = min(min(c00.r, c10.r), min(c01.r, c11.r)) - 182;
        let rmax = max(max(c00.r, c10.r), max(c01.r, c11.r));
        let w16 = self.w * 32 + 16;
        let h16 = self.h * 32 + 16;
        let x16 = -self.w * 16 - 16;
        let y16 = -self.h * 16 - 16;
        let beyond = (c00.sx < x16 && c10.sx < x16 && c01.sx < x16 && c11.sx < x16)
            || (c00.sy < y16 && c10.sy < y16 && c01.sy < y16 && c11.sy < y16)
            || (c00.sx > w16 && c10.sx > w16 && c01.sx > w16 && c11.sx > w16)
            || (c00.sy > h16 && c10.sy > h16 && c01.sy > h16 && c11.sy > h16);
        let disk = k.disk != 0 && rmin < self.disk_far;
        let edge = rmin < rs + 16;
        let mut fine = rmin < k.fine;
        let keep = k.keep * ((c00.vig + c10.vig + c01.vig + c11.vig) >> 2) >> 8;
        let ow = out.w as usize;
        if (keep == 0 && !disk) || rmax < rs - 16 || (beyond && !disk) {
            for y in b.y..b.y + b.h {
                for x in b.x..b.x + b.w {
                    out.px[y as usize * ow + x as usize] = 0;
                }
            }
            return;
        }
        let hm = (c00.h >> 2) + (c10.h >> 2) + (c01.h >> 2) + (c11.h >> 2);
        let mut lvl = (hm + 98304) >> 17;
        lvl = if keep < 16 && lvl < 3 { 3 } else { lvl };
        lvl = if lvl >= LEVELS as u32 { LEVELS as u32 - 1 } else { lvl };
        let lw = self.w >> lvl;
        let lh = self.h >> lvl;
        let crowded = c00.crowd > 256 || c10.crowd > 256 || c01.crowd > 256 || c11.crowd > 256;
        let cb = cos(k.beam) as i64;
        let sb = sin(k.beam) as i64;
        let inv_far = (1u32 << 30) / (if self.disk_far > 0 { self.disk_far } else { 1 }) as u32;
        if !fine {
            let mut e = rect(b.x & !1, b.y & !1, 0, 0);
            e.w = ((b.x + b.w + 1) & !1) - e.x;
            e.h = ((b.y + b.h + 1) & !1) - e.y;
            if e.x >= d.x && e.y >= d.y && e.x + e.w <= d.x + d.w && e.y + e.h <= d.y + d.h {
                b = e;
            } else {
                fine = true;
            }
        }
        let step = if fine { 1 } else { 2 };
        let mut y = b.y;
        while y < b.y + b.h {
            let wy = 2 * (y - by) + step;
            let row = y as usize * ow;
            let mut x = b.x;
            while x < b.x + b.w {
                let wx = 2 * (x - bx) + step;
                let w00 = (16 - wx) * (16 - wy);
                let w10 = wx * (16 - wy);
                let w01 = (16 - wx) * wy;
                let w11 = wx * wy;
                let mut c: u32 = 0;
                if !beyond {
                    let ax = (c00.ax * w00 + c10.ax * w10 + c01.ax * w01 + c11.ax * w11) >> 8;
                    let ay = (c00.ay * w00 + c10.ay * w10 + c01.ay * w01 + c11.ay * w11) >> 8;
                    c = sample_kept(kept, lvl as usize, lw, lh, (x * 16 + 8 + ax) >> lvl, (y * 16 + 8 + ay) >> lvl);
                    if c != 0 && crowded {
                        let g = ((c00.crowd as i32 * w00 + c10.crowd as i32 * w10 + c01.crowd as i32 * w01 + c11.crowd as i32 * w11) as u32) >> 8;
                        c = brighten(c, g);
                    }
                    if keep < 256 {
                        c = scale(c, keep);
                    }
                }
                if disk || edge {
                    let r = (c00.r * w00 + c10.r * w10 + c01.r * w01 + c11.r * w11) >> 8;
                    if disk && r < self.disk_far {
                        let i = ((r as u32).wrapping_mul(inv_far) >> 20) as usize;
                        let ii = if i < 1024 { i } else { 1023 };
                        let mut a = self.disk_a[ii] as u32;
                        if a != 0 {
                            let inv = (c00.inv as i64 * w00 as i64 + c10.inv as i64 * w10 as i64 + c01.inv as i64 * w01 as i64 + c11.inv as i64 * w11 as i64) >> 8;
                            let dot = ((x * 16 + 8 - hx) as i64 * cb + (y * 16 + 8 - hy) as i64 * sb) >> 14;
                            let cosb = dot * inv >> 14;
                            a = ((a as i64 * (16384 + (cosb * 110 >> 8))) >> 14) as u32;
                            c = add(c, self.disk_c[ii], if a > 255 { 255 } else { a });
                        }
                    }
                    if edge {
                        c = mix(c, 0, cover(r - rs));
                    }
                }
                out.px[row + x as usize] = c;
                if !fine {
                    out.px[row + x as usize + 1] = c;
                    out.px[row + x as usize + ow] = c;
                    out.px[row + x as usize + ow + 1] = c;
                }
                x += step;
            }
            y += step;
        }
    }

    /// The rectangle a twinkling star needs redrawn.
    pub fn star_box(&self, i: usize) -> Rect {
        let s = &self.tw[i];
        rect((s.x - s.r) >> 4, (s.y - s.r) >> 4, ((2 * s.r) >> 4) + 2, ((2 * s.r) >> 4) + 2)
    }

    /// Adds the twinkling and, with shimmer > 0 (0..256), the light sweeping the galaxies; voice
    /// (0..256) brightens their cores.
    pub fn lights(&self, out: &mut Canvas, r: &Rect, shimmer: u32, voice: u32) {
        for i in 0..self.tw.len() {
            let s = &self.tw[i];
            let b = self.star_box(i);
            if !meets(b, r) || (self.lens.w != 0 && meets(b, &self.lens)) {
                continue;
            }
            let tw = (16384 + sin((self.t + s.phase as u32) % s.period as u32 * 65536 / s.period as u32)) as u32 >> 7;
            let mut a = (tw * tw >> 8) * 120 >> 8;
            a = a * self.faded(s.x >> 4, s.y >> 4, 255) >> 8;
            glow(out, r, s.x, s.y, s.r, s.color, a);
        }
        if shimmer == 0 && voice == 0 {
            return;
        }
        let band_t = &tabs().band_t;
        for g in &self.gal {
            let mut b = g.bx;
            if g.bx.w == 0 || !clip(&mut b, r) {
                continue;
            }
            let sweep = self.t % g.period as u32 * 256 / g.period as u32;
            for y in b.y..b.y + b.h {
                let base = (y - g.bx.y) as usize * g.bx.w as usize;
                let row = y as usize * out.w as usize;
                for x in b.x..b.x + b.w {
                    let li = base + (x - g.bx.x) as usize;
                    let l = g.light[li] as u32;
                    if l == 0 {
                        continue;
                    }
                    let mut a = (l * band_t[((g.phase[li] as u32).wrapping_sub(sweep) & 255) as usize] as u32 >> 8) * shimmer >> 8;
                    a += (l * l >> 8) * voice >> 9;
                    if a != 0 {
                        out.px[row + x as usize] = add(out.px[row + x as usize], g.tint, a);
                    }
                }
            }
        }
    }
}

fn star_color(h: u32) -> u32 {
    let p = h % 100;
    if p < 10 {
        rgb(170, 192, 255)
    } else if p < 55 {
        rgb(218, 228, 255)
    } else if p < 85 {
        rgb(255, 250, 242)
    } else if p < 97 {
        rgb(255, 234, 196)
    } else {
        rgb(255, 204, 160)
    }
}

/// l^(1/8), Q30
fn root8(mut l: u64) -> u64 {
    for _ in 0..3 {
        l = isqrt(l << 30);
    }
    l
}

const ORBIT_MS: u32 = 5200;

/// returns (separation in 16ths, angle)
fn orbit_angle(t: u32, a0: i32) -> (i32, u32) {
    let spiral_from = (SPIRAL_MS - HOLES_MS) * 65536 / ORBIT_MS;
    if t < SPIRAL_MS {
        return (a0, (t.wrapping_sub(HOLES_MS)).wrapping_mul(65536) / ORBIT_MS);
    }
    let left = (((MERGE_MS - t) as u64) << 30) / (MERGE_MS - SPIRAL_MS) as u64;
    let r = root8(left);
    let r2 = r * r >> 30;
    let r5 = (r2 * r2 >> 30) * r >> 30;
    let sep = (a0 as u64 * r2 >> 30) as i32;
    let turns = 65536u64 * 8 * (MERGE_MS - SPIRAL_MS) as u64 / (5 * ORBIT_MS as u64);
    (sep, spiral_from.wrapping_add((turns * (Q30 - r5) >> 30) as u32))
}

fn lerp16(a: u32, b: u32, f: u32) -> u32 {
    ((((a & 0xff00ff) * (16 - f) + (b & 0xff00ff) * f) >> 4) & 0xff00ff) | ((((a & 0x00ff00) * (16 - f) + (b & 0x00ff00) * f) >> 4) & 0x00ff00)
}

/// half the width and height of `from` (sw x sh): the mean of each 2 x 2
fn halve(from: &[u32], sw: u32, sh: u32) -> Vec<u32> {
    let dw = sw / 2;
    let dh = sh / 2;
    let mut to = vec![0u32; (dw * dh) as usize];
    for y in 0..dh as usize {
        let r0 = 2 * y * sw as usize;
        let r1 = r0 + sw as usize;
        for x in 0..dw as usize {
            let (a, b, c, e) = (from[r0 + 2 * x], from[r0 + 2 * x + 1], from[r1 + 2 * x], from[r1 + 2 * x + 1]);
            let rb = (((a & 0xff00ff) + (b & 0xff00ff) + (c & 0xff00ff) + (e & 0xff00ff)) >> 2) & 0xff00ff;
            let g = (((a & 0xff00) + (b & 0xff00) + (c & 0xff00) + (e & 0xff00)) >> 2) & 0xff00;
            to[y * dw as usize + x] = rb | g;
        }
    }
    to
}

struct Corner {
    ax: i32,
    ay: i32,
    sx: i32,
    sy: i32,
    crowd: u32,
    h: u32,
    r: i32,
    inv: i32,
    vig: u32,
}

/// the kept picture at (sx, sy) (16ths of a pixel of copy l, w x h)
fn sample_kept(kept: &Picture, l: usize, w: i32, h: i32, mut sx: i32, mut sy: i32) -> u32 {
    let mut px = &kept.level[l];
    let ww = (w - 1) * 16;
    let hh = (h - 1) * 16;
    sx -= 8;
    sy -= 8;
    if sx < 0 || sy < 0 || sx >= ww || sy >= hh {
        sx = if sx < 0 { -sx } else if sx >= ww { 2 * ww - sx } else { sx };
        sy = if sy < 0 { -sy } else if sy >= hh { 2 * hh - sy } else { sy };
        if sx < 0 || sy < 0 || sx >= ww || sy >= hh {
            return 0;
        }
        px = &kept.sky[l];
    }
    let ix = (sx >> 4) as usize;
    let iy = (sy >> 4) as usize;
    let r0 = iy * w as usize;
    let r1 = r0 + w as usize;
    let fx = (sx & 15) as u32;
    let fy = (sy & 15) as u32;
    lerp16(lerp16(px[r0 + ix], px[r0 + ix + 1], fx), lerp16(px[r1 + ix], px[r1 + ix + 1], fx), fy)
}

/// c brightened g/256 times (g above 256), the excess warmer than the light itself
fn brighten(c: u32, g: u32) -> u32 {
    if g <= 256 || c == 0 {
        return c;
    }
    let x = g - 256;
    let r = ch(c, 16) * (256 + x) >> 8;
    let gg = ch(c, 8) * (256 + x * 7 / 8) >> 8;
    let b = ch(c, 0) * (256 + x * 5 / 8) >> 8;
    (if r > 255 { 255 } else { r }) << 16 | (if gg > 255 { 255 } else { gg }) << 8 | (if b > 255 { 255 } else { b })
}
