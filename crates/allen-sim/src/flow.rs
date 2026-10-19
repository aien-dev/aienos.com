//! Allen's ink, ported from native/kernel/ui/flow.[ch]: the whole screen is a shallow tank of
//! water with a little luminous ink in it. Stam's stable fluid, all integer (16.16 fixed point) on
//! a coarse grid, drawn with bilinear interpolation.
use crate::font::isqrt;
use crate::paint::*;

pub const W: i32 = 64;
pub const H: i32 = 36;
pub const CELLS: usize = ((W + 2) * (H + 2)) as usize;

pub const IDLE: i32 = 0;
pub const LISTEN: i32 = 1;
pub const THINK: i32 = 2;
pub const SPEAK: i32 = 3;
pub const SLEEP: i32 = 4;

pub const STEP_MS: u32 = 16;

const Q: i32 = 65536;
const HALF: i32 = 32768;
const VISC_PER_MS: i32 = 205;
const INK_PER_MS: i32 = 328;
const INK_FADE_PER_MS: i32 = 33;
const VEL_FADE_PER_MS: i32 = 59;
const BLEND_MS: u32 = 700;
const SOLVE_STEPS: usize = 10;

fn ix(i: i32, j: i32) -> usize {
    (i + (W + 2) * j) as usize
}

fn mul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> 16) as i32
}

pub struct Flow {
    pub w: i32,
    pub h: i32,
    cw: i32,
    ch: i32,
    cx: i32,
    cy: i32,
    r: i32,
    bx: i32,
    by: i32,
    pub t: u32,
    stepped: u32,
    pub mode: i32,
    weight: [u32; 5],
    think_t: u32,
    pub word_t: u32,
    pub key_t: u32,
    rng: u32,
    u: Vec<i32>,
    v: Vec<i32>,
    d: Vec<i32>,
    u0: Vec<i32>,
    v0: Vec<i32>,
    d0: Vec<i32>,
    shown: Vec<i32>,
    warm: Vec<u16>,
    shown_warm: Vec<u16>,
    pub steps: u32,
}

fn sample(x: &[i32], gx: i32, gy: i32) -> i32 {
    let gx = clamp(gx - HALF, 0, (W + 1) * Q - 1);
    let gy = clamp(gy - HALF, 0, (H + 1) * Q - 1);
    let i = gx >> 16;
    let j = gy >> 16;
    let s1 = (gx & 0xffff) as i64;
    let t1 = (gy & 0xffff) as i64;
    let s0 = Q as i64 - s1;
    let t0 = Q as i64 - t1;
    let a = x[ix(i, j)] as i64 * t0 + x[ix(i, j + 1)] as i64 * t1;
    let b = x[ix(i + 1, j)] as i64 * t0 + x[ix(i + 1, j + 1)] as i64 * t1;
    ((a * s0 + b * s1) >> 32) as i32
}

fn set_bnd(x: &mut [i32], which: i32) {
    for i in 1..=W {
        x[ix(i, 0)] = if which == 2 { -x[ix(i, 1)] } else { x[ix(i, 1)] };
        x[ix(i, H + 1)] = if which == 2 { -x[ix(i, H)] } else { x[ix(i, H)] };
    }
    for j in 1..=H {
        x[ix(0, j)] = if which == 1 { -x[ix(1, j)] } else { x[ix(1, j)] };
        x[ix(W + 1, j)] = if which == 1 { -x[ix(W, j)] } else { x[ix(W, j)] };
    }
    x[ix(0, 0)] = (x[ix(1, 0)] + x[ix(0, 1)]) / 2;
    x[ix(0, H + 1)] = (x[ix(1, H + 1)] + x[ix(0, H)]) / 2;
    x[ix(W + 1, 0)] = (x[ix(W, 0)] + x[ix(W + 1, 1)]) / 2;
    x[ix(W + 1, H + 1)] = (x[ix(W, H + 1)] + x[ix(W + 1, H)]) / 2;
}

fn lin_solve(x: &mut [i32], x0: &[i32], a: i32, c: i32, which: i32) {
    let inv: i64 = (1i64 << 46) / c as i64;
    for _ in 0..SOLVE_STEPS {
        for j in 1..=H {
            for i in 1..=W {
                let sum = x[ix(i - 1, j)] as i64 + x[ix(i + 1, j)] as i64 + x[ix(i, j - 1)] as i64 + x[ix(i, j + 1)] as i64;
                let num = (((x0[ix(i, j)] as i64) << 16).wrapping_add((a as i64).wrapping_mul(sum))) >> 14;
                x[ix(i, j)] = (num.wrapping_mul(inv) >> 32) as i32;
            }
        }
        set_bnd(x, which);
    }
}

fn diffuse(x: &mut [i32], x0: &[i32], a: i32, which: i32) {
    lin_solve(x, x0, a, Q + 4 * a, which);
}

fn advect(d: &mut [i32], d0: &[i32], u: &[i32], v: &[i32], dt: i32, which: i32) {
    for j in 1..=H {
        for i in 1..=W {
            let mut x = (i << 16) - mul(dt, u[ix(i, j)]);
            let mut y = (j << 16) - mul(dt, v[ix(i, j)]);
            x = clamp(x, HALF, W * Q + HALF);
            y = clamp(y, HALF, H * Q + HALF);
            let i0 = x >> 16;
            let j0 = y >> 16;
            let s1 = (x & 0xffff) as i64;
            let t1 = (y & 0xffff) as i64;
            let s0 = Q as i64 - s1;
            let t0 = Q as i64 - t1;
            let a = d0[ix(i0, j0)] as i64 * t0 + d0[ix(i0, j0 + 1)] as i64 * t1;
            let b = d0[ix(i0 + 1, j0)] as i64 * t0 + d0[ix(i0 + 1, j0 + 1)] as i64 * t1;
            d[ix(i, j)] = ((a * s0 + b * s1) >> 32) as i32;
        }
    }
    set_bnd(d, which);
}

fn project(u: &mut [i32], v: &mut [i32], p: &mut [i32], div: &mut [i32]) {
    for j in 1..=H {
        for i in 1..=W {
            div[ix(i, j)] = -(u[ix(i + 1, j)] - u[ix(i - 1, j)] + v[ix(i, j + 1)] - v[ix(i, j - 1)]) / 2;
            p[ix(i, j)] = 0;
        }
    }
    set_bnd(div, 0);
    set_bnd(p, 0);
    lin_solve(p, div, Q, 4 * Q, 0);
    for j in 1..=H {
        for i in 1..=W {
            u[ix(i, j)] -= (p[ix(i + 1, j)] - p[ix(i - 1, j)]) / 2;
            v[ix(i, j)] -= (p[ix(i, j + 1)] - p[ix(i, j - 1)]) / 2;
        }
    }
    set_bnd(u, 1);
    set_bnd(v, 2);
}

fn fade(x: &mut [i32], per_ms: i32, dt_ms: u32) {
    let k = min(Q, (per_ms as u32).wrapping_mul(dt_ms) as i32);
    for e in x.iter_mut() {
        *e -= mul(*e, k);
    }
}

impl Flow {
    fn rnd(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn cell_x(&self, px: i32) -> i32 {
        ((((px as i64) << 24) + (self.cw / 2) as i64) / self.cw as i64) as i32 + Q
    }
    fn cell_y(&self, py: i32) -> i32 {
        ((((py as i64) << 24) + (self.ch / 2) as i64) / self.ch as i64) as i32 + Q
    }
    fn cells_x(&self, vx: i32) -> i32 {
        (((vx as i64) << 24) / self.cw as i64) as i32
    }
    fn cells_y(&self, vy: i32) -> i32 {
        (((vy as i64) << 24) / self.ch as i64) as i32
    }

    /// The screen w x h, Allen at (cx, cy) with radius r, the bar's mouth at (bx, by).
    pub fn new(w: i32, h: i32, cx: i32, cy: i32, r: i32, bx: i32, by: i32, t: u32) -> Flow {
        let mut weight = [0u32; 5];
        weight[SLEEP as usize] = 256;
        Flow {
            w,
            h,
            cw: max(256, w * 256 / W),
            ch: max(256, h * 256 / H),
            cx,
            cy,
            r,
            bx,
            by,
            t,
            stepped: t,
            mode: SLEEP,
            weight,
            think_t: 0,
            word_t: 0,
            key_t: 0,
            rng: 0x9e3779b9 ^ t,
            u: vec![0; CELLS],
            v: vec![0; CELLS],
            d: vec![0; CELLS],
            u0: vec![0; CELLS],
            v0: vec![0; CELLS],
            d0: vec![0; CELLS],
            shown: vec![0; CELLS],
            warm: vec![0; CELLS],
            shown_warm: vec![0; CELLS],
            steps: 0,
        }
    }

    pub fn set_mode(&mut self, mode: i32, t: u32) {
        if mode < 0 || mode > SLEEP || mode == self.mode {
            return;
        }
        self.mode = mode;
        self.think_t = if mode == THINK { if t != 0 { t } else { 1 } } else { 0 };
    }

    /// A push over a soft disc: velocity (vx, vy) and ink, both in grid units, strongest at the centre.
    fn push(&mut self, gx: i32, gy: i32, vx: i32, vy: i32, ink: i32, r: i32) {
        let i0 = max(1, (gx - r) >> 16);
        let i1 = min(W, (gx + r) >> 16);
        let j0 = max(1, (gy - r) >> 16);
        let j1 = min(H, (gy + r) >> 16);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let dx = ((i as i64) << 16) + HALF as i64 - gx as i64;
                let dy = ((j as i64) << 16) + HALF as i64 - gy as i64;
                let d2 = (dx * dx + dy * dy) >> 16;
                let r2 = (r as i64 * r as i64) >> 16;
                if d2 >= r2 {
                    continue;
                }
                let w = exp_neg((d2 * 3 * 256 / r2) as u32) as i64;
                let k = ix(i, j);
                self.u[k] += ((vx as i64 * w) >> 16) as i32;
                self.v[k] += ((vy as i64 * w) >> 16) as i32;
                self.d[k] = min(Q, self.d[k] + ((ink as i64 * w) >> 16) as i32);
            }
        }
    }

    fn forces(&mut self, dt_ms: u32) {
        let cx = self.cell_x(self.cx);
        let cy = self.cell_y(self.cy);
        let rr = max(Q / 2, (((self.r as i64) << 24) / self.cw as i64) as i32);
        let bx = self.cell_x(self.bx);
        let by = self.cell_y(self.by);
        let (wi, wl, wt, ws) = (self.weight[IDLE as usize], self.weight[LISTEN as usize], self.weight[THINK as usize], self.weight[SPEAK as usize]);
        let deep: u32 = if self.think_t != 0 { 256 + smooth(prog(self.t, self.think_t, self.think_t + 4000)) * 3 / 4 } else { 256 };
        let breath = sin(self.t.wrapping_mul(65536) / 7000);
        for j in 1..=H {
            for i in 1..=W {
                let dx = ((i as i64) << 16) + HALF as i64 - cx as i64;
                let dy = ((j as i64) << 16) + HALF as i64 - cy as i64;
                let mut r = (isqrt(((dx * dx + dy * dy) >> 16) as u64) << 8) as i64;
                if r < (Q / 8) as i64 {
                    r = (Q / 8) as i64;
                }
                let tx = (-dy * Q as i64 / r) as i32;
                let ty = (dx * Q as i64 / r) as i32;
                let rx = (dx * Q as i64 / r) as i32;
                let ry = (dy * Q as i64 / r) as i32;
                let ring = r - (3 * rr / 2) as i64;
                let bell: u32 = if r < (7 * rr / 2) as i64 { exp_neg((ring * ring * 512 / (rr as i64 * rr as i64)) as u32) } else { 0 };
                let mut ax: i64 = 0;
                let mut ay: i64 = 0;
                if wi != 0 && bell != 0 {
                    let k = (bell as i64 * wi as i64) >> 8;
                    let turn: i64 = (Q / 5 + ((breath * (Q / 8)) >> 14)) as i64;
                    ax += ((tx as i64 * turn >> 16) * k) >> 16;
                    ay += ((ty as i64 * turn >> 16) * k) >> 16;
                    ax += ((rx as i64 * ((breath * (Q / 10)) >> 14) as i64 >> 16) * k) >> 16;
                    ay += ((ry as i64 * ((breath * (Q / 10)) >> 14) as i64 >> 16) * k) >> 16;
                }
                if wt != 0 && bell != 0 {
                    let k = (bell as i64 * wt as i64 * deep as i64) >> 16;
                    let turn: i64 = (3 * Q / 2) as i64;
                    ax += ((tx as i64 * turn >> 16) * k) >> 16;
                    ay += ((ty as i64 * turn >> 16) * k) >> 16;
                    ax -= ((rx as i64 * (Q / 4) as i64 >> 16) * k) >> 16;
                    ay -= ((ry as i64 * (Q / 4) as i64 >> 16) * k) >> 16;
                }
                if wl != 0 {
                    let lx = (cx - bx) as i64;
                    let ly = (cy - by) as i64;
                    let len = (isqrt(((lx * lx + ly * ly) >> 16) as u64) << 8) as i64;
                    if len > Q as i64 {
                        let px = ((i as i64) << 16) + HALF as i64 - bx as i64;
                        let py = ((j as i64) << 16) + HALF as i64 - by as i64;
                        let along = (px * lx + py * ly) / len;
                        let across = (px * ly - py * lx) / len;
                        if along > 0 && along < len {
                            let near = exp_neg((across * across * 256 / (rr as i64 * rr as i64)) as u32);
                            let k = (near as i64 * wl as i64) >> 8;
                            ax += (((lx * Q as i64 / len) * (Q / 2) as i64 >> 16) * k) >> 16;
                            ay += (((ly * Q as i64 / len) * (Q / 2) as i64 >> 16) * k) >> 16;
                        }
                    }
                }
                let kk = ix(i, j);
                self.u[kk] += (ax * dt_ms as i64 / 1000) as i32;
                self.v[kk] += (ay * dt_ms as i64 / 1000) as i32;
            }
        }
        let ink = ((((wi * 6 + wl * 10 + ws * 60) as i64) * dt_ms as i64) >> 8) as i32;
        if ink != 0 {
            self.push(cx, cy, 0, 0, ink, rr);
        }
        let think_ink = ((wt as i64 * 120 * deep as i64 / 256 * dt_ms as i64) >> 8) as i32;
        if think_ink != 0 {
            for k in 0..8u32 {
                let a = (self.t.wrapping_mul(65536) / 9000).wrapping_add(k * 8192);
                self.push(
                    cx + ((cos(a) as i64 * 3 * rr as i64 / 2) >> 14) as i32,
                    cy + ((sin(a) as i64 * 3 * rr as i64 / 2) >> 14) as i32,
                    0,
                    0,
                    think_ink,
                    rr / 2,
                );
            }
        }
        if (wi != 0 || wl != 0) && (self.rnd() & 15) == 0 {
            // The two random draws sit in one argument list in the C code (order not defined there).
            let ry = self.rnd();
            let rx = self.rnd();
            let gx = Q + (rx % (W as u32 * Q as u32)) as i32;
            let gy = Q + (ry % (H as u32 * Q as u32)) as i32;
            self.push(gx, gy, 0, 0, Q / 40, rr / 2);
        }
    }

    fn step(&mut self, dt_ms: u32) {
        let dt = ((dt_ms << 16) / 1000) as i32;
        for m in 0..5usize {
            let want: u32 = if m as i32 == self.mode { 256 } else { 0 };
            let cur = self.weight[m];
            let mut stepw = 256 * dt_ms / BLEND_MS;
            if stepw == 0 {
                stepw = 1;
            }
            self.weight[m] = if cur < want { min(want as i32, (cur + stepw) as i32) } else { max(want as i32, cur as i32 - stepw as i32) } as u32;
        }
        self.forces(dt_ms);
        self.u0.copy_from_slice(&self.u);
        self.v0.copy_from_slice(&self.v);
        diffuse(&mut self.u, &self.u0, VISC_PER_MS * dt_ms as i32, 1);
        diffuse(&mut self.v, &self.v0, VISC_PER_MS * dt_ms as i32, 2);
        project(&mut self.u, &mut self.v, &mut self.u0, &mut self.v0);
        self.u0.copy_from_slice(&self.u);
        self.v0.copy_from_slice(&self.v);
        advect(&mut self.u, &self.u0, &self.u0, &self.v0, dt, 1);
        advect(&mut self.v, &self.v0, &self.u0, &self.v0, dt, 2);
        project(&mut self.u, &mut self.v, &mut self.u0, &mut self.v0);
        self.d0.copy_from_slice(&self.d);
        diffuse(&mut self.d, &self.d0, INK_PER_MS * dt_ms as i32, 0);
        self.d0.copy_from_slice(&self.d);
        advect(&mut self.d, &self.d0, &self.u, &self.v, dt, 0);
        let still = self.weight[SLEEP as usize];
        fade(&mut self.d, INK_FADE_PER_MS + ((INK_FADE_PER_MS as u32 * 3 * still) >> 8) as i32, dt_ms);
        fade(&mut self.u, VEL_FADE_PER_MS, dt_ms);
        fade(&mut self.v, VEL_FADE_PER_MS, dt_ms);
        for i in 0..CELLS {
            let s2 = ((self.u[i] as i64 * self.u[i] as i64) + (self.v[i] as i64 * self.v[i] as i64)) >> 16;
            let s = (isqrt(s2 as u64) as u32) << 8;
            let warm: u32 = if s >= 2 * Q as u32 { 256 } else { s * 256 / (2 * Q as u32) };
            self.warm[i] = (self.warm[i] as i32 + ((warm as i32 - self.warm[i] as i32) * dt_ms as i32 / 250)) as u16;
        }
        self.steps += 1;
    }

    /// A key was typed: a drop at the bar, lifted toward Allen.
    pub fn key(&mut self, t: u32) {
        self.key_t = t;
        let gx = self.cell_x(self.bx);
        let gy = self.cell_y(self.by);
        let lx = self.cell_x(self.cx) - gx;
        let ly = self.cell_y(self.cy) - gy;
        let mut len = (isqrt(((lx as i64 * lx as i64 + ly as i64 * ly as i64) >> 16) as u64) << 8) as i64;
        if len < Q as i64 {
            len = Q as i64;
        }
        let speed = (5 * Q / 2) as i64;
        self.push(gx, gy, (lx as i64 * speed / len) as i32, (ly as i64 * speed / len) as i32, Q / 2, Q);
    }

    /// A word was spoken: a ring from the heart.
    pub fn word(&mut self, t: u32) {
        self.word_t = t;
        let cx = self.cell_x(self.cx);
        let cy = self.cell_y(self.cy);
        let rr = max(Q / 2, (((self.r as i64) << 24) / self.cw as i64) as i32);
        for k in 0..12u32 {
            let a = k * 65536 / 12 + (self.rnd() & 2047);
            let ux = cos(a) << 2;
            let uy = sin(a) << 2;
            self.push(
                cx + ((ux as i64 * rr as i64 / 2) >> 16) as i32,
                cy + ((uy as i64 * rr as i64 / 2) >> 16) as i32,
                ((ux as i64 * (3 * Q / 2) as i64) >> 16) as i32,
                ((uy as i64 * (3 * Q / 2) as i64) >> 16) as i32,
                Q / 6,
                rr / 2,
            );
        }
    }

    /// A push at (px, py): velocity in px per second and ink (0..65536) over radius r px.
    pub fn touch(&mut self, px: i32, py: i32, vx: i32, vy: i32, ink: i32, r: i32) {
        let (gx, gy) = (self.cell_x(px), self.cell_y(py));
        let (gvx, gvy) = (self.cells_x(vx), self.cells_y(vy));
        let gr = max(Q / 4, (((r as i64) << 24) / self.cw as i64) as i32);
        self.push(gx, gy, gvx, gvy, ink, gr);
    }

    /// Moves the water to time t (ms); the solver steps every STEP_MS.
    pub fn at(&mut self, t: u32) {
        self.t = t;
        if t.wrapping_sub(self.stepped) > 8 * STEP_MS {
            self.stepped = t.wrapping_sub(8 * STEP_MS);
        }
        while t.wrapping_sub(self.stepped) >= STEP_MS {
            self.stepped = self.stepped.wrapping_add(STEP_MS);
            self.step(STEP_MS);
        }
    }

    /// The water's velocity at a point, px per second.
    pub fn velocity(&self, px: i32, py: i32) -> (i32, i32) {
        let gx = self.cell_x(px);
        let gy = self.cell_y(py);
        ((((sample(&self.u, gx, gy) as i64) * self.cw as i64) >> 24) as i32, (((sample(&self.v, gx, gy) as i64) * self.ch as i64) >> 24) as i32)
    }

    /// Calls mark(rect) for every cell whose ink changed enough to redraw since the last call.
    pub fn mark(&mut self, mut mark: impl FnMut(Rect)) {
        for j in 0..=H + 1 {
            for i in 0..=W + 1 {
                let k = ix(i, j);
                let now = self.d[k];
                let was = self.shown[k];
                let warm = self.warm[k] as i32;
                let warm_was = self.shown_warm[k] as i32;
                if abs(now - was) < Q / 400 && (now < Q / 200 || abs(warm - warm_was) < 24) {
                    continue;
                }
                self.shown[k] = now;
                self.shown_warm[k] = warm as u16;
                let r = rect(
                    (((i - 2) as i64 * self.cw as i64) >> 8) as i32,
                    (((j - 2) as i64 * self.ch as i64) >> 8) as i32,
                    ((3 * self.cw as i64 >> 8) + 2) as i32,
                    ((3 * self.ch as i64 >> 8) + 2) as i32,
                );
                mark(r);
            }
        }
    }

    /// The ink into the part r of out (light added). gain scales the brightness (256 = as is).
    pub fn draw(&self, out: &mut Canvas, r: &Rect, gain: u32) {
        let mut c = *r;
        let all = rect(0, 0, out.w, out.h);
        if !clip(&mut c, &all) {
            return;
        }
        let cool = rgb(64, 86, 196);
        let warm = rgb(255, 228, 196);
        let dgx = (1i64 << 24) / self.cw as i64;
        let dgy = (1i64 << 24) / self.ch as i64;
        let mut gy = self.cell_y(c.y) as i64;
        for y in c.y..c.y + c.h {
            let row = y as usize * out.w as usize;
            let mut gx = self.cell_x(c.x) as i64;
            for x in c.x..c.x + c.w {
                let ink = sample(&self.d, gx as i32, gy as i32);
                if ink >= Q / 512 {
                    let mut a = ((ink as i64 * gain as i64) >> 16) as u32;
                    a = if a > 230 { 230 } else { a };
                    let gxx = clamp(gx as i32 - HALF, 0, (W + 1) * Q - 1);
                    let gyy = clamp(gy as i32 - HALF, 0, (H + 1) * Q - 1);
                    let w = self.warm[ix(gxx >> 16, gyy >> 16)] as u32;
                    let col = mix(cool, warm, w);
                    out.px[row + x as usize] = add(out.px[row + x as usize], col, a);
                }
                gx += dgx;
            }
            gy += dgy;
        }
    }

    /// Total ink in the tank, 16.16.
    pub fn ink(&self) -> i64 {
        self.d.iter().map(|&v| v as i64).sum()
    }
}
