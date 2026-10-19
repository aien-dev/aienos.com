//! Runs the port through the same script as evidence/allen-sim/reference.c (which drives the
//! kernel's own C code) and compares the screen hash after every 4th frame, line for line.
use allen_sim::chirp::{Chirp, FRAMES};
use allen_sim::ui::Ui;

const REFERENCE: &str = include_str!("../../../evidence/allen-sim/reference.txt");
const EA: [u8; 15] = [0, 0, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 2, 5];
const EB: [u8; 15] = [1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11, 7, 9];

fn fnv(h: &mut u64, b: u8) {
    *h ^= b as u64;
    *h = h.wrapping_mul(1099511628211);
}

fn hash_frame(p: &[u32]) -> u64 {
    let mut h = 1469598103934665603u64;
    for &c in p {
        let v = c & 0x00ffffff;
        for b in 0..3 {
            fnv(&mut h, (v >> (8 * b)) as u8);
        }
    }
    h
}

fn at(t: u32, x: u32) -> bool {
    t >= x && t < x + 16
}

fn run(w: u32, h: u32, out: &mut Vec<String>) {
    let mut ui = Ui::new(w, h).expect("start");
    out.push(format!("size {}x{}", w, h));
    let typed = b"Hello Allen";
    let mut t = 0;
    while t <= 44000 {
        if at(t, 14000) {
            ui.wake("Say anything");
        }
        if t >= 15000 && t < 15000 + 11 * 140 && (t - 15000) % 140 < 16 {
            let n = ((t - 15000) / 140 + 1) as usize;
            ui.input(&typed[..n]);
        }
        if at(t, 17000) {
            ui.said(Some("Hello Allen"), Some("Hello Humanity. I am here, and the night is clear."));
        }
        if at(t, 25000) {
            ui.map(12, &EA, &EB, 3, "a note about the night sky", 12, 15);
        }
        if at(t, 30000) {
            ui.input(b"what is");
        }
        if at(t, 31000) {
            ui.think("what is water");
        }
        if at(t, 33000) {
            ui.said(None, Some("Water is where I live."));
        }
        if at(t, 36000) {
            ui.sleep(true);
        }
        if at(t, 41000) {
            ui.sleep(false);
        }
        let drew = ui.tick(t);
        if (t / 16) % 4 == 0 {
            out.push(format!("{} {} {:016x}", t, drew as u32, hash_frame(&ui.fb)));
        }
        t += 16;
    }
    out.push(format!("asleep {} on {}", ui.asleep() as u32, ui.is_on() as u32));
}

#[test]
fn screen_matches_the_kernel_code() {
    let mut out = Vec::new();
    run(1280, 720, &mut out);
    run(800, 500, &mut out);
    let c = Chirp::new();
    let mut h = 1469598103934665603u64;
    for f in 0..FRAMES {
        let s = c.sample(f) as u16;
        for _ in 0..2 {
            fnv(&mut h, s as u8);
            fnv(&mut h, (s >> 8) as u8);
        }
    }
    out.push(format!("chirp {:016x}", h));
    let want: Vec<&str> = REFERENCE.lines().collect();
    let mut first_bad = None;
    for (i, (a, b)) in out.iter().zip(want.iter()).enumerate() {
        if a != b && first_bad.is_none() {
            first_bad = Some(i);
        }
    }
    if let Some(i) = first_bad {
        let bad = out.iter().zip(want.iter()).filter(|(a, b)| a != b).count();
        panic!("first difference at line {}: port `{}` kernel `{}` ({} lines differ of {})", i + 1, out[i], want[i], bad, want.len());
    }
    assert_eq!(out.len(), want.len());
}
