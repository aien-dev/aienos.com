//! dump <width> <ms> <file>: runs the reference script and writes the raw screen at <ms> (same as
//! `reference <width> <ms> <file>` built from evidence/allen-sim/reference.c), for finding the first
//! differing pixel.
use allen_sim::ui::Ui;
use std::io::Write;

const EA: [u8; 15] = [0, 0, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 2, 5];
const EB: [u8; 15] = [1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11, 7, 9];

fn at(t: u32, x: u32) -> bool {
    t >= x && t < x + 16
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let w: u32 = a[1].parse().unwrap();
    let dump: u32 = a[2].parse().unwrap();
    let mut ui = Ui::new(w, if w == 800 { 500 } else { w * 9 / 16 }).unwrap();
    let typed = b"Hello Allen";
    let mut t = 0;
    while t <= dump {
        if at(t, 14000) {
            ui.wake("Say anything");
        }
        if t >= 15000 && t < 15000 + 11 * 140 && (t - 15000) % 140 < 16 {
            ui.input(&typed[..((t - 15000) / 140 + 1) as usize]);
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
        ui.tick(t);
        t += 16;
    }
    let bytes: Vec<u8> = ui.fb.iter().flat_map(|c| c.to_le_bytes()).collect();
    std::fs::File::create(&a[3]).unwrap().write_all(&bytes).unwrap();
}
