//! Allen's stroke typeface, ported from native/kernel/ui/allen_font.c of the AIENOS kernel.
//! Every printable ASCII character is round-capped pen strokes (lines and elliptical arcs) on a
//! design grid, rasterized on demand into anti-aliased masks. Integer math only.

pub const FIRST: u8 = 32;
pub const LAST: u8 = 126;
pub const COUNT: usize = (LAST - FIRST) as usize + 1;

/// Each glyph: "wN" (advance in design units), then pen commands:
/// Mx,y move; Lx,y line; Acx,cy,rx,ry,a0,a1 elliptical arc (degrees, 0 right, 90 down); Dx,y dot.
static GLYPHS: [&str; COUNT] = [
    "w6",
    "w6 M3,0 L3,14 D3,20",
    "w10 M3,0 L3,5 M7,0 L7,5",
    "w15 M6,2 L4,20 M11,2 L9,20 M2,8 L13,8 M1,14 L12,14",
    "w15 A8,5,5,5,-30,-270 A8,15,5,5,270,510 M8,-2 L8,22",
    "w16 A4,4,2,3,0,360 A12,16,2,3,0,360 M13,1 L3,19",
    "w16 M14,20 L5,9 A7,5,3,4,150,400 L3,14 A7,15,4,5,180,60 L13,11",
    "w6 M3,0 L3,5",
    "w9 A10,10,6,12,240,120",
    "w9 A-1,10,6,12,-60,60",
    "w14 M7,2 L7,10 M3,4 L11,8 M11,4 L3,8",
    "w14 M2,12 L12,12 M7,7 L7,17",
    "w6 M3,19 L2,23",
    "w11 M2,13 L9,13",
    "w6 D3,20",
    "w12 M10,0 L2,20",
    "w14 A7,10,5,10,0,360",
    "w14 M3,4 L8,0 L8,20",
    "w14 A7,6,5,6,200,400 L2,20 L12,20",
    "w14 A7,5,5,5,200,450 A7,15,5,5,270,520",
    "w14 M10,20 L10,0 L1,14 L13,14",
    "w14 M12,0 L4,0 L4,9 A7,14,5,6,235,520",
    "w14 A7,14,5,6,0,360 M10,0 L3,11",
    "w14 M2,0 L12,0 L5,20",
    "w14 A7,5,4,5,0,360 A7,15,5,5,0,360",
    "w14 A7,6,5,6,0,360 M4,20 L11,9",
    "w6 D3,8 D3,20",
    "w6 D3,8 M3,19 L2,23",
    "w14 M12,6 L2,12 L12,18",
    "w14 M2,10 L12,10 M2,15 L12,15",
    "w14 M2,6 L12,12 L2,18",
    "w14 A7,5,5,5,190,450 L7,14 D7,20",
    "w19 A9,12,3,4,0,360 M12,8 L12,14 A14,14,2,2,180,0 A9,11,7,9,0,-300",
    "w16 M2,20 L8,0 L14,20 M4,13 L12,13",
    "w15 M2,0 L2,20 M2,0 L8,0 A8,5,4,5,270,450 L2,10 M2,10 L9,10 A9,15,4,5,270,450 L2,20",
    "w17 A9,10,7,10,45,315",
    "w16 M2,0 L2,20 M2,0 L6,0 A6,10,8,10,270,450 L2,20",
    "w14 M12,0 L2,0 L2,20 L12,20 M2,10 L10,10",
    "w13 M12,0 L2,0 L2,20 M2,10 L10,10",
    "w18 A9,10,7,10,-45,-360 M11,10 L16,10",
    "w16 M2,0 L2,20 M14,0 L14,20 M2,10 L14,10",
    "w6 M3,0 L3,20",
    "w13 M10,0 L10,14 A6,14,4,6,0,180",
    "w15 M2,0 L2,20 M13,0 L2,12 M6,8 L14,20",
    "w13 M2,0 L2,20 L12,20",
    "w20 M2,20 L2,0 L10,14 L18,0 L18,20",
    "w16 M2,20 L2,0 L14,20 L14,0",
    "w20 A10,10,8,10,0,360",
    "w15 M2,20 L2,0 L8,0 A8,5,5,5,270,450 L2,10",
    "w20 A10,10,8,10,0,360 M12,14 L18,21",
    "w15 M2,20 L2,0 L8,0 A8,5,5,5,270,450 L2,10 M8,10 L14,20",
    "w16 A8,5,5,5,-30,-270 A8,15,5,5,270,510",
    "w16 M1,0 L15,0 M8,0 L8,20",
    "w16 M2,0 L2,13 A8,13,6,7,180,0 L14,0",
    "w16 M1,0 L8,20 L15,0",
    "w22 M1,0 L6,20 L11,0 L16,20 L21,0",
    "w16 M2,0 L14,20 M14,0 L2,20",
    "w16 M1,0 L8,10 L15,0 M8,10 L8,20",
    "w16 M2,0 L14,0 L2,20 L14,20",
    "w9 M7,-1 L3,-1 L3,21 L7,21",
    "w12 M2,0 L10,20",
    "w9 M2,-1 L6,-1 L6,21 L2,21",
    "w12 M2,8 L6,1 L10,8",
    "w14 M1,23 L13,23",
    "w7 M2,0 L4,4",
    "w15 A7,14,5,6,0,360 M12,8 L12,20",
    "w15 M2,0 L2,20 A7,14,5,6,0,360",
    "w13 A7,14,5,6,40,320",
    "w15 A7,14,5,6,0,360 M12,0 L12,20",
    "w14 M2,14 L12,14 A7,14,5,6,0,-320",
    "w11 M5,20 L5,4 A9,4,4,4,180,300 M2,8 L10,8",
    "w15 A7,14,5,6,0,360 M12,8 L12,22 A7,22,5,5,0,150",
    "w15 M2,0 L2,20 A7,13,5,5,180,360 L12,20",
    "w6 M3,8 L3,20 D3,3",
    "w7 M4,8 L4,22 A0,22,4,5,0,120 D4,3",
    "w13 M2,0 L2,20 M11,8 L2,15 M5,13 L12,20",
    "w6 M3,0 L3,20",
    "w21 M2,8 L2,20 A6,12,4,4,180,360 L10,20 A14,12,4,4,180,360 L18,20",
    "w15 M2,8 L2,20 A7,13,5,5,180,360 L12,20",
    "w14 A7,14,5,6,0,360",
    "w15 M2,8 L2,27 A7,14,5,6,0,360",
    "w15 A7,14,5,6,0,360 M12,8 L12,27",
    "w10 M2,8 L2,20 A7,13,5,5,180,290",
    "w14 A7,11,4,3,-30,-270 A7,17,4,3,270,510",
    "w11 M5,2 L5,17 A8,17,3,3,180,40 M2,8 L10,8",
    "w15 M2,8 L2,15 A7,15,5,5,180,0 M12,8 L12,20",
    "w14 M2,8 L7,20 L12,8",
    "w20 M2,8 L6,20 L10,8 L14,20 L18,8",
    "w14 M2,8 L12,20 M12,8 L2,20",
    "w14 M2,8 L7,20 M12,8 L4,27",
    "w14 M2,8 L12,8 L2,20 L12,20",
    "w10 M8,-1 L6,0 L6,8 L4,10 L6,12 L6,20 L8,21",
    "w6 M3,-1 L3,23",
    "w10 M2,-1 L4,0 L4,8 L6,10 L4,12 L4,20 L2,21",
    "w14 M2,13 L5,11 L9,14 L12,12",
];

static SIN_Q: [u16; 91] = [
    0, 286, 572, 857, 1143, 1428, 1713, 1997, 2280, 2563, 2845, 3126, 3406, 3686, 3964, 4240, 4516, 4790,
    5063, 5334, 5604, 5872, 6138, 6402, 6664, 6924, 7182, 7438, 7692, 7943, 8192, 8438, 8682, 8923, 9162,
    9397, 9630, 9860, 10087, 10311, 10531, 10749, 10963, 11174, 11381, 11585, 11786, 11982, 12176, 12365,
    12551, 12733, 12911, 13085, 13255, 13421, 13583, 13741, 13894, 14044, 14189, 14330, 14466, 14598,
    14726, 14849, 14968, 15082, 15191, 15296, 15396, 15491, 15582, 15668, 15749, 15826, 15897, 15964,
    16026, 16083, 16135, 16182, 16225, 16262, 16294, 16322, 16344, 16362, 16374, 16382, 16384,
];

/// sin of whole degrees (any sign), scaled by 16384.
pub fn af_sin(deg: i32) -> i32 {
    let mut deg = deg % 360;
    if deg < 0 {
        deg += 360;
    }
    let d = deg as usize;
    if d <= 90 {
        SIN_Q[d] as i32
    } else if d <= 180 {
        SIN_Q[180 - d] as i32
    } else if d <= 270 {
        -(SIN_Q[d - 180] as i32)
    } else {
        -(SIN_Q[360 - d] as i32)
    }
}

pub fn af_cos(deg: i32) -> i32 {
    af_sin(deg + 90)
}

/// Integer square root (floor).
pub fn isqrt(v: u64) -> u64 {
    let mut r = (v as f64).sqrt() as u64;
    while (r as u128) * (r as u128) > v as u128 {
        r -= 1;
    }
    while ((r + 1) as u128) * ((r + 1) as u128) <= v as u128 {
        r += 1;
    }
    r
}

fn idx(c: u8) -> usize {
    if (FIRST..=LAST).contains(&c) {
        (c - FIRST) as usize
    } else {
        (b'?' - FIRST) as usize
    }
}

#[derive(Clone, Default)]
pub struct Glyph {
    /// w*h coverage 0..255; empty when the glyph has no ink
    pub alpha: Vec<u8>,
    pub w: u16,
    pub h: u16,
    /// mask top-left relative to the pen on the baseline, px
    pub ox: i16,
    pub oy: i16,
    done: bool,
}

pub struct Face {
    /// 1/256 px per design unit
    pub s256: u32,
    /// half the stroke width, 1/256 px
    pub hw256: u32,
    g: Vec<Glyph>,
}

struct Seg {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

/// Reads a signed integer at the cursor; advances past it and one ',' if present.
fn num(b: &[u8], p: &mut usize) -> i32 {
    let mut neg = false;
    let mut v: i32 = 0;
    if *p < b.len() && b[*p] == b'-' {
        neg = true;
        *p += 1;
    }
    while *p < b.len() && b[*p].is_ascii_digit() {
        v = v * 10 + (b[*p] - b'0') as i32;
        *p += 1;
    }
    if *p < b.len() && b[*p] == b',' {
        *p += 1;
    }
    if neg {
        -v
    } else {
        v
    }
}

const SEG_MAX: usize = 160;

impl Face {
    /// cap_px: cap height in pixels; weight: half the stroke width in thousandths of a design unit.
    pub fn new(cap_px: u32, weight: u32) -> Face {
        let s256 = cap_px * 256 / 20;
        let hw256 = (weight as u64 * s256 as u64 / 1000) as u32;
        Face { s256, hw256, g: vec![Glyph::default(); COUNT] }
    }

    /// Advance after c, 1/256 px.
    pub fn advance(&self, c: u8) -> u32 {
        let b = GLYPHS[idx(c)].as_bytes();
        let mut p = 1;
        num(b, &mut p) as u32 * self.s256
    }

    /// Width of the first n characters of s, whole px (rounded up).
    pub fn measure(&self, s: &[u8], n: usize) -> u32 {
        let mut w: u64 = 0;
        for &c in s.iter().take(n) {
            if c == 0 {
                break;
            }
            w += self.advance(c) as u64;
        }
        ((w + 255) / 256) as u32
    }

    fn segments(&self, glyph: &str) -> Vec<Seg> {
        let s = self.s256 as i32;
        let b = glyph.as_bytes();
        let mut out: Vec<Seg> = Vec::new();
        let (mut px, mut py) = (0i32, 0i32);
        let pt_x = |u: i32| u * s;
        let pt_y = |u: i32| (u - 20) * s;
        let mut p = 0;
        while p < b.len() {
            let cmd = b[p];
            p += 1;
            match cmd {
                b' ' => {}
                b'w' => {
                    num(b, &mut p);
                }
                b'M' => {
                    let x = num(b, &mut p);
                    let y = num(b, &mut p);
                    px = pt_x(x);
                    py = pt_y(y);
                }
                b'L' | b'D' => {
                    let x = num(b, &mut p);
                    let y = num(b, &mut p);
                    let (nx, ny) = (pt_x(x), pt_y(y));
                    if cmd == b'D' {
                        px = nx;
                        py = ny;
                    }
                    if out.len() < SEG_MAX {
                        out.push(Seg { x0: px, y0: py, x1: nx, y1: ny });
                    }
                    px = nx;
                    py = ny;
                }
                b'A' => {
                    let cx = num(b, &mut p);
                    let cy = num(b, &mut p);
                    let rx = num(b, &mut p);
                    let ry = num(b, &mut p);
                    let a0 = num(b, &mut p);
                    let a1 = num(b, &mut p);
                    let span = if a1 > a0 { a1 - a0 } else { a0 - a1 };
                    let steps = if span / 9 < 2 { 2 } else { span / 9 };
                    let (mut lx, mut ly) = (0i32, 0i32);
                    for k in 0..=steps {
                        let a = a0 + (a1 - a0) * k / steps;
                        let x = pt_x(cx) + ((rx as i64 * s as i64 * af_cos(a) as i64) / 16384) as i32;
                        let y = pt_y(cy) + ((ry as i64 * s as i64 * af_sin(a) as i64) / 16384) as i32;
                        if k != 0 && out.len() < SEG_MAX {
                            out.push(Seg { x0: lx, y0: ly, x1: x, y1: y });
                        }
                        lx = x;
                        ly = y;
                    }
                    px = lx;
                    py = ly;
                }
                _ => {}
            }
        }
        out
    }

    /// The mask for c (anything outside 32..126 draws as '?'). Rasterized on first use.
    pub fn glyph(&mut self, c: u8) -> &Glyph {
        let i = idx(c);
        if self.g[i].done {
            return &self.g[i];
        }
        let sg = self.segments(GLYPHS[i]);
        let g = &mut self.g[i];
        g.done = true;
        if sg.is_empty() {
            return &self.g[i];
        }
        let (mut minx, mut maxx, mut miny, mut maxy) = (sg[0].x0, sg[0].x0, sg[0].y0, sg[0].y0);
        for s in &sg {
            for (x, y) in [(s.x0, s.y0), (s.x1, s.y1)] {
                minx = minx.min(x);
                maxx = maxx.max(x);
                miny = miny.min(y);
                maxy = maxy.max(y);
            }
        }
        let floor_px = |v: i32| if v >= 0 { v / 256 } else { -((-v + 255) / 256) };
        let pad = self.hw256 as i32 + 256;
        let x0 = floor_px(minx - pad);
        let y0 = floor_px(miny - pad);
        let x1 = floor_px(maxx + pad) + 1;
        let y1 = floor_px(maxy + pad) + 1;
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut a = vec![0u8; w * h];
        let hw = self.hw256 as i64;
        for yy in 0..h {
            let py = (y0 + yy as i32) as i64 * 256 + 128;
            for xx in 0..w {
                let pxc = (x0 + xx as i32) as i64 * 256 + 128;
                let mut best = i64::MAX;
                for s in &sg {
                    let (ax, ay) = (s.x0 as i64, s.y0 as i64);
                    let (dx, dy) = (s.x1 as i64 - ax, s.y1 as i64 - ay);
                    let (qx, qy) = (pxc - ax, py - ay);
                    let len2 = dx * dx + dy * dy;
                    let (ex, ey);
                    if len2 == 0 {
                        ex = qx;
                        ey = qy;
                    } else {
                        let t = qx * dx + qy * dy;
                        if t <= 0 {
                            ex = qx;
                            ey = qy;
                        } else if t >= len2 {
                            ex = qx - dx;
                            ey = qy - dy;
                        } else {
                            ex = qx - dx * t / len2;
                            ey = qy - dy * t / len2;
                        }
                    }
                    let d2 = ex * ex + ey * ey;
                    if d2 < best {
                        best = d2;
                    }
                }
                let d = isqrt(best as u64) as i64;
                let cov = hw + 128 - d;
                a[yy * w + xx] = if cov <= 0 { 0 } else if cov >= 256 { 255 } else { (cov * 255 / 256) as u8 };
            }
        }
        let g = &mut self.g[i];
        g.w = w as u16;
        g.h = h as u16;
        g.ox = x0 as i16;
        g.oy = y0 as i16;
        g.alpha = a;
        &self.g[i]
    }
}
