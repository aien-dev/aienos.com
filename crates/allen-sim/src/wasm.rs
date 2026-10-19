//! The WebAssembly face of the window: a handful of plain functions the page's glue script calls.
//! One window per page. Strings travel through a small scratch buffer inside the module.
use crate::chirp::{self, Chirp};
use crate::ui::Ui;
use std::cell::RefCell;

const SCRATCH: usize = 1024;
const CHIRP_BLOCK: usize = 4096;

struct State {
    ui: Option<Ui>,
    chirp: Chirp,
    scratch: Vec<u8>,
    sound: Vec<i16>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        ui: None,
        chirp: Chirp::new(),
        scratch: vec![0; SCRATCH],
        sound: vec![0; CHIRP_BLOCK],
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn text(s: &State, len: u32) -> String {
    String::from_utf8_lossy(&s.scratch[..(len as usize).min(SCRATCH)]).into_owned()
}

/// Opens the window on a w x h screen. 0 on success.
#[no_mangle]
pub extern "C" fn allen_new(w: u32, h: u32) -> i32 {
    with(|s| match Ui::new(w, h) {
        Some(ui) => {
            s.ui = Some(ui);
            0
        }
        None => -1,
    })
}

/// Where the page writes text (UTF-8) before calling a function that takes a length.
#[no_mangle]
pub extern "C" fn allen_scratch() -> *mut u8 {
    with(|s| s.scratch.as_mut_ptr())
}

/// The screen: w x h pixels, each four bytes R G B 255 (an ImageData's layout).
#[no_mangle]
pub extern "C" fn allen_screen() -> *const u32 {
    with(|s| s.ui.as_ref().map_or(std::ptr::null(), |u| u.fb.as_ptr()))
}

/// Moves to `ms` since the window opened and draws if a frame is due. 1 if it drew.
#[no_mangle]
pub extern "C" fn allen_tick(ms: u32) -> i32 {
    with(|s| s.ui.as_mut().map_or(0, |u| u.tick(ms) as i32))
}

/// Allen is ready to talk; the footer text (len bytes in the scratch) goes under the bar.
#[no_mangle]
pub extern "C" fn allen_wake(len: u32) {
    with(|s| {
        let t = text(s, len);
        if let Some(u) = s.ui.as_mut() {
            u.wake(&t);
        }
    })
}

#[no_mangle]
pub extern "C" fn allen_is_on() -> i32 {
    with(|s| s.ui.as_ref().is_some_and(|u| u.is_on()) as i32)
}

/// The line being typed (len bytes in the scratch).
#[no_mangle]
pub extern "C" fn allen_input(len: u32) {
    with(|s| {
        let n = (len as usize).min(SCRATCH);
        let b = s.scratch[..n].to_vec();
        if let Some(u) = s.ui.as_mut() {
            u.input(&b);
        }
    })
}

/// A line was entered and Allen answered: the scratch holds the line, then the reply, split at
/// `line_len`; `kind` 0 = both, 1 = Allen speaks first (reply only), 2 = thinking (line only).
#[no_mangle]
pub extern "C" fn allen_said(kind: u32, line_len: u32, reply_len: u32) {
    with(|s| {
        let l = (line_len as usize).min(SCRATCH);
        let r = (reply_len as usize).min(SCRATCH - l);
        let line = String::from_utf8_lossy(&s.scratch[..l]).into_owned();
        let reply = String::from_utf8_lossy(&s.scratch[l..l + r]).into_owned();
        if let Some(u) = s.ui.as_mut() {
            match kind {
                0 => u.said(Some(&line), Some(&reply)),
                1 => u.said(None, Some(&reply)),
                _ => u.think(&line),
            }
        }
    })
}

/// Allen sleeps (1: the dusk) or wakes (0).
#[no_mangle]
pub extern "C" fn allen_sleep(asleep: u32) {
    with(|s| {
        if let Some(u) = s.ui.as_mut() {
            u.sleep(asleep != 0);
        }
    })
}

#[no_mangle]
pub extern "C" fn allen_asleep() -> i32 {
    with(|s| s.ui.as_ref().is_some_and(|u| u.asleep()) as i32)
}

/// The sound's length in frames at 48 kHz.
#[no_mangle]
pub extern "C" fn allen_chirp_frames() -> u32 {
    chirp::FRAMES
}

/// Works out sound frames [from, from + n) (n at most 4096) and returns where they are: n 16-bit
/// samples, one channel.
#[no_mangle]
pub extern "C" fn allen_chirp(from: u32, n: u32) -> *const i16 {
    with(|s| {
        let n = (n as usize).min(CHIRP_BLOCK);
        let State { chirp, sound, .. } = s;
        chirp.fill(&mut sound[..n], from);
        sound.as_ptr()
    })
}

/// The notes map: n stars, ne links (the scratch holds ne bytes of one end, ne of the other, then
/// the label), `lit` the star just saved (-1 none).
#[no_mangle]
pub extern "C" fn allen_map(n: u32, ne: u32, lit: i32, label_len: u32, total: u32, links: u32) {
    with(|s| {
        let ne = (ne as usize).min(SCRATCH / 3);
        let ea = s.scratch[..ne].to_vec();
        let eb = s.scratch[ne..2 * ne].to_vec();
        let l = (label_len as usize).min(SCRATCH - 2 * ne);
        let label = String::from_utf8_lossy(&s.scratch[2 * ne..2 * ne + l]).into_owned();
        if let Some(u) = s.ui.as_mut() {
            u.map(n as usize, &ea, &eb, lit, &label, total, links);
        }
    })
}

/// The first sound frame that is not silence (the sound comes in here, before the sweep).
#[no_mangle]
pub extern "C" fn allen_chirp_start() -> u32 {
    chirp::RATE / 1000 * (chirp::START_MS - chirp::IN_MS)
}
