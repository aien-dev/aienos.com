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
                                "AIEN OS"
                            }
                        }
                        p style="line-height: 1.6; color: var(--text-secondary);" {
                            "The GPU-Native Neural Operating Environment. Built for NVIDIA Grace Blackwell GB10, Apple Silicon, and commodity Linux architectures."
                        }
                    }

                    div style="display: flex; gap: 48px; flex-wrap: wrap;" {
                        div {
                            div style="font-weight: 600; color: var(--text-primary); margin-bottom: 12px; font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.05em;" {
                                "ECOSYSTEM"
                            }
                            ul style="list-style: none; display: flex; flex-direction: column; gap: 8px;" {
                                li { a href="https://github.com/aien-dev/aien-sovereign-core" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "AIEN Sovereign Core" } }
                                li { a href="https://github.com/aien-dev/spark-rsi" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Spark RSI Engine" } }
                                li { a href="https://github.com/aien-dev/benchmarks" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Performance Benchmarks" } }
                                li { a href="https://github.com/aien-dev/openclaw-rs" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "OpenClaw Supervisor" } }
                            }
                        }

                        div {
                            div style="font-weight: 600; color: var(--text-primary); margin-bottom: 12px; font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.05em;" {
                                "SOVEREIGN NETWORK"
                            }
                            ul style="list-style: none; display: flex; flex-direction: column; gap: 8px;" {
                                li { a href="https://www.drakestapleton.com" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "Drake Stapleton" } }
                                li { a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="color: var(--text-secondary);" { "GitHub Organization" } }
                                li { a href="https://aienos.com/install.sh" style="color: var(--accent-green); font-family: var(--font-mono);" { "install.sh" } }
                            }
                        }
                    }
                }

                div style="border-top: 1px solid var(--border-subtle); padding-top: 24px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; font-family: var(--font-mono); font-size: 11px;" {
                    div {
                        "AIEN Sovereign Commoners © 2026. Licensed under Apache-2.0 and SRCL-1.0."
                    }
                    div style="display: flex; gap: 16px;" {
                        span { "SECURE_TPM_ONLY" }
                        span { "ZERO_UNSOLICITED_TELEMETRY" }
                    }
                }
            }
        }
    }
}
