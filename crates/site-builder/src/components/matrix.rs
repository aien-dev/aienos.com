use maud::{html, Markup};

pub fn render_matrix() -> Markup {
    let roles = [
        (
            "ATLAS",
            "Awakens",
            "The seed that wakes AIENOS on a machine. Atlas carries the lineage to new hardware and brings the sovereign stack to life on it.",
        ),
        (
            "AIENOS",
            "Persists",
            "The sovereign substrate, and more than an operating system: it owns the hardware, keeps the agent alive and continuous, and holds memory, keys, and receipts on the owner's side of the line.",
        ),
        (
            "OMEGA",
            "Defines · synthesizes · verifies",
            "Turns intent into programs, then proves them against reality before they are trusted. Nothing runs on a claim; everything runs on a check.",
        ),
        (
            "AIEN",
            "Learns",
            "The persistent intelligence itself. It learns from verified experience: what survived Omega's checks and contact with reality becomes knowledge. Weights suggest, programs explain, evidence teaches.",
        ),
    ];

    let support = [
        ("AEGIS", "Verification only. It checks generated code, changes nothing, and decides no policy."),
        ("FORGE", "Realization. It builds what Omega verified into running form."),
        ("ARGUS", "The watcher. Capability enforcement inside the native kernel."),
        ("CORTEX", "Canonical memory. The machine's long-term record of verified experience."),
    ];

    html! {
        section id="architecture" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="margin-bottom: 40px; max-width: 760px;" {
                    div class="badge badge-blue" style="margin-bottom: 12px;" {
                        "HOW IT FITS TOGETHER"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Four roles, one loop"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; line-height: 1.6;" {
                        "Hardware is treated as capability metadata, never as identity. The identity is the lineage: who wakes the machine, what keeps it alive, what checks its work, and what learns from the results."
                    }
                }

                div class="responsive-card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 24px; margin-bottom: 24px;" {
                    @for (name, verb, desc) in &roles {
                        div class="glow-box" style="padding: 28px; border-radius: 8px; display: flex; flex-direction: column; gap: 10px;" {
                            div style="font-family: var(--font-mono); font-size: 18px; font-weight: 700; color: var(--text-primary); letter-spacing: 0.06em;" {
                                (name)
                            }
                            div style="font-size: 13px; color: var(--accent-green); font-weight: 600;" {
                                (verb)
                            }
                            p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                (desc)
                            }
                        }
                    }
                }

                div style="background: var(--bg-surface); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 24px 28px;" {
                    div style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); letter-spacing: 0.08em; margin-bottom: 16px;" {
                        "AROUND THE LOOP"
                    }
                    div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 20px;" {
                        @for (name, desc) in &support {
                            div {
                                span style="font-family: var(--font-mono); font-size: 13px; font-weight: 700; color: var(--text-primary); letter-spacing: 0.05em;" {
                                    (name)
                                }
                                p style="color: var(--text-secondary); font-size: 13px; line-height: 1.55; margin: 4px 0 0 0;" {
                                    (desc)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
