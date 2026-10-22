use maud::{html, Markup};

use super::boxlink::{foot, Target};
use super::research_status as rs;

/// One receipt card. Every field is written from the site inventory of
/// 2026-10-19 (Spark ~/handoffs/2026-10-19-site-inventory.md, ledger L057),
/// which checked each link on GitHub. Each card states what the receipt
/// proves and what it does not prove, so it can be quoted without being
/// misquoted. The failure cards stay: a meter you cannot fail is not a meter.
struct Card {
    verdict: &'static str,
    badge: &'static str,
    figure: &'static str,
    title: &'static str,
    desc: &'static str,
    date: &'static str,
    environment: &'static str,
    gate: &'static str,
    proves: &'static str,
    not_proves: &'static str,
    url: &'static str,
    source: &'static str,
}

fn cards() -> Vec<Card> {
    vec![
        Card {
            verdict: "MEASURED",
            badge: "badge-green",
            figure: "7.63 tok/s · 27B on Omega",
            title: "A 27B model answers through our own server",
            desc: "Qwen3.8-27B (FP8) served by aien-serve-http on our own Omega GPU kernels, on the Spark. 131.05 ms per token, the median of 4 runs of 256 greedy tokens, 0.752 of the 10.15 tok/s memory ceiling. The old binary in the same folder did 6.29.",
            date: "2026-10-19",
            environment: "Spark GB10, port 18091, branch qwen38/serve-fast",
            gate: "Live-server decode receipt",
            proves: "The 27B answers through our own server and kernels at this speed, with the runs published beside the ceiling they sit under.",
            not_proves: "That the number is on main yet (it is on a branch), or anything about any other machine.",
            url: "https://github.com/aien-dev/aien-sovereign-core-mv/tree/qwen38/serve-fast/evidence/qwen38-serve-fast-20261019",
            source: "aien-sovereign-core-mv · qwen38/serve-fast",
        },
        Card {
            verdict: "FAIL AS WRITTEN",
            badge: "badge-amber",
            figure: "install.sh exits 101",
            title: "A stranger follows the alpha guide",
            desc: "A fresh Linux account followed the public guide to install and run the 27B server. The install script failed at the server build step. With a hand workaround, the remaining steps passed: 81 of 81 checksums, the gate, and 7.657 tok/s with answers matching the shipped reference.",
            date: "2026-10-19",
            environment: "Spark GB10, new user, branch alpha/fresh-account-evidence",
            gate: "Fresh-account alpha test",
            proves: "The guide is almost there, and exactly where it breaks is on the record with the fix named.",
            not_proves: "That anyone can install from the guide today. The verdict is FAIL until the script is fixed and the test is run again.",
            url: "https://github.com/aien-dev/aien-sovereign-core-mv/tree/alpha/fresh-account-evidence/evidence/alpha-fresh-account-20261019",
            source: "aien-sovereign-core-mv · alpha/fresh-account-evidence",
        },
        Card {
            verdict: "PASS",
            badge: "badge-green",
            figure: "24 of 24 emulator gates",
            title: "Every kernel gate, in QEMU, on main",
            desc: "The AIENOS C kernel passes all 24 emulator gates at commit 7f0de7e, each one a script anyone can rerun, with the receipt on main. The gate script now lists 26, so a newer receipt is owed.",
            date: "2026-10-09",
            environment: "QEMU, AArch64 virt and x86_64 q35, on main",
            gate: "ck_gates.sh, 24 gates",
            proves: "The kernel reaches every gated stage under emulation at the recorded commit.",
            not_proves: "Anything physical. QEMU qualifies nothing physical, by the repository's own words.",
            url: "https://github.com/aien-dev/aienos-mv/blob/main/evidence/ck_gates_29ed03c97454354a18676f768273dd7f7a8e15462c4be1939d9994d277d0d639.json",
            source: "aienos-mv · evidence on main",
        },
        Card {
            verdict: "MERGED",
            badge: "badge-blue",
            figure: "Four cores on a Raspberry Pi 5",
            title: "The kernel on real hardware",
            desc: "The AArch64 C kernel ran on a Raspberry Pi 5: interrupts, all four cores, user-mode tasks, scheduling, message passing, and a log written to the SD card and read back. Pull requests #345 and #346, evidence on main.",
            date: "2026-10-13",
            environment: "Raspberry Pi 5, BCM2712, SD image per run",
            gate: "Pi 5 hardware bring-up runs",
            proves: "The kernel's core services run on physical hardware, not only in the emulator.",
            not_proves: "A resident kernel. Each run writes a fresh SD image; release qualification (PI5-8) is still open.",
            url: "https://github.com/aien-dev/aienos-mv/blob/main/evidence/pi5_hardware_runs_2026-10-13",
            source: "aienos-mv · evidence on main",
        },
        Card {
            verdict: "MERGED",
            badge: "badge-blue",
            figure: "Allen's window on an iMac",
            title: "The x86 port boots bare metal",
            desc: "The x86_64 port of the C kernel booted an Intel iMac 2017: all cores started, both disks read, Ethernet seen. The whole screen is Allen's window. The kernel the iMac runs today sits in the open pull request stack (#414 to #424).",
            date: "2026-10-17",
            environment: "Intel iMac 2017, bare metal, netboot from the Spark",
            gate: "iMac batch run, evidence/imac_kernel1_2026-10-17",
            proves: "The port runs on the physical machine with its cores, disks and network found.",
            not_proves: "That today's iMac kernel is on main. The evidence is; the newest kernel is in open pull requests.",
            url: "https://github.com/aien-dev/aienos-mv/blob/main/evidence/imac_kernel1_2026-10-17",
            source: "aienos-mv · evidence on main",
        },
        Card {
            verdict: rs::ty2_status(),
            badge: "badge-green",
            figure: rs::ty2_headline(),
            title: rs::ty2_title(),
            desc: rs::ty2_plain(),
            date: rs::ty2_date(),
            environment: "Omega verification suite, turing-yield CI on main",
            gate: rs::ty2_receipt(),
            proves: rs::ty2_scope(),
            not_proves: rs::ty2_limitations(),
            url: "https://github.com/aien-dev/omega/pull/88",
            source: "aien-dev/omega · PR #88",
        },
    ]
}

pub fn render_evidence() -> Markup {
    let cards = cards();
    html! {
        section id="evidence" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle); background: var(--bg-surface);" {
            div class="container" {
                div style="margin-bottom: 40px; max-width: 760px;" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "PROOF, NOT PROMISES"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Every claim carries a receipt"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; line-height: 1.6;" {
                        "This project does not ask to be believed. Each card names its environment, its gate, its date, and its verdict, and states plainly what the receipt proves and what it does not prove. That includes the failures, especially the failures."
                    }
                }

                div class="responsive-card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 24px;" {
                    @for c in &cards {
                        div class="glow-box" style="padding: 28px; border-radius: 8px; display: flex; flex-direction: column; gap: 14px;" {
                            div {
                                span class=(format!("badge {}", c.badge)) style="font-size: 10px;" {
                                    (c.verdict)
                                }
                            }
                            div class="tabular-nums" style="font-size: 26px; font-weight: 800; color: var(--text-primary); font-family: var(--font-mono); line-height: 1.2;" {
                                (c.figure)
                            }
                            h3 style="font-size: 17px; font-weight: 700; color: var(--text-primary);" {
                                (c.title)
                            }
                            p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                (super::omega::link_first(c.desc))
                            }
                            div style="font-size: 13.5px; line-height: 1.6; display: flex; flex-direction: column; gap: 6px;" {
                                p style="margin: 0; color: var(--text-secondary);" {
                                    span style="color: var(--accent-green); font-weight: 700;" { "Proves: " }
                                    (c.proves)
                                }
                                p style="margin: 0; color: var(--text-secondary);" {
                                    span style="color: #f5a623; font-weight: 700;" { "Does not prove: " }
                                    (c.not_proves)
                                }
                            }
                            div style="margin-top: auto; padding-top: 14px; border-top: 1px solid var(--border-subtle); font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); display: flex; flex-direction: column; gap: 5px;" {
                                span { "DATE " (c.date) }
                                span { "ENVIRONMENT " (c.environment) }
                                span { "GATE " (c.gate) }
                                span { "RECEIPT " (c.source) }
                            }
                            (foot(c.url, Target::Evidence))
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_are_fresh_linked_and_clean() {
        let cs = cards();
        assert_eq!(cs.len(), 6);
        for c in &cs {
            assert!(c.url.starts_with("https://github.com/aien-dev/"), "{}: {}", c.title, c.url);
            for s in [c.figure, c.title, c.desc, c.proves, c.not_proves, c.environment, c.gate] {
                assert!(!s.contains('\u{2014}') && !s.contains('\u{2013}'), "dash in {s:?}");
            }
        }
        let dates: Vec<&str> = cs.iter().map(|c| c.date).collect();
        assert!(dates.iter().any(|d| d.starts_with("2026-10-19")), "newest evidence must be on the cards");
        let page = render_evidence().into_string();
        for must in ["7.63 tok/s", "FAIL AS WRITTEN", "24 of 24", "Raspberry Pi 5", "iMac", "install.sh exits 101"] {
            assert!(page.contains(must), "missing {must}");
        }
        let lower = page.to_ascii_lowercase();
        for w in crate::components::machines::FORBIDDEN {
            assert!(!lower.contains(w), "forbidden word {w}");
        }
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(6), "every receipt card links to GitHub");
    }
}
