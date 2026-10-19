//! Allen's presence, ported from native/kernel/ui/mind.[ch]: no face and no body, a mind drawn as
//! light. About a hundred points of light on and inside a sphere, joined by fine threads, turning
//! slowly round a warm heart; sparks run along the threads. Integer only.
use crate::font::isqrt;
use crate::paint::*;

pub const NODES: usize = 120;
pub const EDGES: usize = 300;
pub const SPARKS: usize = 64;
pub const FORM_MS: u32 = 3000;

pub const IDLE: i32 = 0;
pub const LISTEN: i32 = 1;
pub const THINK: i32 = 2;
pub const SPEAK: i32 = 3;

const INNER: usize = 24;
const TILT: u32 = 3641;

#[derive(Clone, Copy, Default)]
struct Spark {
    e: u16,
    dur: u16,
    back: u8,
    t0: u32,
}

pub struct Mind {
    pub cx: i32,
    pub cy: i32,
    pub r: i32,
    pub k: u32,
    home: [[i16; 3]; NODES],
    ea: Vec<u8>,
    eb: Vec<u8>,
    pub born: u32,
    pub t: u32,
    rng: u32,
    spawn: u32,
    pub mode: i32,
    angle: u32,
    spin: u32,
    squeeze: u32,
    pub heart: u32,
    voice: u32,
    words: [u32; 4],
    keys: [u32; 4],
    act: [u16; NODES],
    spark: Vec<Spark>,
    px: [i32; NODES],
    py: [i32; NODES],
    lum: [u16; NODES],
    near: [u16; NODES],
    pub alpha: u32,
    hx: i32,
    hy: i32,
    hr: i32,
    pub bx: Rect,
    pub was: Rect,
}

fn dist2(a: &[i16; 3], b: &[i16; 3]) -> i64 {
    let dx = a[0] as i64 - b[0] as i64;
    let dy = a[1] as i64 - b[1] as i64;
    let dz = a[2] as i64 - b[2] as i64;
    dx * dx + dy * dy + dz * dz
}

fn toward(cur: u32, want: u32, dt: u32, ms: u32) -> u32 {
    let d = want as i64 - cur as i64;
    let mut step = d * (if dt < ms { dt } else { ms }) as i64 / ms as i64;
    if step == 0 && d != 0 && dt != 0 {
        step = if d > 0 { 1 } else { -1 };
    }
    (cur as i64 + step) as u32
}

/// (1 - (off/width)^2)^2, 0..256
fn pulse(off: i32, width: i32) -> u32 {
    if off <= -width || off >= width {
        return 0;
    }
    let q = 256 - (off as i64 * off as i64 * 256 / (width as i64 * width as i64));
    (q * q >> 8) as u32
}

fn push(ring: &mut [u32; 4], t: u32) {
    for i in (1..4).rev() {
        ring[i] = ring[i - 1];
    }
    ring[0] = if t != 0 { t } else { 1 };
}

impl Mind {
    fn s(&self, v: i64) -> i32 {
        ((v * self.k as i64 + 128) / 256) as i32
    }

    fn rnd(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn has_edge(&self, a: usize, b: usize) -> bool {
        (0..self.ea.len()).any(|i| (self.ea[i] as usize == a && self.eb[i] as usize == b) || (self.ea[i] as usize == b && self.eb[i] as usize == a))
    }

    /// (cx, cy) its centre, px; k the scale (1/256 px per design px); born the time it starts to gather (ms).
    pub fn new(cx: i32, cy: i32, k: u32, born: u32) -> Mind {
        let mut m = Mind {
            cx,
            cy,
            r: 0,
            k,
            home: [[0; 3]; NODES],
            ea: Vec::new(),
            eb: Vec::new(),
            born,
            t: 0,
            rng: 0x9e3779b9,
            spawn: 0,
            mode: IDLE,
            angle: 0,
            spin: 349,
            squeeze: 0,
            heart: 0,
            voice: 0,
            words: [0; 4],
            keys: [0; 4],
            act: [0; NODES],
            spark: Vec::new(),
            px: [0; NODES],
            py: [0; NODES],
            lum: [0; NODES],
            near: [0; NODES],
            alpha: 0,
            hx: 0,
            hy: 0,
            hr: 0,
            bx: Rect::default(),
            was: Rect::default(),
        };
        m.r = m.s(118);
        let outer = NODES - INNER;
        for i in 0..NODES {
            let (y, x, z, len): (i64, i64, i64, i64);
            if i < outer {
                y = 16384 - (2 * i as i64 + 1) * 16384 / outer as i64;
                let rr = isqrt((16384 * 16384 - y * y) as u64) as i64;
                let th = (i as u32) * 25033;
                x = rr * cos(th) as i64 >> 14;
                z = rr * sin(th) as i64 >> 14;
                len = 16384 - 2950 + (hash(i as i32, 1, 31) % 5900) as i64;
            } else {
                let h1 = hash(i as i32, 2, 31);
                let h2 = hash(i as i32, 3, 31);
                y = 16384 - (h1 % 32768) as i64;
                let rr = isqrt((16384 * 16384 - y * y) as u64) as i64;
                x = rr * cos(h2) as i64 >> 14;
                z = rr * sin(h2) as i64 >> 14;
                len = 4915 + (hash(i as i32, 4, 31) % 5734) as i64;
            }
            m.home[i][0] = (x * len >> 14) as i16;
            m.home[i][1] = (y * len >> 14) as i16;
            m.home[i][2] = (z * len >> 14) as i16;
        }
        for i in 0..NODES {
            let want = if i < outer { 2 } else { 3 };
            for _ in 0..want {
                let mut best: i32 = -1;
                let mut bd: i64 = 0;
                for j in 0..NODES {
                    if j == i || m.has_edge(i, j) {
                        continue;
                    }
                    let d = dist2(&m.home[i], &m.home[j]);
                    if best < 0 || d < bd {
                        best = j as i32;
                        bd = d;
                    }
                }
                if best >= 0 && m.ea.len() < EDGES {
                    m.ea.push(i as u8);
                    m.eb.push(best as u8);
                }
            }
        }
        m
    }

    pub fn set_mode(&mut self, mode: i32, _t: u32) {
        self.mode = mode;
    }

    /// a word was spoken
    pub fn word(&mut self, t: u32) {
        push(&mut self.words, t);
    }

    /// a key was typed
    pub fn key(&mut self, t: u32) {
        push(&mut self.keys, t);
    }

    pub fn set_voice(&mut self, level: u32) {
        self.voice = if level > 256 { 256 } else { level };
    }

    fn spark_from(&mut self, from: usize) {
        let ne = self.ea.len();
        if self.spark.len() >= SPARKS || ne == 0 {
            return;
        }
        let mut e = (self.rnd() as usize) % ne;
        let mut tries = 0usize;
        if from < NODES {
            while self.ea[e] as usize != from && self.eb[e] as usize != from {
                let go = tries < ne;
                tries += 1;
                if !go {
                    break;
                }
                e = (e + 1) % ne;
            }
        }
        let back = if from < NODES { (self.eb[e] as usize == from) as u8 } else { (self.rnd() & 1) as u8 };
        let dur = if self.mode == THINK { 240 + self.rnd() % 260 } else { 420 + self.rnd() % 480 } as u16;
        self.spark.push(Spark { e: e as u16, dur, back, t0: self.t });
    }

    pub fn at(&mut self, t: u32) {
        let mut dt = if t > self.t { t - self.t } else { 0 };
        if dt > 250 {
            dt = 250;
        }
        self.t = t;
        self.was = self.bx;
        if t < self.born {
            self.alpha = 0;
            self.bx = rect(0, 0, 0, 0);
            return;
        }
        static SPINS: [u32; 4] = [349, 520, 1400, 600];
        static SQUEEZES: [u32; 4] = [0, 4, 16, 0];
        static HEARTS: [u32; 4] = [50, 70, 120, 80];
        static RATES: [u32; 4] = [2, 6, 25, 12];
        let md = (self.mode & 3) as usize;
        self.spin = toward(self.spin, SPINS[md], dt, 700);
        self.squeeze = toward(self.squeeze, SQUEEZES[md], dt, 500);
        let mut want_heart = HEARTS[md] + if md == SPEAK as usize { self.voice * 120 / 256 } else { 0 };
        if md == THINK as usize {
            want_heart += ((16384 + sin(t.wrapping_mul(65536) / 900)) as u32 * 30) >> 15;
        }
        self.heart = toward(self.heart, want_heart, dt, if md == SPEAK as usize { 80 } else { 400 });
        self.angle = self.angle.wrapping_add(self.spin.wrapping_mul(dt));

        let form = prog(t, self.born, self.born + FORM_MS) as u32;
        if form >= 192 {
            self.spawn += RATES[md] * dt;
            while self.spawn >= 1000 {
                self.spawn -= 1000;
                self.spark_from(NODES);
            }
        }
        let mut i = 0;
        while i < self.spark.len() {
            let age = t.wrapping_sub(self.spark[i].t0);
            if age < self.spark[i].dur as u32 {
                i += 1;
                continue;
            }
            let e = self.spark[i].e as usize;
            let to = if self.spark[i].back != 0 { self.ea[e] } else { self.eb[e] } as usize;
            self.act[to] = 256;
            self.spark.swap_remove(i);
            let go = if md == THINK as usize { self.rnd() % 100 < 70 } else { self.rnd() % 100 < 18 };
            if go {
                self.spark_from(to);
            }
        }
        let decay = exp_neg(dt * 256 / 260);
        for i in 0..NODES {
            self.act[i] = (self.act[i] as u32 * decay >> 16) as u16;
        }

        let mut scale: i64 = 65536 + ((sin(t.wrapping_mul(65536) / 5500) as i64 * 1966) >> 14) - self.squeeze as i64 * 65536 / 256;
        let gather = ease_out(form as i32);
        scale = scale * (256 + (256 - gather) as i64 * 560 / 256) >> 8;
        let swirl = (256 - gather) * (256 - gather) >> 8;
        self.alpha = smooth(form as i32 * 3);
        let a = (self.angle >> 8).wrapping_add(swirl * 256);
        let ca = cos(a) as i64;
        let sa = sin(a) as i64;
        let ct = cos(TILT) as i64;
        let st = sin(TILT) as i64;
        let r16 = self.r as i64 * 16;
        let mut x0 = self.cx * 16;
        let mut x1 = x0;
        let mut y0 = self.cy * 16;
        let mut y1 = y0;
        for i in 0..NODES {
            let x = self.home[i][0] as i64;
            let y = self.home[i][1] as i64;
            let z = self.home[i][2] as i64;
            let rx = (x * ca - z * sa) >> 14;
            let rz = (x * sa + z * ca) >> 14;
            let ry = (y * ct - rz * st) >> 14;
            let fz = (y * st + rz * ct) >> 14;
            let persp = 16384 + fz * 3 / 16;
            self.px[i] = (self.cx as i64 * 16 + 8 + (rx * persp >> 14) * r16 / 16384 * scale / 65536) as i32;
            self.py[i] = (self.cy as i64 * 16 + 8 + (ry * persp >> 14) * r16 / 16384 * scale / 65536) as i32;
            let nr = ((fz + 16384) * 256 / 32768) as u32;
            self.near[i] = (if nr > 256 { 256 } else { nr }) as u16;
            let rad = isqrt((x * x + y * y + z * z) as u64) as i64 * 256 / 16384;
            let mut extra: u32 = 0;
            for w in 0..4 {
                if self.words[w] != 0 && t.wrapping_sub(self.words[w]) < 900 {
                    let age = t - self.words[w];
                    extra += pulse(rad as i32 - (age * 300 / 600) as i32, 70) * (900 - age) / 900 * 150 >> 8;
                }
                if self.keys[w] != 0 && t.wrapping_sub(self.keys[w]) < 600 {
                    let age = t - self.keys[w];
                    let ky = 300 - (age * 600 / 500) as i32;
                    let ny = (y * 256 / 16384) as i32;
                    extra += pulse(ny - ky, 60) * (600 - age) / 600 * 90 >> 8;
                }
            }
            let nn = self.near[i] as u32;
            let l = 34 + nn * nn / 256 * 180 / 256 + extra + self.act[i] as u32 * 120 / 256;
            self.lum[i] = (if l > 256 { 256 } else { l }) as u16;
            x0 = min(x0, self.px[i]);
            x1 = max(x1, self.px[i]);
            y0 = min(y0, self.py[i]);
            y1 = max(y1, self.py[i]);
        }
        self.hx = self.cx * 16 + 8;
        self.hy = self.cy * 16 + 8;
        self.hr = (r16 * 52 / 100 * scale / 65536) as i32;
        let halo = (r16 * 17 / 10 * min((scale >> 8) as i32, 256) as i64 / 256) as i32;
        let pad = self.s(8) * 16;
        x0 = min(x0 - pad, self.hx - halo);
        x1 = max(x1 + pad, self.hx + halo);
        y0 = min(y0 - pad, self.hy - halo);
        y1 = max(y1 + pad, self.hy + halo);
        self.bx = rect((x0 >> 4) - 1, (y0 >> 4) - 1, ((x1 - x0) >> 4) + 3, ((y1 - y0) >> 4) + 3);
    }

    /// What needs redrawing for this frame: where it was and where it is.
    pub fn dirty(&self) -> Rect {
        union(self.was, self.bx)
    }

    /// The figure into the part r of out (light added).
    pub fn draw(&self, out: &mut Canvas, r: &Rect) {
        if self.alpha == 0 || !meets(self.bx, r) {
            return;
        }
        let big_a = self.alpha;
        let form = prog(self.t, self.born, self.born + FORM_MS);
        let r16 = self.r as i64 * 16;
        glow(out, r, self.hx, self.hy, (r16 * 17 / 10) as i32, rgb(60, 74, 210), (52 + self.heart / 8) * big_a >> 8);
        glow(out, r, self.hx, self.hy, r16 as i32, rgb(112, 92, 255), (30 + self.heart / 8) * big_a >> 8);
        let ta = smooth((form - 128) * 2) * big_a >> 8;
        if ta != 0 {
            for i in 0..self.ea.len() {
                let a = self.ea[i] as usize;
                let b = self.eb[i] as usize;
                let a0 = (self.lum[a] as u32 * (16 + self.near[a] as u32 * 60 / 256) >> 8) * ta >> 8;
                let a1 = (self.lum[b] as u32 * (16 + self.near[b] as u32 * 60 / 256) >> 8) * ta >> 8;
                thread(out, r, self.px[a], self.py[a], self.px[b], self.py[b], rgb(120, 140, 255), a0, a1);
                if self.k > 320 {
                    thread(out, r, self.px[a], self.py[a] + 16, self.px[b], self.py[b] + 16, rgb(120, 140, 255), a0 / 2, a1 / 2);
                }
            }
        }
        for i in 0..NODES {
            let rad = ((self.s(3) as i64 * 16 * (160 + self.near[i] as i64 * 120 / 256)) >> 8) as i32;
            let c = mix(rgb(206, 222, 255), rgb(255, 214, 160), self.act[i] as u32);
            glow(out, r, self.px[i], self.py[i], rad * 3, c, (self.lum[i] as u32 * 50 >> 8) * big_a >> 8);
            glow(out, r, self.px[i], self.py[i], rad, c, self.lum[i] as u32 * big_a >> 8);
        }
        for sp in &self.spark {
            let e = sp.e as usize;
            let (mut a, mut b) = (self.ea[e] as usize, self.eb[e] as usize);
            if sp.back != 0 {
                a = self.eb[e] as usize;
                b = self.ea[e] as usize;
            }
            let f = (self.t.wrapping_sub(sp.t0) * 256 / sp.dur as u32) as i32;
            for k in 0..3i32 {
                let g = clamp(f - k * 22, 0, 256);
                let x = self.px[a] + (((self.px[b] - self.px[a]) as i64 * g as i64) >> 8) as i32;
                let y = self.py[a] + (((self.py[b] - self.py[a]) as i64 * g as i64) >> 8) as i32;
                glow(out, r, x, y, self.s(5) * 16 - k * self.s(1) * 16, rgb(255, 222, 176), (200 >> k) * big_a >> 8);
            }
        }
        glow(out, r, self.hx, self.hy, self.hr, rgb(255, 210, 170), self.heart * 7 / 10 * big_a >> 8);
        glow(out, r, self.hx, self.hy, self.hr * 2 / 5, rgb(255, 236, 214), (90 + self.heart) * big_a >> 8);
        glow(out, r, self.hx, self.hy, self.hr / 8, rgb(255, 250, 240), 230 * big_a >> 8);
    }

    // accessors the interface needs
    pub fn edge_count(&self) -> usize {
        self.ea.len()
    }
}
