use maud::{html, Markup, PreEscaped};

pub fn render_ecosystem() -> Markup {
    let projects = [
        (
            "aien-sovereign-core",
            "AIEN Sovereign Core",
            "Core native agent runtime, fail-closed safety engine, Axum web gateway, and custom Modular MAX model architectures.",
            "Rust 1.85+ / MAX 26.5",
            "Core Engine",
            "https://github.com/aien-dev/aien-sovereign-core"
        ),
        (
            "spark-neural-os",
            "Spark Neural OS",
            "GPU-native neural operating system, resident cognitive kernel, and pure Mojo 1.0 SIMD vector kernels.",
            "Mojo 1.0 / MLIR",
            "Kernel Stack",
            "https://github.com/aien-dev"
        ),
        (
            "spark-cockpit",
            "AIEN Universal Cockpit",
            "Ultra-low-latency Axum HTTP/SSE gateway and PWA console delivering real-time telemetry, reasoning logs, and vault inspection.",
            "Rust / Axum",
            "12.9 MB RSS",
            "https://github.com/aien-dev"
        ),
        (
            "spark-rsi",
            "Spark RSI Engine",
            "Recursive Self-Improvement engine with TPM ECDSA-signed cryptographic ledger (.rsi/ledger.db) and dual-jail container isolation.",
            "Rust / TPM 2.0",
            "Cryptographic",
            "https://github.com/aien-dev/spark-rsi"
        ),
        (
            "openclaw-rs",
            "OpenClaw Sovereign Host",
            "Ultra-high-performance process supervisor and event dispatcher operating under 5MB resident set size.",
            "Rust",
            "4.56 MB RSS",
            "https://github.com/aien-dev/openclaw-rs"
        ),
        (
            "cortex-rs",
            "Cortex Canonical Memory",
            "Canonical memory and vector storage engine with SQLite WAL persistence, sub-millisecond search, and graph traversal.",
            "Rust / SQLite",
            "19k req/s",
            "https://github.com/aien-dev/cortex-rs"
        ),
    ];

    html! {
        section id="ecosystem" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle); background: var(--bg-surface);" {
            div class="container" {
                div style="margin-bottom: 48px;" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "OPEN SOURCE SUBSYSTEMS"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "The AIEN Sovereign Ecosystem"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; max-width: 680px;" {
                        "A unified network of pure compiled Rust and Mojo repositories operating under the Sovereign Reciprocal Commons License."
                    }
                </div>

                div class="responsive-card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 24px;" {
                    @for (_repo, name, desc, lang, badge, url) in projects {
                        div class="glow-box" style="padding: 28px; border-radius: 8px; display: flex; flex-direction: column; justify-content: space-between; gap: 20px;" {
                            div {
                                div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;" {
                                    span class="badge badge-blue" style="font-size: 10px;" {
                                        (badge)
                                    }
                                    a href=(url) target="_blank" rel="noopener noreferrer" aria-label={ "Visit " (name) } style="color: var(--text-muted); transition: color 0.15s ease;" {
                                        (PreEscaped(r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6"></path><path d="M10 14 21 3"></path><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path></svg>"#))
                                    }
                                }
                                h3 style="font-size: 18px; font-weight: 700; color: var(--text-primary); margin-bottom: 8px;" {
                                    a href=(url) target="_blank" rel="noopener noreferrer" style="color: inherit;" {
                                        (name)
                                    }
                                }
                                p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                    (desc)
                                }
                            }
                            div style="display: flex; align-items: center; justify-content: space-between; padding-top: 16px; border-top: 1px solid var(--border-subtle); font-family: var(--font-mono); font-size: 12px; color: var(--text-muted);" {
                                span { (lang) }
                                a href=(url) target="_blank" rel="noopener noreferrer" style="color: var(--accent-green); font-weight: 600;" {
                                    "Inspect Code →"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
