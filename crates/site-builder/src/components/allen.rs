//! Allen: the live window on the front page and the /allen/ page.
//!
//! The picture is drawn by `public/allen/allen.wasm`, a Rust port (crates/allen-sim) of the AIENOS
//! kernel's own integer drawing code, checked frame by frame against the C original
//! (evidence/allen-sim). `public/allen/allen.js` carries the canvas, keyboard, clock and sound.
//! The words come from `public/allen/brain.js`, a port of Allen's reply rules. First party files
//! only; the page makes no outside request. Without scripts the recorded video stands in.

use maud::{html, Markup};

use super::boxlink::{foot, source_href, Target};

pub const TITLE: &str = "Allen | AIENOS";
pub const DESCRIPTION: &str = "Allen is a small mind of light that lives in AIENOS. This is his window, drawn live in your browser by a port of the kernel's own drawing code, with the recording, the sound and an honest list of where he runs.";
pub const URL: &str = "https://www.aienos.com/allen/";

/// Where Allen runs. Every row links to public evidence, and says what is not true yet.
pub struct Place {
    pub id: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub state: &'static str,
    pub today: &'static str,
    pub not_yet: &'static str,
    pub link: &'static str,
    pub links: &'static [(&'static str, &'static str)],
}

pub const PLACES: [Place; 5] = [
    Place {
        id: "imac",
        name: "Intel iMac 2017",
        status: "Bare metal",
        state: "runs",
        today: "The whole screen is Allen's window: the night sky, the black holes, Allen, the quiet bar. The kernel draws it with the same code this page runs. The machine boots from its own disk, or from a signed image sent over the network by the Spark.",
        not_yet: "The kernel that draws the window today is in open pull requests, not on main yet. The goodbye dusk passed its host tests but has not been run on the real iMac.",
        link: "https://github.com/aien-dev/aienos-mv/blob/main/evidence/imac_kernel1_2026-10-17",
        links: &[
            ("The x86 port", "https://github.com/aien-dev/aienos-mv/blob/main/native/kernel/x86"),
            ("Allen's chat, PR #415", "https://github.com/aien-dev/aienos-mv/pull/415"),
            ("Screensaver, PR #424", "https://github.com/aien-dev/aienos-mv/pull/424"),
        ],
    },
    Place {
        id: "pi5",
        name: "Raspberry Pi 5",
        status: "Decision maker only",
        state: "part",
        today: "ALLEN REFLEX, a small decision maker for Allen, was built for the Pi 5 kernel image. Version 0 is merged and got 21 of 26 held-out answers exactly right. The Pi has also run the kernel on real hardware: interrupts, four cores, user tasks and a log written to its SD card and read back.",
        not_yet: "Allen's drawn window on the Pi is not claimed. The interactive version on the Pi image is an open pull request.",
        link: "https://github.com/aien-dev/aienos-mv/blob/main/evidence/pi5_hardware_runs_2026-10-13",
        links: &[
            ("ALLEN REFLEX v0, PR #379", "https://github.com/aien-dev/aienos-mv/pull/379"),
            ("Interactive on the Pi, PR #402", "https://github.com/aien-dev/aienos-mv/pull/402"),
        ],
    },
    Place {
        id: "rig",
        name: "Windows PC with an RTX 5070",
        status: "In progress",
        state: "no",
        today: "This machine is the target for AIEN Rig, the local engine in a 12 GiB budget. The engine is being measured on the Spark under that budget.",
        not_yet: "Nothing of Allen's has run on the 5070 yet.",
        link: "https://github.com/aien-dev/aien-rig",
        links: &[("aien-rig", "https://github.com/aien-dev/aien-rig")],
    },
    Place {
        id: "qemu",
        name: "QEMU, the emulator",
        status: "Tests",
        state: "part",
        today: "The kernel's gates run in the emulator first, and there are emulator tests for the iMac kernel, its keyboard and Allen's courier. Last full receipt: 24 of 24 gates PASS at 7f0de7e.",
        not_yet: "The gate script now lists 26 gates, so a newer receipt is owed. An emulator qualifies nothing physical, by the repository's own words.",
        link: "https://github.com/aien-dev/aienos-mv/blob/main/evidence/ck_gates_29ed03c97454354a18676f768273dd7f7a8e15462c4be1939d9994d277d0d639.json",
        links: &[("The gate list", "https://github.com/aien-dev/aienos-mv/blob/main/native/kernel/GATES.md")],
    },
    Place {
        id: "spark",
        name: "NVIDIA DGX Spark",
        status: "Builds and judges",
        state: "part",
        today: "Allen's code is built and tested here, and the 27 billion parameter model runs here on our own GPU kernels. A branch that lets Allen's window ask that model through a courier is in progress.",
        not_yet: "The AIENOS C kernel has not booted on the Spark, and the end to end emulator test of the courier is pending.",
        link: "https://github.com/aien-dev/aien-sovereign-core-mv/tree/qwen38/serve-fast/evidence/qwen38-serve-fast-20261019",
        links: &[("First native boot, 2026-09-24", "https://github.com/aien-dev/aienos-mv/blob/main/evidence/m2_first_boot_2026-09-24.md")],
    },
];

/// The window itself: a canvas, a hidden text field for the keyboard, the buttons, the caption.
fn window(compact: bool) -> Markup {
    html! {
        div class=(if compact { "allen allen-compact" } else { "allen" }) data-allen {
            div class="allen-stage" data-allen-stage data-allen-state="loading" {
                img class="allen-poster" src="/allen/allen-window-poster.jpg" width="1280" height="720"
                    alt="Allen's window: a night sky with a small ball of blue light at its centre, a quiet text bar near the bottom.";
                canvas class="allen-canvas" data-allen-canvas width="1280" height="720" role="img"
                    aria-label="Allen's window, drawn live: a night sky, two black holes that merge, and Allen forming from the light." {}
                input class="allen-input" data-allen-input type="text" maxlength="120" autocomplete="off"
                    autocapitalize="off" spellcheck="false" enterkeyhint="send"
                    aria-label="Say something to Allen, then press Enter";
                button class="allen-wake" data-allen-wake type="button" hidden { "Wake Allen" }
            }
            div class="allen-controls" {
                button class="allen-btn" type="button" data-allen-sound aria-pressed="false" { "Sound off" }
                button class="allen-btn" type="button" data-allen-skip { "Skip to Allen" }
                button class="allen-btn" type="button" data-allen-replay { "Replay" }
                @if !compact {
                    button class="allen-btn" type="button" data-allen-pop { "Hear the pop" }
                }
            }
            p class="allen-status" data-allen-status aria-live="polite" { "Starting the window." }
            noscript {
                p class="allen-status" {
                    "The live window needs scripts from this site. The recording below shows the same screen."
                }
            }
        }
    }
}

/// The top of the front page: Allen first, one sentence of promise, one link in.
pub fn render_home_hero() -> Markup {
    html! {
        section id="allen" class="allen-hero" aria-labelledby="allen-hero-title" {
            div class="container allen-hero-grid" {
                div class="allen-hero-copy" {
                    p class="kicker" { "Meet Allen" }
                    h2 id="allen-hero-title" class="allen-hero-title" { "A small mind of light, living in the screen." }
                    p class="allen-hero-lead" {
                        "Allen is the face of AIENOS. He wakes in a night sky where two black holes merge, forms out of the light, and talks with whoever types to him. This is his window, drawn live in your browser by the same integer code the bare metal kernel uses."
                    }
                    p class="allen-hero-actions" {
                        a class="home-cta home-cta-primary" href="/allen/" { "Talk to Allen" }
                        a class="home-cta" href="/allen/#recording" { "Watch the real screen" }
                        a class="home-cta" href="/allen/#where" { "Where he runs" }
                    }
                    p class="allen-fine" {
                        "Pre-alpha. Click the window and type. Nothing you type leaves this page."
                    }
                }
                div class="allen-hero-window" { (window(true)) }
            }
        }
    }
}

const STEPS: [(&str, &str); 7] = [
    ("The night", "A sky of stars, a faint band of dust and a few distant galaxies, drawn pixel by pixel from the pixel's own position and the time. Nothing is a picture."),
    ("Two black holes", "They circle each other, bending the sky behind them. The lens you see is worked out for every pixel near them."),
    ("The merger", "They spiral in and merge. A ripple crosses the sky, and a short rising sound plays: the pop. It is the last moment of the chirp that two merging black holes would make, raised into hearing."),
    ("Allen forms", "A mind of light gathers where the holes were. Allen is made of points joined by threads, and he moves with the water of light around him."),
    ("He listens and speaks", "A quiet bar rises. As you type, ink leaves the bar and flows to him. When he answers, the words appear one by one and the sky shimmers with each."),
    ("Notes", "Ask him to save something and a small map of stars appears. Each star is a note; a thread joins notes that share words."),
    ("Goodbye", "Say goodbye and everything falls into a black hole, the disk glows, the horizon closes to black. Wake him and the sky comes back, calmly."),
];

fn steps() -> Markup {
    html! {
        ol class="allen-steps" {
            @for (i, (name, text)) in STEPS.iter().enumerate() {
                li class="allen-step" {
                    span class="allen-step-n mono" { (format!("{:02}", i + 1)) }
                    div {
                        h3 class="allen-step-name" { (name) }
                        p { (text) }
                    }
                }
            }
        }
    }
}

fn place_card(p: &Place) -> Markup {
    html! {
        article class="mx-card allen-place" id=(format!("allen-{}", p.id)) {
            header class="mx-card-head" {
                h3 class="mx-name" { (p.name) }
                p class="mx-detail" {
                    span class=(format!("allen-pill allen-pill-{}", p.state)) { (p.status) }
                }
            }
            dl class="mx-facts" {
                dt { "Today" }
                dd { (p.today) }
                dt { "Not yet" }
                dd { (p.not_yet) }
            }
            @if !p.links.is_empty() {
                footer class="mx-card-foot" {
                    span class="mx-more" {
                        @for (i, (label, link)) in p.links.iter().enumerate() {
                            @if i > 0 { " \u{b7} " }
                            a href=(link) target="_blank" rel="noopener noreferrer" { (label) }
                        }
                    }
                }
            }
            (foot(p.link, Target::Evidence))
        }
    }
}

/// The /allen/ page.
pub fn render_allen_page() -> Markup {
    html! {
        section class="allen-page-top" aria-labelledby="allen-title" {
            div class="container" {
                p class="kicker" { "Allen" }
                h1 id="allen-title" class="allen-title" { "Allen lives in the screen." }
                p class="allen-lead" {
                    "Allen is a small mind of light in AIENOS. On the iMac he is the whole screen. Below is his window, running live in your browser. Click it and talk to him. He answers with the same rules he uses on his own machine: no model and no network, and nothing you type leaves this page."
                }
                (window(false))
            }
        }

        section id="watching" class="allen-section" aria-labelledby="watching-title" {
            div class="container allen-prose" {
                p class="kicker" { "What you are watching" }
                h2 id="watching-title" class="allen-h2" { "Seven moments, from the dark to goodbye" }
                (steps())
            }
        }

        section id="made" class="allen-section" aria-labelledby="made-title" {
            div class="container allen-prose" {
                p class="kicker" { "How this page is made" }
                h2 id="made-title" class="allen-h2" { "The kernel's drawing code, in your browser" }
                p {
                    "The window on the iMac is drawn by integer code in the AIENOS kernel: no floating point, angles in 65536ths of a turn, positions in 16ths of a pixel. We ported that code to Rust and compiled it to WebAssembly, a small program the browser runs at near native speed. The script on this page only passes the canvas, the keyboard, the clock and the sound to it."
                }
                p {
                    "We checked the port against the original. The kernel's C code and the Rust port ran the same scripted evening at two screen sizes, covering the night, the merger, Allen forming, typing, an answer, thinking, the notes map, the dusk and the dawn. A hash of every screen, every 64 milliseconds, matched on all 1,381 comparison lines, and so did a hash of the sound. The compiled module in this page passes the same check at 1280 by 720."
                }
                p class="allen-more" {
                    a href=(source_href("evidence/allen-sim/README.md")) target="_blank" rel="noopener noreferrer" { "The check and its receipts" }
                    " \u{b7} "
                    a href=(source_href("crates/allen-sim/README.md")) target="_blank" rel="noopener noreferrer" { "The port" }
                }
                p {
                    "His words come from a JavaScript port of Allen's reply rules: greetings, your name, things you teach him, notes, sums said in words, a joke, help and goodbye. There is no model behind this window. The page keeps nothing once you close it."
                }
                p {
                    "This is a copy of Allen's window, not Allen himself. The real Allen runs in the kernel."
                }
            }
        }

        section id="recording" class="allen-section" aria-labelledby="recording-title" {
            div class="container allen-prose" {
                p class="kicker" { "The real screen" }
                h2 id="recording-title" class="allen-h2" { "A recording of the kernel's own window code" }
                p {
                    "This 70 second recording was made by running the kernel's own drawing code and the kernel's own reply code on the Spark, then encoding every second frame at 1280 by 720. It is not a screen capture of the iMac. It shows a first meeting: he has no name yet, asks what to call you, and is called Allen. Then a question, a sum, goodbye, the dusk and the dawn. The pop is the real sound at the merger. Press play and turn the volume up."
                }
                figure class="allen-video" {
                    video controls preload="none" playsinline poster="/allen/allen-window-poster.jpg" width="1280" height="720" {
                        source src="/allen/allen-window.mp4" type="video/mp4";
                        "Your browser cannot play this video. "
                        a href="/allen/allen-window.mp4" { "Download it" }
                        "."
                    }
                    figcaption {
                        "Made by evidence/allen-sim/build-video.sh from kernel snapshot 903277b."
                    }
                }
            }
        }

        section id="where" class="allen-section mx-section" aria-labelledby="where-title" {
            div class="container" {
                div class="mx-head" {
                    p class="kicker" { "Where Allen runs" }
                    h2 id="where-title" class="mx-h2" { "One Allen, five places, each stated plainly" }
                    p class="mx-lead" {
                        "A card says what runs on that machine today and what does not, with a link to the receipt. A line that is not yet true is written as not yet."
                    }
                }
                div class="mx-grid" {
                    @for p in PLACES.iter() { (place_card(p)) }
                }
                p class="mx-stamp" {
                    "Inventory as of 2026-10-19. Allen's window is open source, AGPL-3.0-or-later. "
                    a href="/machine/#real-machines" { "All the machines" }
                    "."
                }
            }
        }

        section id="read" class="allen-section" aria-labelledby="read-title" {
            div class="container allen-prose" {
                p class="kicker" { "Read the source" }
                h2 id="read-title" class="allen-h2" { "Everything here is open" }
                p {
                    "The kernel's window code is in "
                    a href="https://github.com/aien-dev/aienos-mv/tree/main/native/kernel/ui" target="_blank" rel="noopener noreferrer" { "native/kernel/ui" }
                    ". The port, the check and this page are in "
                    a href="https://github.com/aien-dev/aienos.com" target="_blank" rel="noopener noreferrer" { "aien-dev/aienos.com" }
                    "."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_text() -> String {
        format!(
            "{}{}",
            render_home_hero().into_string(),
            render_allen_page().into_string()
        )
    }

    #[test]
    fn no_dashes_and_no_comparison_words() {
        let s = all_text();
        assert!(!s.contains('\u{2014}') && !s.contains('\u{2013}'), "em or en dash");
        let lower = s.to_ascii_lowercase();
        for w in super::super::machines::FORBIDDEN {
            assert!(!lower.contains(w), "comparison word {w:?}");
        }
    }

    #[test]
    fn every_place_has_a_github_receipt() {
        for p in PLACES.iter() {
            assert!(p.link.starts_with("https://github.com/"), "{}", p.id);
            for (_, l) in p.links {
                assert!(l.starts_with("https://github.com/"), "{}", p.id);
            }
            assert!(!p.not_yet.is_empty(), "{} must say what is not yet true", p.id);
        }
    }

    #[test]
    fn window_has_what_the_script_looks_for() {
        let s = render_allen_page().into_string();
        for hook in ["data-allen-canvas", "data-allen-input", "data-allen-status", "data-allen-sound", "data-allen-skip", "data-allen-replay", "data-allen-pop"] {
            assert!(s.contains(hook), "missing {hook}");
        }
    }
}
