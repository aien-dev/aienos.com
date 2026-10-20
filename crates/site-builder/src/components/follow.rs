use maud::{html, Markup};

use super::boxlink::{foot, Target};

/// Replaces the waitlist. There is no release to wait for; there is work to follow.
pub fn render_follow() -> Markup {
    html! {
        section id="follow" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle); background: var(--bg-base);" {
            div class="container" {
                div class="glow-box" style="max-width: 760px; margin: 0 auto; padding: 48px 40px; border-radius: 12px; text-align: center; background: var(--bg-surface);" {
                    div class="badge badge-green" style="margin-bottom: 16px; display: inline-flex;" { "BUILT IN THE OPEN" }
                    h2 style="font-size: clamp(28px, 4vw, 40px); font-weight: 800; letter-spacing: -0.02em; color: var(--text-primary); margin-bottom: 16px;" {
                        "Follow the work."
                    }
                    p style="color: var(--text-secondary); line-height: 1.6; max-width: 600px; margin: 0 auto 32px auto; font-size: 16px;" {
                        "There is no release to wait for and no list to join. Everything happens in public repositories, every change is judged on the machine itself, and this page reads the record every hour. Watch the repositories, read the receipts, open an issue."
                    }
                    div style="display: flex; gap: 12px; flex-wrap: wrap; justify-content: center;" {
                        a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="background: var(--accent-green); color: var(--bg-base); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 13px; font-weight: 700;" {
                            "GitHub: aien-dev"
                        }
                        a href="https://github.com/aien-dev/aienix" target="_blank" rel="noopener noreferrer" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                            "AienOS, the core tree"
                        }
                        a href="/now/" style="background: transparent; color: var(--text-primary); border: 1px solid var(--accent-green); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                            "Now: what is being done today"
                        }
                        a href="/progress/" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                            "Project progress"
                        }
                    }
                    p style="color: var(--text-muted); font-size: 13px; margin-top: 28px; font-family: var(--font-mono);" {
                        "Machine-readable: "
                        a href="/status.json" style="color: var(--text-secondary);" { "status.json" }
                        " · "
                        a href="/facts.json" style="color: var(--text-secondary);" { "facts.json" }
                        " · "
                        a href="/llms.txt" style="color: var(--text-secondary);" { "llms.txt" }
                    }
                    div style="margin-top: 22px; display: flex; justify-content: center;" {
                        (foot("https://github.com/aien-dev", Target::Evidence))
                    }
                }
            }
        }
    }
}
