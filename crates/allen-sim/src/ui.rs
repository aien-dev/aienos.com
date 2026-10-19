//! Allen's home on the display, ported from native/kernel/ui/allen_ui.[ch]. The screen opens on
//! the night sky; two black holes merge; Allen gathers from the light; a quiet bar takes what the
//! guest types; answers appear word by word; goodbye is a dusk into a black hole.
//! Every pixel is worked out from its position and the time; a frame redraws only the tiles that
//! changed. Integer only.
use crate::flow::{self, Flow};
use crate::font::{isqrt, Face, FIRST, LAST};
use crate::mind::{self, Mind};
use crate::night::{self, Dusk, Night, Picture};
use crate::paint::*;

const TEXT_MAX: usize = 160;
const LINES_MAX: usize = 5;
const TW: i32 = 64;
const TH: i32 = 32;
const JOBS_MAX: usize = 1200;
const FRAME_MS: u32 = 16;

const BORN_MS: u32 = night::MERGE_MS + 300;
const FORMED_MS: u32 = BORN_MS + mind::FORM_MS;
const LETTER_MS: u32 = 35;
const BAR_REST_MS: u32 = 9000;
const DUSK_MS: u32 = 4200;
const DAWN_MS: u32 = 900;
const MAP_SHOW_MS: u32 = 9000;
const DIM_FULL: u32 = 256 << 8;

pub const MAP_NODES: usize = 48;
pub const MAP_LINKS: usize = 160;

const CAP_VOICE: u32 = 19;
const CAP_BODY: u32 = 15;
const CAP_SMALL: u32 = 11;

#[derive(Clone, Default)]
struct Words {
    text: Vec<u8>,
    lines: usize,
    st: [usize; LINES_MAX],
    ln: [usize; LINES_MAX],
    x: [i32; LINES_MAX],
    from: u32,
    done: u32,
    fade: u32,
    bx: Rect,
}

impl Words {
    fn len(&self) -> usize {
        self.text.len()
    }
}

/// what the keyboard side has left for the next frame
#[derive(Default)]
struct Note {
    refresh: bool,
    input_gen: u32,
    said_gen: u32,
    map_gen: u32,
    wake_gen: u32,
    sleep: bool,
    sleep_gen: u32,
    input: Vec<u8>,
    line: Vec<u8>,
    reply: Vec<u8>,
    has_line: bool,
    think: bool,
    footer: Option<Vec<u8>>,
    n: usize,
    total: u32,
    links: u32,
    lit: i32,
    ea: Vec<u8>,
    eb: Vec<u8>,
    label: Vec<u8>,
}

/// at most max-1 bytes of s, stopping at a NUL
fn copy(s: &[u8], max: usize) -> Vec<u8> {
    s.iter().copied().take_while(|&c| c != 0).take(max - 1).collect()
}

pub struct Ui {
    note: Note,
    pub w: i32,
    pub h: i32,
    k: u32,
    /// where a frame is put together (0x00RRGGBB)
    pub back: Canvas,
    /// the screen: one u32 per pixel, bytes R G B 0xFF in memory order
    pub fb: Vec<u32>,
    night: Night,
    mind: Mind,
    flow: Flow,
    dim: u32,
    dusk_t: u32,
    kept: Option<Picture>,
    far16: i32,
    thinking: bool,
    voice: Face,
    body: Face,
    small: Face,
    t: u32,
    last: u32,
    cx: i32,
    cy: i32,
    on: bool,
    footer: Option<Vec<u8>>,
    bar: Rect,
    bar_box: Rect,
    bar_a: u32,
    bar_was: u32,
    wake_t: u32,
    quiet_t: u32,
    input: Vec<u8>,
    key_t: u32,
    seen_input: u32,
    seen_said: u32,
    seen_map: u32,
    seen_wake: u32,
    asleep: bool,
    seen_sleep: u32,
    input_moved: bool,
    now: Words,
    next: Words,
    speak_t: u32,
    guest: Vec<u8>,
    guest_x: i32,
    guest_t: u32,
    hint: Vec<u8>,
    shimmer: u32,
    shimmer_was: u32,
    level: u32,
    word_t: u32,
    guest_box: Rect,
    words_was: Rect,
    n: usize,
    ne: usize,
    total: u32,
    links: u32,
    lit: i32,
    ea: Vec<u8>,
    eb: Vec<u8>,
    label: Vec<u8>,
    mx: i32,
    my: i32,
    mr: i32,
    map_t: u32,
    map_a: u32,
    map_was: u32,
    map_box: Rect,
    tiles: Vec<u8>,
    tx: i32,
    ty: i32,
    jobs: Vec<Rect>,
    twinkle_turn: u32,
    flash_was: Rect,
    lens_was: Rect,
    ripple_was: i32,
    pub frames: u32,
}

fn scale_for(w: u32, h: u32) -> u32 {
    let kw = w * 256 / 1920;
    let kh = h * 256 / 1080;
    if kw < kh { kw } else { kh }
}

fn toward(cur: u32, want: u32, dt: u32, full_ms: u32) -> u32 {
    let mut step = (256u64 * dt as u64 / full_ms as u64) as u32;
    step = if step != 0 || dt == 0 { step } else { 1 };
    if cur < want {
        return if cur + step < want { cur + step } else { want };
    }
    if cur > want.wrapping_add(step) { cur - step } else { want }
}

fn mark_into(tiles: &mut [u8], w: i32, h: i32, tx: i32, mut r: Rect) {
    let all = rect(0, 0, w, h);
    if r.w <= 0 || r.h <= 0 || !clip(&mut r, &all) {
        return;
    }
    for ty in r.y / TH..=(r.y + r.h - 1) / TH {
        for x in r.x / TW..=(r.x + r.w - 1) / TW {
            tiles[(ty * tx + x) as usize] = 1;
        }
    }
}

impl Ui {
    fn s(&self, v: i64) -> i32 {
        ((v * self.k as i64 + 128) / 256) as i32
    }

    fn mark(&mut self, r: Rect) {
        mark_into(&mut self.tiles, self.w, self.h, self.tx, r);
    }

    fn mark_all(&mut self) {
        for t in self.tiles.iter_mut() {
            *t = 1;
        }
    }

    /// Opens Allen's window on a w x h screen; the night starts at time 0.
    pub fn new(w: u32, h: u32) -> Option<Ui> {
        if w < 640 || h < 400 || ((w + TW as u32 - 1) / TW as u32) * ((h + TH as u32 - 1) / TH as u32) > 16384 {
            return None;
        }
        let k = scale_for(w, h);
        let (wi, hi) = (w as i32, h as i32);
        let cx = wi / 2;
        let cy = hi * 40 / 100;
        let night = Night::new(w, h, k, cx, cy)?;
        let mut far16 = 0;
        for i in 0..4 {
            let dx = (if i & 1 != 0 { wi } else { 0 } - cx) as i64;
            let dy = (if i & 2 != 0 { hi } else { 0 } - cy) as i64;
            let d = isqrt((dx * dx + dy * dy) as u64) as i32 * 16;
            far16 = max(far16, d);
        }
        let mut voice = Face::new(CAP_VOICE * k / 256, 1000);
        let mut body = Face::new(CAP_BODY * k / 256, 1150);
        let mut small = Face::new(CAP_SMALL * k / 256, 1250);
        for f in [&mut voice, &mut body, &mut small] {
            for c in FIRST..=LAST {
                f.glyph(c);
            }
        }
        let mind = Mind::new(cx, cy, k, BORN_MS);
        let mut ui = Ui {
            note: Note::default(),
            w: wi,
            h: hi,
            k,
            back: Canvas::new(wi, hi),
            fb: vec![0xff000000; (w * h) as usize],
            night,
            mind,
            flow: Flow::new(wi, hi, cx, cy, 0, 0, 0, 0),
            dim: 0,
            dusk_t: 0,
            kept: None,
            far16,
            thinking: false,
            voice,
            body,
            small,
            t: 0,
            last: 0,
            cx,
            cy,
            on: false,
            footer: None,
            bar: Rect::default(),
            bar_box: Rect::default(),
            bar_a: 0,
            bar_was: 0,
            wake_t: 0,
            quiet_t: 0,
            input: Vec::new(),
            key_t: 0,
            seen_input: 0,
            seen_said: 0,
            seen_map: 0,
            seen_wake: 0,
            asleep: false,
            seen_sleep: 0,
            input_moved: false,
            now: Words::default(),
            next: Words::default(),
            speak_t: 0,
            guest: Vec::new(),
            guest_x: 0,
            guest_t: 0,
            hint: Vec::new(),
            shimmer: 0,
            shimmer_was: 0,
            level: 0,
            word_t: 0,
            guest_box: Rect::default(),
            words_was: Rect::default(),
            n: 0,
            ne: 0,
            total: 0,
            links: 0,
            lit: -1,
            ea: Vec::new(),
            eb: Vec::new(),
            label: Vec::new(),
            mx: 0,
            my: 0,
            mr: 0,
            map_t: 0,
            map_a: 0,
            map_was: 0,
            map_box: Rect::default(),
            tiles: Vec::new(),
            tx: ((w + TW as u32 - 1) / TW as u32) as i32,
            ty: ((h + TH as u32 - 1) / TH as u32) as i32,
            jobs: Vec::new(),
            twinkle_turn: 0,
            flash_was: Rect::default(),
            lens_was: Rect::default(),
            ripple_was: -1,
            frames: 0,
        };
        ui.tiles = vec![0; (ui.tx * ui.ty) as usize];
        ui.named("Allen");
        let bw = ui.s(720);
        let bh = ui.s(54);
        ui.bar = rect((ui.w - bw) / 2, ui.h - ui.s(150), bw, bh);
        ui.bar_box = rect(ui.bar.x - ui.s(20), ui.bar.y - ui.s(20), bw + ui.s(40), bh + ui.s(80));
        ui.bar_box.x = min(ui.bar_box.x, ui.w / 2 - ui.s(360));
        ui.bar_box.w = max(ui.bar_box.w, ui.s(720));
        ui.mx = ui.w * 84 / 100;
        ui.my = ui.h * 42 / 100;
        ui.mr = ui.s(130);
        ui.map_box = rect(ui.mx - ui.s(190), ui.my - ui.mr - ui.s(64), ui.s(380), 2 * ui.mr + ui.s(150));
        ui.flow = Flow::new(ui.w, ui.h, ui.cx, ui.cy, ui.mind.r, ui.bar.x + ui.bar.w / 2, ui.bar.y - ui.bar.h / 2, 0);
        ui.seen_map = ui.note.map_gen.wrapping_sub(1);
        Some(ui)
    }

    // ---- the keyboard side: notes left for the next frame ----

    pub fn refresh(&mut self) {
        self.note.refresh = true;
    }

    /// Allen sleeps (asleep = true): the dusk. false brings the sky back, calmly.
    pub fn sleep(&mut self, asleep: bool) {
        self.note.sleep = asleep;
        self.note.sleep_gen += 1;
    }

    pub fn asleep(&self) -> bool {
        self.asleep && self.dim == DIM_FULL
    }

    /// Allen is ready to talk: the bar comes up once he has formed. footer: one short line shown
    /// under the empty bar.
    pub fn wake(&mut self, footer: &str) {
        self.note.footer = Some(footer.as_bytes().to_vec());
        self.note.wake_gen += 1;
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The line being typed changed.
    pub fn input(&mut self, buf: &[u8]) {
        let len = buf.len().min(TEXT_MAX - 1);
        self.note.input = buf[..len].to_vec();
        self.note.input_gen += 1;
    }

    /// A line was entered and Allen answered. line None: Allen speaks first.
    pub fn said(&mut self, line: Option<&str>, reply: Option<&str>) {
        self.note.has_line = line.is_some();
        self.note.think = reply.is_none();
        self.note.line = copy(line.unwrap_or("").as_bytes(), TEXT_MAX);
        self.note.reply = copy(reply.unwrap_or("").as_bytes(), TEXT_MAX);
        self.note.said_gen += 1;
    }

    /// A line was entered and the answer is coming from further off.
    pub fn think(&mut self, line: &str) {
        self.said(Some(line), None);
    }

    /// The notes map: n stars, links ea[i]-eb[i], lit the star just saved (-1 none).
    pub fn map(&mut self, n: usize, ea: &[u8], eb: &[u8], lit: i32, label: &str, total: u32, links: u32) {
        self.note.n = n.min(MAP_NODES);
        self.note.ea.clear();
        self.note.eb.clear();
        for i in 0..ea.len().min(eb.len()) {
            if self.note.ea.len() >= MAP_LINKS {
                break;
            }
            if (ea[i] as usize) < self.note.n && (eb[i] as usize) < self.note.n && ea[i] != eb[i] {
                self.note.ea.push(ea[i]);
                self.note.eb.push(eb[i]);
            }
        }
        self.note.lit = if lit >= 0 && (lit as usize) < self.note.n { lit } else { -1 };
        self.note.label = copy(label.as_bytes(), TEXT_MAX);
        self.note.total = total;
        self.note.links = links;
        self.note.map_gen += 1;
    }

    /// The name Allen goes by, for the bar's "Speak to ..." hint.
    pub fn named(&mut self, name: &str) {
        let mut hint = b"Speak to ".to_vec();
        for &c in name.as_bytes() {
            if hint.len() + 1 >= 40 {
                break;
            }
            hint.push(c);
        }
        self.hint = hint;
        if !self.tiles.is_empty() {
            let b = self.bar;
            self.mark(b);
        }
    }

    // ---- text ----

    /// Word-wraps s to width px with face f: line starts and lengths.
    fn wrap(f: &Face, s: &[u8], width: i32) -> (usize, [usize; LINES_MAX], [usize; LINES_MAX]) {
        let n = s.len();
        let mut st = [0usize; LINES_MAX];
        let mut ln = [0usize; LINES_MAX];
        let mut lines = 0;
        let mut i = 0;
        while i < n && lines < LINES_MAX {
            while i < n && s[i] == b' ' {
                i += 1;
            }
            let start = i;
            let mut last_space = 0;
            let mut j = i;
            let mut w = 0i32;
            while j < n {
                let aw = ((f.advance(s[j]) + 255) / 256) as i32;
                if w + aw > width && j > start {
                    break;
                }
                if s[j] == b' ' {
                    last_space = j;
                }
                w += aw;
                j += 1;
            }
            if j < n && last_space > start {
                j = last_space;
            }
            st[lines] = start;
            ln[lines] = j - start;
            lines += 1;
            i = j;
        }
        (lines, st, ln)
    }

    fn line_h(&self) -> i32 {
        self.s(40)
    }

    fn words_top(&self) -> i32 {
        self.cy + self.s(250)
    }

    fn lay_out(&mut self, which_now: bool) {
        let width = self.s(900);
        let top = self.words_top();
        let lh = self.line_h();
        let (s8, s34, s16, s30) = (self.s(8), self.s(34), self.s(16), self.s(30));
        let ui_w = self.w;
        let mut w = if which_now { std::mem::take(&mut self.now) } else { std::mem::take(&mut self.next) };
        let (lines, st, ln) = Self::wrap(&self.voice, &w.text, width);
        w.lines = lines;
        w.st = st;
        w.ln = ln;
        let mut x0 = ui_w;
        let mut x1 = 0;
        for i in 0..w.lines {
            let lw = self.voice.measure(&w.text[w.st[i]..], w.ln[i]) as i32;
            w.x[i] = (ui_w - lw) / 2;
            x0 = min(x0, w.x[i]);
            x1 = max(x1, w.x[i] + lw);
        }
        w.bx = if w.lines != 0 { rect(x0 - s8, top - s34, x1 - x0 + s16, w.lines as i32 * lh + s30) } else { rect(0, 0, 0, 0) };
        let last = if w.lines != 0 { w.st[w.lines - 1] + w.ln[w.lines - 1] } else { 0 };
        w.done = w.from + last as u32 * LETTER_MS + 700;
        if which_now {
            self.now = w;
        } else {
            self.next = w;
        }
    }
}

/// how long words stay once said: time to read them
fn read_ms(w: &Words) -> u32 {
    6000 + w.len() as u32 * 70
}

fn words_alpha(w: &Words, t: u32) -> u32 {
    if w.len() == 0 {
        return 0;
    }
    let out = if w.fade != 0 { w.fade } else { w.done + read_ms(w) };
    256 - smooth(prog(t, out, out + 450))
}

/// The pen's end after drawing the n letters of s from x on baseline y; returns whole px.
fn text(cv: &mut Canvas, cl: &Rect, f: &mut Face, x: i32, y: i32, s: &[u8], color: u32, a: u32) -> i32 {
    let mut pen = x as u64 * 256;
    for &c in s {
        pen = letter(cv, cl, f, pen, y, c, color, a, false);
    }
    ((pen + 255) / 256) as i32
}

fn put_num(v: u32) -> Vec<u8> {
    v.to_string().into_bytes()
}

impl Ui {
    fn draw_words(&mut self, r: &Rect, t: u32) {
        let all = words_alpha(&self.now, t);
        if all == 0 || !meets(self.now.bx, r) {
            return;
        }
        let top = self.words_top();
        let lh = self.line_h();
        let s6 = self.s(6) as i64;
        for li in 0..self.now.lines {
            let mut pen = self.now.x[li] as u64 * 256;
            let y = top + li as i32 * lh;
            for j in 0..self.now.ln[li] {
                let i = self.now.st[li] + j;
                let at = self.now.from + i as u32 * LETTER_MS;
                if t <= at {
                    break;
                }
                let age = t - at;
                let a = smooth(prog(age, 0, 260));
                let c = mix(rgb(255, 212, 150), rgb(234, 238, 252), smooth(prog(age, 0, 900)));
                let rise = (s6 * (256 - ease_out(prog(age, 0, 360))) as i64 >> 8) as i32;
                let ch = self.now.text[i];
                pen = letter(&mut self.back, r, &mut self.voice, pen, y + rise, ch, c, a * all >> 8, false);
            }
        }
    }

    // ---- the bar ----

    fn draw_bar(&mut self, r: &Rect, t: u32) {
        let big_a = self.bar_a;
        if big_a == 0 || !meets(self.bar_box, r) {
            return;
        }
        let b = self.bar;
        let mut d = grow(b, self.s(14));
        if !clip(&mut d, r) {
            d.w = 0;
        }
        let rad = b.h / 2;
        let typing = !self.input.is_empty();
        let s14 = self.s(14);
        let bw = self.w as usize;
        let mut y = d.y;
        while d.w != 0 && y < d.y + d.h {
            for x in d.x..d.x + d.w {
                let sd = sd_rrect(x, y, &b, rad);
                let inn = cover(sd);
                let p = &mut self.back.px[y as usize * bw + x as usize];
                if inn != 0 {
                    *p = mix(*p, rgb(6, 8, 18), (inn * 150 >> 8) * big_a >> 8);
                }
                let edge = abs(sd + 8);
                if edge < 16 {
                    *p = add(*p, rgb(255, 255, 255), (16 - edge) as u32 * (if typing { 4 } else { 3 }) * big_a >> 8);
                }
                if typing && sd > 0 && sd < s14 * 16 {
                    let g = 256 - (sd * 256 / (s14 * 16)) as u32;
                    *p = add(*p, rgb(110, 130, 255), ((g * g >> 8) * 34 >> 8) * big_a >> 8);
                }
            }
            y += 1;
        }
        let pad = self.s(30);
        let base = b.y + b.h / 2 + self.s(7);
        let mut tc = rect(b.x + pad, b.y, b.w - 2 * pad, b.h);
        if !clip(&mut tc, r) {
            tc.w = 0;
        }
        let mut x = b.x + pad;
        if self.input.is_empty() {
            if tc.w != 0 {
                text(&mut self.back, &tc, &mut self.body, x, base, &self.hint, rgb(150, 160, 196), 150 * big_a >> 8);
            }
        } else {
            let avail = b.w - 2 * pad - self.s(12);
            let mut from = 0;
            while from < self.input.len() && self.body.measure(&self.input[from..], self.input.len() - from) as i32 > avail {
                from += 1;
            }
            if tc.w != 0 {
                x = text(&mut self.back, &tc, &mut self.body, x, base, &self.input[from..], rgb(236, 240, 252), big_a);
            } else {
                x += self.body.measure(&self.input[from..], self.input.len() - from) as i32;
            }
        }
        let mut ca = ((16384 + sin(t.wrapping_sub(self.key_t).wrapping_mul(65536) / 1100 + 16384)) as u32 * 200) >> 15;
        if t.wrapping_sub(self.key_t) < 500 {
            ca = 200;
        }
        let mut caret = rect(x + self.s(3), b.y + b.h / 2 - self.s(12), max(self.s(2), 2), self.s(24));
        if clip(&mut caret, r) {
            for yy in caret.y..caret.y + caret.h {
                for xx in caret.x..caret.x + caret.w {
                    let p = &mut self.back.px[yy as usize * bw + xx as usize];
                    *p = add(*p, rgb(150, 170, 255), (40 + ca) * big_a >> 8);
                }
            }
        }
        if let Some(footer) = &self.footer {
            if self.input.is_empty() {
                let fw = self.small.measure(footer, footer.len()) as i32;
                let fy = b.y + b.h + self.s(34);
                text(&mut self.back, r, &mut self.small, (self.w - fw) / 2, fy, footer, rgb(130, 140, 176), 110 * big_a >> 8);
            }
        }
    }

    // ---- the guest's line rising into Allen, and the thoughts it becomes ----

    fn curve(&self, i: u32, t: u32) -> (i32, i32, u32) {
        let start = self.guest_t + 150 + i * 40;
        let dur = 850 + hash(i as i32, 1, 5) % 300;
        let p = prog(t, start, start + dur);
        let mut a: u32 = if t < start || p >= 256 { 0 } else { 230 };
        let gw = self.body.measure(&self.guest, self.guest.len()) as i32;
        let x0 = (self.guest_x + gw * (2 * i as i32 + 1) / 18) * 16;
        let y0 = (self.bar.y + self.bar.h / 2 - self.s(60)) * 16;
        let x2 = self.mind.hx;
        let y2 = self.mind.hy;
        let side = if hash(i as i32, 2, 5) % 2 != 0 { 1 } else { -1 };
        let x1 = (x0 + x2) / 2 + side * self.s(160 + (hash(i as i32, 3, 5) % 140) as i64) * 16;
        let y1 = (y0 + y2) / 2;
        let e = smooth(p) as i64;
        let f = 256 - e;
        let x = ((x0 as i64 * f * f + 2 * x1 as i64 * f * e + x2 as i64 * e * e) >> 16) as i32;
        let y = ((y0 as i64 * f * f + 2 * y1 as i64 * f * e + y2 as i64 * e * e) >> 16) as i32;
        if p > 200 {
            a = a * (256 - p) as u32 / 56;
        }
        (x, y, a)
    }

    fn guest_area(&self) -> Rect {
        if self.guest_t == 0 {
            return rect(0, 0, 0, 0);
        }
        let y0 = self.cy - self.s(40);
        let y1 = self.bar.y + self.bar.h;
        rect(self.w / 2 - self.s(720), y0, self.s(1440), y1 - y0)
    }

    fn draw_guest(&mut self, r: &Rect, t: u32) {
        if self.guest_t == 0 || t.wrapping_sub(self.guest_t) > 2400 || !meets(self.guest_box, r) {
            return;
        }
        let p = prog(t, self.guest_t, self.guest_t + 900) as u32;
        if p < 256 {
            let y = self.bar.y + self.bar.h / 2 + self.s(7) - ((self.s(90) as i64 * ease_out(p as i32) as i64) >> 8) as i32;
            let gx = self.guest_x;
            text(&mut self.back, r, &mut self.body, gx, y, &self.guest, rgb(196, 208, 255), (256 - p) * 210 >> 8);
        }
        for i in 0..9u32 {
            let (x, y, a) = self.curve(i, t);
            if a == 0 {
                continue;
            }
            let (g9, g3) = (self.s(9) * 16, self.s(3) * 16);
            glow(&mut self.back, r, x, y, g9, rgb(150, 170, 255), a / 4);
            glow(&mut self.back, r, x, y, g3, rgb(255, 236, 210), a);
        }
    }

    // ---- the notes constellation ----

    fn map_point(&self, j: usize) -> (i32, i32) {
        let spread = if self.n > 7 { self.n as u64 - 1 } else { 6 };
        let rad = self.mr as i64 * 16 * isqrt(j as u64 * 65536 / spread) as i64 / 256;
        let ang = (j as u32).wrapping_mul(25033);
        (self.mx * 16 + 8 + ((rad * cos(ang) as i64) >> 14) as i32, self.my * 16 + 8 + ((rad * sin(ang) as i64) >> 14) as i32)
    }

    fn draw_map(&mut self, r: &Rect, t: u32) {
        let big_a = self.map_a;
        if big_a == 0 || self.n == 0 || !meets(self.map_box, r) {
            return;
        }
        for i in 0..self.ne {
            let (x0, y0) = self.map_point(self.ea[i] as usize);
            let (x1, y1) = self.map_point(self.eb[i] as usize);
            thread(&mut self.back, r, x0, y0, x1, y1, rgb(130, 150, 255), 70 * big_a >> 8, 70 * big_a >> 8);
        }
        for j in 0..self.n {
            let (x, y) = self.map_point(j);
            let lit = j as i32 == self.lit;
            let pulse = if lit { ((16384 + sin(t.wrapping_sub(self.map_t).wrapping_mul(65536) / 1400)) as u32 * 60) >> 15 } else { 0 };
            let c = if lit { rgb(255, 214, 160) } else { rgb(206, 220, 255) };
            let (g1, g2) = (self.s(if lit { 16 } else { 8 }) * 16, self.s(if lit { 4 } else { 3 }) * 16);
            glow(&mut self.back, r, x, y, g1, c, (if lit { 70 + pulse } else { 30 }) * big_a >> 8);
            glow(&mut self.back, r, x, y, g2, c, (if lit { 255 } else { 190 }) * big_a >> 8);
        }
        if self.lit >= 0 && t.wrapping_sub(self.map_t) < 1100 {
            let (lx, ly) = self.map_point(self.lit as usize);
            let p = prog(t, self.map_t, self.map_t + 900);
            let mut k = 0;
            while k < 4 && p < 256 {
                let e = smooth(clamp(p - k * 18, 0, 256)) as i64;
                let x = self.mind.hx + (((lx - self.mind.hx) as i64 * e) >> 8) as i32;
                let y = self.mind.hy + (((ly - self.mind.hy) as i64 * e) >> 8) as i32 - ((self.s(60) as i64 * 16 * (e * (256 - e) >> 6)) >> 8) as i32;
                let g5 = self.s(5) * 16;
                glow(&mut self.back, r, x, y, g5, rgb(255, 222, 176), 220u32 >> k);
                k += 1;
            }
        }
        let mut line = put_num(self.total);
        line.extend_from_slice(if self.total == 1 { b" note, " } else { b" notes, " });
        line.extend_from_slice(&put_num(self.links));
        line.extend_from_slice(if self.links == 1 { b" link" } else { b" links" });
        let hw = self.small.measure(b"Notes", 5) as i32;
        let (mx, my, mr) = (self.mx, self.my, self.mr);
        let (s34, s40, s68) = (self.s(34), self.s(40), self.s(68));
        text(&mut self.back, r, &mut self.small, mx - hw / 2, my - mr - s34, b"Notes", rgb(170, 180, 214), 170 * big_a >> 8);
        let lw = self.small.measure(&line, line.len()) as i32;
        text(&mut self.back, r, &mut self.small, mx - lw / 2, my + mr + s40, &line, rgb(150, 160, 196), 150 * big_a >> 8);
        if self.lit >= 0 && !self.label.is_empty() {
            let mut m = self.label.len();
            let room = self.s(300);
            let s14 = self.s(14);
            while m > 0 && self.small.measure(&self.label, m) as i32 + (if m < self.label.len() { s14 } else { 0 }) > room {
                m -= 1;
            }
            let mut cut: Vec<u8> = self.label[..m].to_vec();
            if m < self.label.len() {
                cut.extend_from_slice(b"...");
            }
            let cw = self.small.measure(&cut, cut.len()) as i32;
            text(&mut self.back, r, &mut self.small, mx - cw / 2, my + mr + s68, &cut, rgb(255, 222, 180), 200 * big_a >> 8);
        }
    }

    // ---- the flash of the merger ----

    fn flash(&self, t: u32) -> Option<(i32, u32)> {
        if t < night::MERGE_MS || t >= night::MERGE_MS + 1600 {
            return None;
        }
        let tm = t - night::MERGE_MS;
        let rad = (self.s(40) + ((self.s(220) as i64 * ease_out(prog(tm, 0, 1200)) as i64) >> 8) as i32) * 16;
        let rise = smooth(prog(tm, 0, 120));
        let a = rise * (exp_neg(if tm > 120 { (tm - 120) * 256 / 450 } else { 0 }) >> 8) * 230 >> 16;
        if a > 0 { Some((rad, a)) } else { None }
    }

    fn unflash(&mut self, r: &Rect, t: u32) {
        let tm = t - night::MERGE_MS;
        if self.night.holes != 1 || tm < 100 {
            return;
        }
        let keep = 256 - smooth(prog(tm, 100, 400));
        let (hx, hy, rs) = (self.night.hole[0].x, self.night.hole[0].y, self.night.hole[0].rs);
        let mut bx = rect((hx - rs) >> 4, (hy - rs) >> 4, ((2 * rs) >> 4) + 2, ((2 * rs) >> 4) + 2);
        if rs <= 16 || !clip(&mut bx, r) {
            return;
        }
        let w = self.w as usize;
        for y in bx.y..bx.y + bx.h {
            for x in bx.x..bx.x + bx.w {
                let dx = x * 16 + 8 - hx;
                let dy = y * 16 + 8 - hy;
                let d = isqrt((dx * dx + dy * dy) as u32 as u64) as u32;
                if d >= rs as u32 {
                    continue;
                }
                let inn: u32 = if rs as u32 - d >= 16 { 256 } else { (rs as u32 - d) * 16 };
                let k = 256 - (inn * (256 - keep) >> 8);
                let p = &mut self.back.px[y as usize * w + x as usize];
                *p = scale(*p, k);
            }
        }
    }

    fn flash_box(&self, t: u32) -> Rect {
        match self.flash(t) {
            None => rect(0, 0, 0, 0),
            Some((rad, _)) => {
                let r = (rad >> 4) + 2;
                rect(self.cx - r, self.cy - r, 2 * r + 1, 2 * r + 1)
            }
        }
    }

    // ---- putting a part of the screen together ----

    fn present(&mut self, r: &Rect) {
        let w = self.w as usize;
        for y in r.y..r.y + r.h {
            let base = y as usize * w + r.x as usize;
            for x in 0..r.w as usize {
                let c = self.back.px[base + x];
                self.fb[base + x] = 0xff000000 | (c & 0xff00) | (c >> 16 & 0xff) | (c & 0xff) << 16;
            }
        }
    }

    fn dark(&mut self, r: &Rect) {
        let w = self.w as usize;
        for y in r.y..r.y + r.h {
            for x in 0..r.w as usize {
                self.back.px[y as usize * w + r.x as usize + x] = 0;
            }
        }
        self.present(r);
    }

    fn dawn(&mut self, r: &Rect) {
        let keep = 256 - (self.dim >> 8);
        let w = self.w as usize;
        for y in r.y..r.y + r.h {
            for x in 0..r.w as usize {
                let p = &mut self.back.px[y as usize * w + r.x as usize + x];
                *p = scale(*p, keep);
            }
        }
    }

    /// The dusk at d ms: how far everything has fallen, the hole, the disk, the closing.
    fn horizon_at(&mut self, d: u32) {
        let mut k = Dusk::default();
        let mut q = smooth(prog(d, 0, 3400));
        q = (q * q >> 8) * q >> 8;
        k.fallen = (self.far16 as i64 * self.far16 as i64 * q as i64) >> 8;
        k.swirl = 65536 * 6 / 10;
        k.calm = self.s(120) * 16;
        k.crowd = 256 + (768 * smooth(prog(d, 300, 2400)) >> 8);
        k.e = ((self.s(90) as i64 * 16 * smooth(prog(d, 0, 2000)) as i64) >> 8) as i32;
        let rs0 = ((self.s(60) as i64 * 16 * smooth(prog(d, 0, 2000)) as i64) >> 8) as i32;
        k.rs = rs0 + ((self.s(60) as i64 * 16 * smooth(prog(d, 3400, 4200)) as i64) >> 8) as i32;
        let rise = smooth(prog(d, 900, 2800));
        let fall = 256 - smooth(prog(d, 3100, 3900));
        k.disk = if rise < fall { rise } else { fall };
        k.beam = 16384u32.wrapping_add(d * 65536 / 9000);
        k.keep = 256 - ease_out(prog(d, 2800, 4000));
        k.near = self.s(260) * 16;
        k.far = self.far16;
        k.shade = 230 * smooth(prog(d, 400, 2600)) >> 8;
        k.fine = self.s(300) * 16;
        self.night.fall(&k);
    }

    fn compose(&mut self, r: &Rect) {
        let t = self.t;
        if self.asleep && self.dim == DIM_FULL {
            self.dark(r);
            return;
        }
        if self.asleep && self.dusk_t != 0 {
            if let Some(kept) = &self.kept {
                self.night.base_from(kept, &mut self.back, r);
            }
            self.present(r);
            return;
        }
        self.night.base(&mut self.back, r);
        self.night.lights(&mut self.back, r, self.shimmer, self.level * self.shimmer >> 9);
        self.flow.draw(&mut self.back, r, 256);
        if let Some((rad, a)) = self.flash(t) {
            glow(&mut self.back, r, self.cx * 16 + 8, self.cy * 16 + 8, rad, rgb(200, 210, 255), a / 3);
            glow(&mut self.back, r, self.cx * 16 + 8, self.cy * 16 + 8, rad / 3, rgb(255, 240, 224), a);
            self.unflash(r, t);
        }
        self.draw_map(r, t);
        self.mind.draw(&mut self.back, r);
        self.draw_guest(r, t);
        self.draw_words(r, t);
        self.draw_bar(r, t);
        if self.dim != 0 {
            self.dawn(r);
        }
        self.present(r);
    }

    // ---- the tiles that changed ----

    /// the tiles the ripple's band crosses, between radius lo and hi
    fn mark_ring(&mut self, lo: i32, hi: i32) {
        if hi <= 0 {
            return;
        }
        let lo = max(lo, 0);
        let lo2 = lo as i64 * lo as i64;
        let hi2 = hi as i64 * hi as i64;
        for ty in 0..self.ty {
            for tx in 0..self.tx {
                let x0 = tx * TW;
                let y0 = ty * TH;
                let x1 = x0 + TW;
                let y1 = y0 + TH;
                let nx = (clamp(self.cx, x0, x1) - self.cx) as i64;
                let ny = (clamp(self.cy, y0, y1) - self.cy) as i64;
                let fx = max(abs(x0 - self.cx), abs(x1 - self.cx)) as i64;
                let fy = max(abs(y0 - self.cy), abs(y1 - self.cy)) as i64;
                if nx * nx + ny * ny <= hi2 && fx * fx + fy * fy >= lo2 {
                    self.tiles[(ty * self.tx + tx) as usize] = 1;
                }
            }
        }
    }

    fn strips(&mut self) {
        self.jobs.clear();
        let (tx_n, ty_n) = (self.tx, self.ty);
        for ty in 0..ty_n {
            let base = (ty * tx_n) as usize;
            let mut runs = 0;
            let mut first = -1i32;
            let mut last = -1i32;
            for tx in 0..tx_n {
                if self.tiles[base + tx as usize] != 0 {
                    if first < 0 {
                        first = tx;
                    }
                    if tx == 0 || self.tiles[base + tx as usize - 1] == 0 {
                        runs += 1;
                    }
                    last = tx;
                }
            }
            if first < 0 {
                continue;
            }
            let y = ty * TH;
            let h = min(TH, self.h - y);
            if runs > 6 || self.jobs.len() + runs as usize > JOBS_MAX {
                let x = first * TW;
                self.jobs.push(rect(x, y, min((last + 1) * TW, self.w) - x, h));
            } else {
                for tx in 0..tx_n {
                    if self.tiles[base + tx as usize] == 0 || (tx > 0 && self.tiles[base + tx as usize - 1] != 0) {
                        continue;
                    }
                    let mut end = tx;
                    while end + 1 < tx_n && self.tiles[base + end as usize + 1] != 0 {
                        end += 1;
                    }
                    self.jobs.push(rect(tx * TW, y, min((end + 1) * TW, self.w) - tx * TW, h));
                }
            }
            for tx in 0..tx_n {
                self.tiles[base + tx as usize] = 0;
            }
            if self.jobs.len() >= JOBS_MAX {
                break;
            }
        }
    }

    // ---- the frame ----

    fn take_notes(&mut self, t: u32) {
        if self.note.refresh {
            self.note.refresh = false;
            self.mark_all();
        }
        if self.note.wake_gen != self.seen_wake {
            self.seen_wake = self.note.wake_gen;
            self.footer = self.note.footer.clone();
            if !self.on {
                self.on = true;
                self.wake_t = t;
                self.quiet_t = max(t as i32, FORMED_MS as i32) as u32;
            }
        }
        if self.note.sleep_gen != self.seen_sleep {
            self.seen_sleep = self.note.sleep_gen;
            self.asleep = self.note.sleep;
            self.mark_all();
        }
        if self.note.input_gen != self.seen_input {
            self.seen_input = self.note.input_gen;
            if self.note.input.len() > self.input.len() {
                self.mind.key(t);
                self.flow.key(t);
            }
            self.input = self.note.input.clone();
            self.key_t = t;
            self.quiet_t = t;
            self.input_moved = true;
        }
        if self.note.said_gen != self.seen_said {
            self.seen_said = self.note.said_gen;
            if self.note.has_line {
                self.guest = copy(&self.note.line, TEXT_MAX);
                self.guest_x = self.bar.x + self.s(30);
                self.guest_t = t;
            }
            if self.now.len() != 0 && self.now.fade == 0 {
                self.now.fade = t;
            }
            self.next = Words::default();
            if self.note.think {
                self.speak_t = 0;
                self.thinking = true;
            } else {
                self.next.text = copy(&self.note.reply, TEXT_MAX);
                let think: u32 = if self.thinking {
                    400
                } else if self.note.has_line {
                    900 + min(self.next.len() as i32 * 6, 700) as u32
                } else {
                    500
                };
                self.speak_t = max((t + think) as i32, FORMED_MS as i32 + 300) as u32;
                self.thinking = false;
            }
            self.input.clear();
            self.input_moved = true;
            self.quiet_t = t;
            self.mind.set_mode(mind::THINK, t);
        }
        if self.note.map_gen != self.seen_map {
            self.seen_map = self.note.map_gen;
            self.n = self.note.n;
            self.ne = self.note.ea.len();
            self.total = self.note.total;
            self.links = self.note.links;
            self.lit = self.note.lit;
            self.ea = self.note.ea.clone();
            self.eb = self.note.eb.clone();
            self.label = copy(&self.note.label, TEXT_MAX);
            if self.lit >= 0 && self.n != 0 {
                self.map_t = if t != 0 { t } else { 1 };
            }
            let mb = self.map_box;
            self.mark(mb);
        }
    }

    /// moves everything to t and marks what changed
    fn advance(&mut self, t: u32) {
        let prev = self.last;
        let mut dt = t.wrapping_sub(prev);
        self.take_notes(t);
        if dt > 100 {
            dt = 100;
        }
        let dim_was = self.dim;
        if self.asleep {
            if self.dusk_t == 0 {
                self.dusk_t = if t != 0 { t } else { 1 };
                self.kept = Some(self.night.picture_take(&self.back.px));
            }
            let d = t.wrapping_sub(self.dusk_t);
            self.dim = if d >= DUSK_MS { DIM_FULL } else { (DIM_FULL as u64 * d as u64 / DUSK_MS as u64) as u32 };
            self.mark_all();
            if self.dim == DIM_FULL {
                return;
            }
            self.night.at(t);
            self.horizon_at(d);
            return;
        }
        if self.dusk_t != 0 {
            self.dusk_t = 0;
            self.dim = DIM_FULL;
        }
        let dim_step = (DIM_FULL as u64 * dt as u64 / DAWN_MS as u64) as u32;
        self.dim = if self.dim > dim_step { self.dim - dim_step } else { 0 };
        if self.dim != dim_was || self.dim != 0 {
            self.mark_all();
        }
        let fmode = if self.asleep || !self.on || t < FORMED_MS {
            flow::SLEEP
        } else if self.mind.mode == mind::THINK {
            flow::THINK
        } else if self.mind.mode == mind::SPEAK {
            flow::SPEAK
        } else if self.mind.mode == mind::LISTEN {
            flow::LISTEN
        } else {
            flow::IDLE
        };
        self.flow.set_mode(fmode, t);
        self.night.at(t);
        if t < night::FADE_END_MS + FRAME_MS * 2 {
            self.mark_all();
        }
        let lw = self.lens_was;
        self.mark(lw);
        let nl = self.night.lens;
        self.mark(nl);
        self.lens_was = self.night.lens;
        if self.night.ripple_w != 0 || self.ripple_was >= 0 {
            let reach = self.night.ripple_w * 22 / 10 + 2;
            if self.ripple_was >= 0 {
                self.mark_ring(self.ripple_was - reach - 8, self.ripple_was + reach + 8);
            }
            if self.night.ripple_w != 0 {
                self.mark_ring(self.night.ripple_r - reach - 8, self.night.ripple_r + reach + 8);
            }
            self.ripple_was = if self.night.ripple_w != 0 { self.night.ripple_r } else { -1 };
        }
        let fb = self.flash_box(t);
        let fw = self.flash_was;
        self.mark(fw);
        self.mark(fb);
        self.flash_was = fb;
        let mut i = (self.twinkle_turn % 3) as usize;
        while i < self.night.tw.len() {
            let b = self.night.star_box(i);
            self.mark(b);
            i += 3;
        }
        self.twinkle_turn += 1;

        if self.speak_t != 0 && t >= self.speak_t {
            self.now = std::mem::take(&mut self.next);
            self.now.from = self.speak_t;
            self.lay_out(true);
            self.speak_t = 0;
            self.mind.set_mode(mind::SPEAK, t);
        }
        if self.now.len() != 0 && t >= self.now.from && prev < self.now.done {
            for i in 0..self.now.len() {
                let at = self.now.from + i as u32 * LETTER_MS;
                if at > prev && at <= t && self.now.text[i] != b' ' && (i == 0 || self.now.text[i - 1] == b' ') {
                    self.mind.word(t);
                    self.flow.word(t);
                    self.word_t = t;
                }
            }
        }
        let wlen = self.now.len();
        let wfrom = self.now.from;
        let wdone = self.now.done;
        let last_letter = if wlen != 0 { wfrom + wlen as u32 * LETTER_MS } else { 0 };
        let speaking = wlen != 0 && t >= wfrom && t < last_letter + 300;
        self.level = if speaking { exp_neg(t.wrapping_sub(self.word_t) * 256 / 200) >> 8 } else { 0 };
        self.mind.set_voice(self.level);
        if !speaking && self.speak_t == 0 && (self.mind.mode == mind::SPEAK || self.mind.mode == mind::THINK) && wlen != 0 && t >= wfrom {
            self.mind.set_mode(mind::IDLE, t);
            self.quiet_t = t;
        }
        if self.mind.mode != mind::THINK && self.mind.mode != mind::SPEAK {
            let m = if !self.input.is_empty() && t.wrapping_sub(self.key_t) < 4000 { mind::LISTEN } else { mind::IDLE };
            self.mind.set_mode(m, t);
        }
        let rise = if wlen != 0 { smooth(prog(t, wfrom, wfrom + 600)) } else { 0 };
        let fall = if wlen != 0 { 256 - smooth(prog(t, last_letter + 300, last_letter + 2800)) } else { 0 };
        self.shimmer = if rise < fall { rise } else { fall };
        if self.shimmer != 0 || self.shimmer_was != 0 {
            for i in 0..night::GALAXIES {
                let b = self.night.gal[i].bx;
                self.mark(b);
            }
        }
        self.shimmer_was = self.shimmer;
        if wlen != 0 && (t < wdone + 40 || words_alpha(&self.now, t) != words_alpha(&self.now, prev)) {
            let b = self.now.bx;
            self.mark(b);
        }
        let ww = self.words_was;
        if ww.w != 0 && (ww.x != self.now.bx.x || ww.w != self.now.bx.w || ww.h != self.now.bx.h) {
            self.mark(ww);
        }
        self.words_was = self.now.bx;
        if wlen != 0 && words_alpha(&self.now, t) == 0 && t > wfrom {
            self.now = Words::default();
        }

        self.flow.at(t);
        {
            let (w, h, tx) = (self.w, self.h, self.tx);
            let tiles = &mut self.tiles;
            self.flow.mark(|r| mark_into(tiles, w, h, tx, r));
        }
        let (vx, vy) = self.flow.velocity(self.cx, self.cy);
        let s10 = self.s(10);
        self.mind.cx = self.cx + clamp(vx * s10 / 1000, -s10, s10);
        self.mind.cy = self.cy + clamp(vy * s10 / 1000, -s10, s10);
        self.mind.at(t);
        let md = self.mind.dirty();
        self.mark(md);
        if self.guest_t != 0 {
            let g = self.guest_area();
            self.mark(g);
            self.guest_box = g;
            if t.wrapping_sub(self.guest_t) > 2400 {
                self.guest_t = 0;
            }
        }

        let want = self.on && t >= FORMED_MS && (!self.input.is_empty() || t.wrapping_sub(self.quiet_t) < BAR_REST_MS || speaking || self.speak_t != 0);
        self.bar_a = toward(self.bar_a, if want { 256 } else { 0 }, dt, if want { if t.wrapping_sub(self.key_t) < 300 { 250 } else { 900 } } else { 1400 });
        if self.bar_a != 0 || self.bar_was != 0 || self.input_moved {
            let bb = self.bar_box;
            self.mark(bb);
        }
        self.bar_was = self.bar_a;
        self.input_moved = false;

        let show = self.map_t != 0 && t.wrapping_sub(self.map_t) < MAP_SHOW_MS && self.n != 0;
        self.map_a = toward(self.map_a, if show { 256 } else { 0 }, dt, if show { 700 } else { 1500 });
        if self.map_a != 0 || self.map_was != 0 {
            let mb = self.map_box;
            self.mark(mb);
        }
        self.map_was = self.map_a;
        if self.lit >= 0 && self.map_t != 0 && t.wrapping_sub(self.map_t) < 1200 {
            let u = union(self.mind.dirty(), self.map_box);
            self.mark(u);
        }
    }

    /// Draws a frame when one is due (at most 60 a second); true if it drew one. `now` is ms
    /// since the window opened.
    pub fn tick(&mut self, now: u32) -> bool {
        let t = now;
        if self.frames != 0 && t.wrapping_sub(self.last) < FRAME_MS && !self.note.refresh {
            return false;
        }
        self.t = t;
        self.advance(t);
        self.strips();
        let jobs = std::mem::take(&mut self.jobs);
        for j in &jobs {
            self.compose(j);
        }
        self.jobs = jobs;
        self.last = t;
        self.frames += 1;
        true
    }
}
