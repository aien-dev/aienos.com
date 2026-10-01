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
    let cards = [
        (
            "PASS",
            "badge-green",
            "230 checks · 0 failures",
            "Gate 14: the foundation closes",
            "The combined foundation receipt for Omega and Physics, recomputed leg by leg and admitted on main. Two repositories, one verdict, nothing taken on trust.",
            "2026-09-30",
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
            "https://github.com/aien-dev/omega/pull/88",
            "aien-dev/omega · PR #88",
        ),
        (
            "MERGED",
            "badge-blue",
            "C kernel boots via UEFI",
            "The native kernel, on main",
            "A freestanding C kernel core boots via UEFI, with NVMe, capability security with ARGUS, and a sealed Store in the boot path (PR #195, same morning). Staged in emulation first on purpose: the hardware path stays closed until it qualifies.",
            "2026-10-01",
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
                        "This project does not ask to be believed. Each card below links to the public record it came from: the merge, the receipt, the number. That includes the failures, especially the failures."
                    }
                }

                div class="responsive-card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 24px;" {
                    @for (verdict, badge_class, figure, title, desc, date, url, source) in &cards {
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
                            p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0; flex: 1;" {
                                (desc)
                            }
                            div style="padding-top: 14px; border-top: 1px solid var(--border-subtle); font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); display: flex; justify-content: space-between; gap: 12px; flex-wrap: wrap;" {
                                span { "DATE " (date) }
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
