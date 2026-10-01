use maud::{html, Markup, PreEscaped};

pub fn render_matrix() -> Markup {
    // The lineage is a pipeline of four roles. AEGIS, FORGE, ARGUS, and
    // CORTEX are NOT further stages in it: they stand around the loop,
    // verifying, realizing, observing, and remembering. The layout keeps
    // that distinction visible: one chain with arrows, then a separate
    // dashed band underneath.
    let roles = [
        (
            "ATLAS",
            "awakens",
            "The seed that wakes AIENOS on a machine. Atlas carries the lineage to new hardware and brings the sovereign stack to life on it.",
        ),
        (
            "AIENOS",
            "persists",
            "The sovereign substrate, and more than an operating system: it owns the hardware, keeps the agent alive and continuous, and holds memory, keys, and receipts on the owner's side of the line.",
        ),
        (
            "OMEGA",
            "defines · synthesizes · verifies",
            "Turns intent into programs, then proves them against reality before they are trusted. Nothing runs on a claim; everything runs on a check.",
        ),
        (
            "AIEN",
            "learns",
            "The persistent intelligence itself. It learns from verified experience: what survived Omega's checks and contact with reality becomes knowledge. Weights suggest, programs explain, evidence teaches.",
        ),
    ];

    let support = [
        ("AEGIS", "verifies", "Checks generated code. Changes nothing, decides no policy, belongs to no generation system."),
        ("FORGE", "realizes", "Builds what Omega verified into running form."),
        ("ARGUS", "observes", "The watcher. Capability enforcement inside the native kernel, on the record."),
        ("CORTEX", "remembers", "Canonical memory: the machine's long-term record of verified experience."),
    ];

    html! {
        section id="architecture" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="margin-bottom: 40px; max-width: 760px;" {
                    div class="badge badge-blue" style="margin-bottom: 12px;" {
                        "HOW IT FITS TOGETHER"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "One pipeline, four roles"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; line-height: 1.6;" {
                        "Hardware is treated as capability metadata, never as identity. The identity is the lineage: who wakes the machine, what keeps it alive, what checks its work, and what learns from the results."
                    }
                }

                div style="display: flex; align-items: stretch; gap: 10px; flex-wrap: wrap; margin-bottom: 14px;" {
                    @for (i, (name, verb, desc)) in roles.iter().enumerate() {
                        div class="glow-box" style="flex: 1 1 210px; padding: 26px; border-radius: 8px; display: flex; flex-direction: column; gap: 8px;" {
                            div style="font-family: var(--font-mono); font-size: 19px; font-weight: 700; color: var(--text-primary); letter-spacing: 0.06em;" {
                                (name)
                            }
                            div style="font-size: 13px; color: var(--accent-green); font-weight: 600;" {
                                (verb)
                            }
                            p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                (desc)
                            }
                        }
                        @if i + 1 < roles.len() {
                            div aria-hidden="true" style="display: flex; align-items: center; color: var(--accent-blue); font-family: var(--font-mono); font-size: 18px;" {
                                (PreEscaped("→"))
                            }
                        }
                    }
                }
                p style="color: var(--text-muted); font-size: 13px; font-family: var(--font-mono); margin-bottom: 36px;" {
                    "read it left to right: each role hands its work to the next, and the evidence loops back."
                }

                div style="border: 1px dashed var(--border-active); border-radius: 8px; padding: 26px 28px; background: rgba(45, 127, 249, 0.04);" {
                    div style="font-family: var(--font-mono); font-size: 11px; color: var(--accent-blue); letter-spacing: 0.08em; margin-bottom: 6px;" {
                        "AROUND THE LOOP, NOT IN THE PIPELINE"
                    }
                    p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0 0 18px 0; max-width: 720px;" {
                        "These four are not stages five through eight. Nothing flows through them in sequence. They stand around the pipeline and serve it: one verifies, one realizes, one observes, one remembers."
                    }
                    div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 20px;" {
                        @for (name, verb, desc) in &support {
                            div {
                                div {
                                    span style="font-family: var(--font-mono); font-size: 13px; font-weight: 700; color: var(--text-primary); letter-spacing: 0.05em;" {
                                        (name)
                                    }
                                    span style="font-size: 12.5px; color: var(--accent-blue); font-weight: 600; margin-left: 8px;" {
                                        (verb)
                                    }
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
