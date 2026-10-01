use maud::{html, Markup};

pub fn render_hero() -> Markup {
    html! {
        section class="bg-grid" style="padding: 88px 0 72px 0; border-bottom: 1px solid var(--border-subtle); position: relative;" {
            div class="container" {
                div style="max-width: 880px; margin: 0 auto; text-align: center;" {
                    div style="display: flex; justify-content: center; gap: 10px; margin-bottom: 24px; flex-wrap: wrap;" {
                        span class="badge" style="border: 1px solid rgba(245, 158, 11, 0.4); color: #f5a623; background: rgba(245, 158, 11, 0.08);" {
                            span style="width: 6px; height: 6px; border-radius: 50%; background: #f5a623;" {}
                            "EXPERIMENTAL · PRE-ALPHA"
                        }
                        span class="badge badge-blue" {
                            "OPEN SOURCE · APACHE-2.0 WITH LLVM-EXCEPTION"
                        }
                    }

                    h1 style="font-size: clamp(36px, 5vw, 64px); font-weight: 800; line-height: 1.1; letter-spacing: -0.03em; margin-bottom: 24px; color: var(--text-primary);" {
                        "Persistent machine intelligence, on a machine you own."
                    }

                    p style="font-size: clamp(16px, 2vw, 20px); color: var(--text-secondary); line-height: 1.6; max-width: 760px; margin: 0 auto 14px auto;" {
                        "AIENOS boots the machine. Omega turns intent into verified programs. AIEN learns from what survives contact with reality."
                    }

                    p style="font-size: 17px; color: var(--text-primary); font-weight: 600; margin-bottom: 40px;" {
                        "Intelligence should be property, not rent."
                    }

                    div style="display: flex; justify-content: center; gap: 12px; flex-wrap: wrap; margin-bottom: 44px;" {
                        a href="#evidence" style="background: var(--accent-green); color: var(--bg-base); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 14px; font-weight: 700;" {
                            "See the evidence"
                        }
                        a href="#architecture" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 14px; font-weight: 600;" {
                            "Explore the architecture"
                        }
                        a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 12px 22px; font-family: var(--font-mono); font-size: 14px; font-weight: 600;" {
                            "GitHub"
                        }
                    }

                    div style="font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); letter-spacing: 0.04em; display: flex; justify-content: center; gap: 28px; flex-wrap: wrap;" {
                        span { "C KERNEL BOOTS VIA UEFI ✓" }
                        span { "EVERY CLAIM CARRIES A RECEIPT ✓" }
                        span { "FAILURES PUBLISHED, NOT HIDDEN ✓" }
                    }
                }
            }
        }
    }
}
