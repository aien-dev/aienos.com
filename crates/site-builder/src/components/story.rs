use maud::{html, Markup, PreEscaped};

pub fn render_story() -> Markup {
    let steps = [
        ("ATLAS", "awakens"),
        ("AIENOS", "persists"),
        ("OMEGA", "verifies"),
        ("AIEN", "learns"),
        ("EVIDENCE", "teaches"),
    ];

    html! {
        section id="story" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="max-width: 760px; margin-bottom: 36px;" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "WHAT THIS IS"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Not another app on someone else's computer"
                    }
                    p style="color: var(--text-secondary); margin-top: 16px; line-height: 1.7; font-size: 16px;" {
                        "Most AI lives in a rented stack. A provider owns the machine, the model, the price, and the off switch. Your agent exists at their pleasure, forgets on their schedule, and works under their terms."
                    }
                    p style="color: var(--text-secondary); margin-top: 14px; line-height: 1.7; font-size: 16px;" {
                        "AIENOS is the opposite bet: a sovereign machine, built from the first instruction after firmware upward, where a persistent intelligence lives on hardware you own. It keeps its memory, its identity, and its receipts on your side of the line. The hardware underneath is capability, not identity: machines can be replaced, and the lineage persists."
                    }
                    p style="color: var(--text-secondary); margin-top: 14px; line-height: 1.7; font-size: 16px;" {
                        "Four names carry the work, and one loop ties them together:"
                    }
                }

                div style="display: flex; align-items: stretch; gap: 10px; flex-wrap: wrap;" {
                    @for (i, (name, verb)) in steps.iter().enumerate() {
                        div style="flex: 1 1 150px; background: var(--bg-surface); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 18px 16px; text-align: center;" {
                            div style="font-family: var(--font-mono); font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: 0.06em;" {
                                (name)
                            }
                            div style="font-size: 13px; color: var(--accent-green); margin-top: 2px;" {
                                (verb)
                            }
                        }
                        @if i + 1 < steps.len() {
                            div aria-hidden="true" style="display: flex; align-items: center; color: var(--text-muted); font-family: var(--font-mono);" {
                                (PreEscaped("→"))
                            }
                        }
                    }
                }
                p style="color: var(--text-muted); font-size: 13px; margin-top: 14px; font-family: var(--font-mono);" {
                    "and the loop closes: what the evidence teaches becomes the next thing AIEN knows."
                }
            }
        }
    }
}
