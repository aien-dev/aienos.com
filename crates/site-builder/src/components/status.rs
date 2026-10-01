use maud::{html, Markup};

pub fn render_status() -> Markup {
    let rows = [
        (
            "ALWAYS",
            "Experimental and pre-alpha. Nothing here is a finished product, and this page will not pretend otherwise.",
        ),
        (
            "2026-10-01",
            "The native C kernel landed on main and boots in emulation via UEFI. Hardware qualification is in progress, in the open.",
        ),
        (
            "2026-09-30",
            "First M5 qualification receipt: NOT QUALIFIED (10 blocked, 6 missing). Published, and being worked down in public.",
        ),
        (
            "NOW RUNNING",
            "The Turing validation experiments (EXP-001, EXP-002, EXP-003) proceed under frozen, published protocols. Milestones and failures both get receipts.",
        ),
        (
            "OPEN BY DEFAULT",
            "Research, receipts, and code are public as they land. If a claim on this page ever outruns its receipt, the receipt wins.",
        ),
    ];

    html! {
        section id="status" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="max-width: 760px; margin-bottom: 36px;" {
                    div class="badge badge-amber" style="margin-bottom: 12px;" {
                        "CURRENT STATUS"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Where it honestly stands"
                    }
                }

                div style="display: flex; flex-direction: column;" {
                    @for (when, text) in &rows {
                        div style="display: flex; gap: 20px; padding: 16px 0; border-top: 1px solid var(--border-subtle); align-items: baseline; flex-wrap: wrap;" {
                            span style="font-family: var(--font-mono); font-size: 12px; font-weight: 700; color: var(--accent-green); letter-spacing: 0.05em; min-width: 110px;" {
                                (when)
                            }
                            p style="color: var(--text-secondary); font-size: 15px; line-height: 1.6; margin: 0; flex: 1; min-width: 260px;" {
                                (text)
                            }
                        }
                    }
                }

                div style="display: flex; gap: 12px; flex-wrap: wrap; margin-top: 32px;" {
                    a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="background: var(--accent-green); color: var(--bg-base); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 700;" {
                        "Follow the work on GitHub"
                    }
                    a href="#research" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                        "Read the research"
                    }
                    a href="#waitlist" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                        "Join the waitlist"
                    }
                }
            }
        }
    }
}
