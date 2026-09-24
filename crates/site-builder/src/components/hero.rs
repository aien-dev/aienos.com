use maud::{html, Markup, PreEscaped};

pub fn render_hero() -> Markup {
    let install_cmd = "curl -fsSL https://aienos.com/install.sh | bash";

    html! {
        section class="bg-grid" style="padding: 80px 0 64px 0; border-bottom: 1px solid var(--border-subtle); position: relative;" {
            div class="container" {
                div style="max-width: 880px; margin: 0 auto; text-align: center;" {
                    div style="display: flex; justify-content: center; gap: 10px; margin-bottom: 24px; flex-wrap: wrap;" {
                        span class="badge badge-green" {
                            span style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-green);" {}
                            "NVIDIA GB10 COHERENT"
                        }
                        span class="badge" style="border: 1px solid rgba(245, 158, 11, 0.4); color: #f5a623; background: rgba(245, 158, 11, 0.08);" {
                            span style="width: 6px; height: 6px; border-radius: 50%; background: #f5a623;" {}
                            "IN ACTIVE DEVELOPMENT"
                        }
                        span class="badge badge-blue" {
                            (PreEscaped(r#"<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>"#))
                            "PURE COMPILED RUST + MOJO"
                        }
                        span class="badge badge-blue" {
                            (PreEscaped(r#"<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path><path d="m9 12 2 2 4-4"></path></svg>"#))
                            "HARDWARE TPM 2.0"
                        }
                    }

                    h1 style="font-size: clamp(36px, 5vw, 64px); font-weight: 800; line-height: 1.1; letter-spacing: -0.03em; margin-bottom: 24px; color: var(--text-primary);" {
                        "The GPU-Native Neural Operating Environment"
                    }

                    p style="font-size: clamp(16px, 2vw, 20px); color: var(--text-secondary); line-height: 1.6; max-width: 760px; margin: 0 auto 16px auto;" {
                        "Replacing the 40-year CPU von Neumann bottleneck with coherent unified memory, in-process cognitive kernels, sub-5MB daemon footprints, and hardware-enforced sovereign security."
                    }

                    p style="font-size: 15px; color: var(--text-secondary); margin-bottom: 40px;" {
                        "AIENOS is in active development. "
                        a href="#waitlist" style="color: var(--accent-green); font-weight: 600;" {
                            "AIEN is coming soon, join the waitlist."
                        }
                    }

                    div class="install-command" style="background: var(--bg-surface); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 16px 20px; display: flex; align-items: center; justify-content: space-between; max-width: 680px; margin: 0 auto 48px auto; box-shadow: 0 12px 36px rgba(0, 0, 0, 0.4);" {
                        div style="display: flex; align-items: center; gap: 12px; font-family: var(--font-mono); font-size: 14px; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" {
                            span style="color: var(--accent-green); user-select: none;" { "$" }
                            span id="hero-cmd-text" { (install_cmd) }
                        }
                        button type="button" id="hero-copy-btn" aria-label="Copy the AIEN OS install command" onclick="navigator.clipboard.writeText('curl -fsSL https://aienos.com/install.sh | bash').then(() => { this.innerText = 'COPIED'; setTimeout(() => this.innerText = 'COPY', 2000); })" style="background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 6px; padding: 8px 14px; display: flex; align-items: center; gap: 8px; cursor: pointer; font-family: var(--font-mono); font-size: 12px; font-weight: 600; transition: all 0.15s ease;" {
                            span { "COPY" }
                        }
                    }

                    div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; text-align: left;" {
                        div class="glow-box" style="padding: 18px 20px; border-radius: 8px;" {
                            div style="font-size: 11px; font-family: var(--font-mono); color: var(--text-muted); margin-bottom: 4px;" {
                                "RESIDENT MEMORY (RSS)"
                            }
                            div class="tabular-nums" style="font-size: 24px; font-weight: 700; color: var(--accent-green); font-family: var(--font-mono);" {
                                "4.56 MB"
                            }
                            div style="font-size: 12px; color: var(--text-secondary); margin-top: 2px;" {
                                "-89.8% vs FastAPI baseline"
                            }
                        }

                        div class="glow-box" style="padding: 18px 20px; border-radius: 8px;" {
                            div style="font-size: 11px; font-family: var(--font-mono); color: var(--text-muted); margin-bottom: 4px;" {
                                "SIMD VECTOR SPEED"
                            }
                            div class="tabular-nums" style="font-size: 24px; font-weight: 700; color: var(--accent-blue); font-family: var(--font-mono);" {
                                "124.2 GFLOP/s"
                            }
                            div style="font-size: 12px; color: var(--text-secondary); margin-top: 2px;" {
                                "Native Mojo 32-lane vector"
                            }
                        }

                        div class="glow-box" style="padding: 18px 20px; border-radius: 8px;" {
                            div style="font-size: 11px; font-family: var(--font-mono); color: var(--text-muted); margin-bottom: 4px;" {
                                "COHERENT BANDWIDTH"
                            }
                            div class="tabular-nums" style="font-size: 24px; font-weight: 700; color: var(--accent-blue); font-family: var(--font-mono);" {
                                "81.6 GB/s"
                            }
                            div style="font-size: 12px; color: var(--text-secondary); margin-top: 2px;" {
                                "Zero-copy GPU-CPU unified"
                            }
                        }

                        div class="glow-box" style="padding: 18px 20px; border-radius: 8px;" {
                            div style="font-size: 11px; font-family: var(--font-mono); color: var(--text-muted); margin-bottom: 4px;" {
                                "SECURITY INVARIANT"
                            }
                            div style="font-size: 24px; font-weight: 700; color: var(--accent-green); font-family: var(--font-mono);" {
                                "TPM 2.0"
                            }
                            div style="font-size: 12px; color: var(--text-secondary); margin-top: 2px;" {
                                "Zero plaintext disk secrets"
                            }
                        }
                    }
                }
            }
        }
    }
}
