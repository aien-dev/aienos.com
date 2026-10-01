use maud::{html, Markup};

pub fn render_story() -> Markup {
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
                }
            }
        }
    }
}
