use maud::{html, Markup, PreEscaped};

pub fn render_footer() -> Markup {
    html! {
        footer style="border-top: 1px solid var(--border-subtle); background: var(--bg-base); padding: 48px 0 32px 0; font-size: 13px; color: var(--text-muted);" {
            div class="container" {
                div style="display: flex; justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 32px; margin-bottom: 40px;" {
                    div style="max-width: 380px;" {
                        div style="display: flex; align-items: center; gap: 8px; margin-bottom: 12px;" {
                            (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#))
                            span style="font-weight: 700; font-size: 16px; color: var(--text-primary);" {
                                "AIENOS"
                            }
                        }
                        p style="line-height: 1.6; color: var(--text-secondary);" {
                            "Persistent machine intelligence on hardware you own. Experimental, open source, and verified in public, receipts and all."
                        }
                    }

                    div style="display: flex; gap: 48px; flex-wrap: wrap;" {
                        div {
                            div style="font-weight: 600; color: var(--text-primary); margin-bottom: 12px; font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.05em;" {
                                "THE LINEAGE"
                            }
                            ul style="list-style: none; display: flex; flex-direction: column; gap: 8px;" {
                                li { a href="https://github.com/aien-dev/aienos" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "AIENOS" } }
                                li { a href="https://github.com/aien-dev/omega" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Omega" } }
                                li { a href="https://github.com/aien-dev/aien-architecture" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Architecture" } }
                                li { a href="https://github.com/aien-dev/physics" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Physics" } }
                            }
                        }

                        div {
                            div style="font-weight: 600; color: var(--text-primary); margin-bottom: 12px; font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.05em;" {
                                "SITE"
                            }
                            ul style="list-style: none; display: flex; flex-direction: column; gap: 8px;" {
                                li { a href="/research/" style="color: var(--text-secondary);" { "Research" } }
                                li { a href="/turing/" style="color: var(--text-secondary);" { "The Turing" } }
                                li { a href="/licensing/" style="color: var(--text-secondary);" { "Licensing" } }
                                li { a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "GitHub Organization" } }
                                li { a href="https://www.drakestapleton.com" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Drake Stapleton" } }
                            }
                        }
                    }
                }

                div style="border-top: 1px solid var(--border-subtle); padding-top: 24px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; font-family: var(--font-mono); font-size: 11px;" {
                    div {
                        "AIEN © 2026. Licensed under "
                        a href="/licensing/" style="color: var(--text-secondary); text-decoration: underline;" { "Apache-2.0 WITH LLVM-exception" }
                        "."
                    }
                    div style="display: flex; gap: 16px;" {
                        span { "RECEIPTS OVER CLAIMS" }
                        span { "ZERO_UNSOLICITED_TELEMETRY" }
                    }
                }
            }
        }
    }
}
