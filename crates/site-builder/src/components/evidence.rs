use maud::{html, Markup};

pub fn render_evidence() -> Markup {
    // Every figure below traces to a merged public record, checked against
    // github.com/aien-dev on 2026-10-01 before publication:
    //   Gate 14:        aien-dev/omega PR #111, merged 2026-09-30 (CDT),
    //                   combined receipt binding Omega 8024e9a + Physics
    //                   e95e3ed, 230 checks, 0 failures.
    //   Turing yield:   aien-dev/omega PR #88, merged 2026-09-29,
    //                   TY-2 verify log re-verified, T = 2,559,679.825 bits.
    //   C kernel:       aien-dev/aienos PR #194 (UEFI boot, QEMU) and
    //                   PR #195 (NVMe, ARGUS, sealed Store), 2026-10-01.
    //   M5 NOT_QUALIFIED: aien-dev/aienos PR #187, merged 2026-09-30 (CDT),
    //                   first qualification receipt, blocked=10, missing=6.
    // Each card states what the receipt proves and what it does not prove,
    // so the card can be quoted without being misquoted.
    let cards = [
        (
            "PASS",
            "badge-green",
            "230 checks · 0 failures",
            "Gate 14: the foundation closes",
            "The combined foundation receipt for Omega and Physics, recomputed leg by leg and admitted on main. Two repositories, one verdict, nothing taken on trust.",
            "2026-09-30",
            "Omega + Physics repositories, combined on main",
            "Gate 14 combined foundation admission",
            "The foundation layer of Omega and Physics recomputes and agrees, end to end, at the recorded commits.",
            "That the native machine is qualified, or that any physical hardware gate has passed.",
            "https://github.com/aien-dev/omega/pull/111",
            "aien-dev/omega · PR #111",
        ),
        (
            "VERIFIED",
            "badge-blue",
            "T = 2,559,679.825 bits",
            "Turing yield, re-verified end to end",
            "The TY-2 verification log, checked again from the public record: a measured quantity of machine understanding in the project's own unit, reproducible by anyone.",
            "2026-09-29",
            "Omega verification suite, turing-yield CI on main",
            "TY-2 verify log re-verification",
            "The recorded Turing yield reproduces exactly from the public log under the frozen profile.",
            "Discovery. That is the job of the EXP-002 experiment series, which is still running.",
            "https://github.com/aien-dev/omega/pull/88",
            "aien-dev/omega · PR #88",
        ),
        (
            "MERGED",
            "badge-blue",
            "C kernel boots via UEFI",
            "The native kernel, on main",
            "A freestanding C kernel core boots via UEFI, with NVMe, capability security with ARGUS, and a sealed Store in the boot path (PR #195, same morning).",
            "2026-10-01",
            "QEMU emulation, UEFI boot path",
            "AIENOS_CK_M1 kernel boot",
            "The C kernel reaches its boot stages under emulation, with storage, capability checks, and the sealed Store in the path.",
            "Physical DGX Spark qualification. The hardware path stays closed until it qualifies; see the M5 card.",
            "https://github.com/aien-dev/aienos/pull/194",
            "aien-dev/aienos · PR #194",
        ),
        (
            "NOT QUALIFIED",
            "badge-amber",
            "10 blocked · 6 missing",
            "First M5 qualification attempt",
            "The first formal qualification of the native machine failed, and the receipt is published anyway. A meter you cannot fail is not a meter. This is what receipts mean here.",
            "2026-09-30",
            "Native machine qualification matrix",
            "M5 qualification",
            "The qualification process runs, measures, and reports honestly, with the gaps named and counted in public.",
            "That the approach failed. It marks exactly what remains to be built and checked.",
            "https://github.com/aien-dev/aienos/pull/187",
            "aien-dev/aienos · PR #187",
        ),
    ];

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
                    @for (verdict, badge_class, figure, title, desc, date, environment, gate, proves, not_proves, url, source) in &cards {
                        div class="glow-box" style="padding: 28px; border-radius: 8px; display: flex; flex-direction: column; gap: 14px;" {
                            div {
                                span class=(format!("badge {}", badge_class)) style="font-size: 10px;" {
                                    (verdict)
                                }
                            }
                            div class="tabular-nums" style="font-size: 26px; font-weight: 800; color: var(--text-primary); font-family: var(--font-mono); line-height: 1.2;" {
                                (figure)
                            }
                            h3 style="font-size: 17px; font-weight: 700; color: var(--text-primary);" {
                                (title)
                            }
                            p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                (desc)
                            }
                            div style="font-size: 13.5px; line-height: 1.6; display: flex; flex-direction: column; gap: 6px;" {
                                p style="margin: 0; color: var(--text-secondary);" {
                                    span style="color: var(--accent-green); font-weight: 700;" { "Proves: " }
                                    (proves)
                                }
                                p style="margin: 0; color: var(--text-secondary);" {
                                    span style="color: #f5a623; font-weight: 700;" { "Does not prove: " }
                                    (not_proves)
                                }
                            }
                            div style="padding-top: 14px; border-top: 1px solid var(--border-subtle); font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); display: flex; flex-direction: column; gap: 5px;" {
                                span { "DATE " (date) }
                                span { "ENVIRONMENT " (environment) }
                                span { "GATE " (gate) }
                                a href=(url) target="_blank" rel="noopener noreferrer" style="color: var(--accent-green); font-weight: 600;" {
                                    "RECEIPT: " (source) " →"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
