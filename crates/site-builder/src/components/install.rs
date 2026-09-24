use maud::{html, Markup, PreEscaped};

pub fn render_install() -> Markup {
    let methods = [
        (
            "Linux & macOS (Apple Silicon / Intel)",
            "curl -fsSL https://aienos.com/install.sh | bash",
            "Universal 1-line native installer. Detects platform architecture, compiles release binaries, and configures local runtime."
        ),
        (
            "Windows (PowerShell)",
            "git clone --depth 1 https://github.com/aien-dev/aien-sovereign-core.git; cd aien-sovereign-core; .\\install.ps1",
            "Checks out the source, configures the local runtime, and compiles native sovereign binaries."
        ),
        (
            "Build From Source (Cargo)",
            "git clone https://github.com/aien-dev/aien-sovereign-core.git && cd aien-sovereign-core && cargo build --release",
            "Full source inspection and compilation for custom architectures, specialized kernels, or air-gapped deployments."
        ),
    ];

    let dns_records = [
        ("A", "@", "185.199.108.153", "1800"),
        ("A", "@", "185.199.109.153", "1800"),
        ("A", "@", "185.199.110.153", "1800"),
        ("A", "@", "185.199.111.153", "1800"),
        ("AAAA", "@", "2606:50c0:8000::153", "1800"),
        ("AAAA", "@", "2606:50c0:8001::153", "1800"),
        ("AAAA", "@", "2606:50c0:8002::153", "1800"),
        ("AAAA", "@", "2606:50c0:8003::153", "1800"),
        ("CNAME", "www", "aien-dev.github.io.", "1800"),
    ];

    html! {
        section id="install" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="margin-bottom: 40px;" {
                    div class="badge badge-blue" style="margin-bottom: 12px;" {
                        "DEPLOYMENT & INSTALLATION"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Installation & Deployment"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; max-width: 680px;" {
                        "Install the complete sovereign ecosystem with zero configuration. Free and open source for all humanity."
                    }
                </div>

                div style="display: flex; flex-direction: column; gap: 20px; margin-bottom: 48px;" {
                    @for (idx, (os, cmd, desc)) in methods.iter().enumerate() {
                        @let method_idx = format!("METHOD 0{}", idx + 1);
                        @let copy_label = format!("Copy install command for {}", os);
                        @let copy_js = format!("navigator.clipboard.writeText('{}').then(() => {{ this.innerText = 'COPIED'; setTimeout(() => this.innerText = 'COPY', 2000); }})", cmd);
                        div class="glow-box" style="padding: 24px; border-radius: 8px;" {
                            div class="install-method-heading" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;" {
                                h3 style="font-size: 16px; font-weight: 700; color: var(--text-primary);" {
                                    (os)
                                }
                                span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);" {
                                    (method_idx)
                                }
                            }
                            p style="color: var(--text-secondary); font-size: 13px; line-height: 1.5; margin-bottom: 16px;" {
                                (desc)
                            }
                            div class="install-method-command" style="background: var(--bg-base); border: 1px solid var(--border-subtle); border-radius: 6px; padding: 12px 16px; display: flex; align-items: center; justify-content: space-between; gap: 12px;" {
                                span style="font-family: var(--font-mono); font-size: 13px; color: var(--accent-green); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" {
                                    (cmd)
                                }
                                button type="button" aria-label=(copy_label) onclick=(copy_js) style="background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 12px; font-family: var(--font-mono); font-size: 11px; font-weight: 600; cursor: pointer;" {
                                    "COPY"
                                }
                            }
                        }
                    }
                }

                // DNS Infrastructure Reference
                div style="margin-top: 48px;" {
                    div style="display: flex; align-items: center; gap: 8px; margin-bottom: 16px;" {
                        (PreEscaped(r#"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>"#))
                        h3 style="font-size: 16px; font-weight: 700; color: var(--text-primary);" {
                            "Sovereign DNS Invariant & Live Routing"
                        }
                    }
                    div style="overflow-x: auto; border: 1px solid var(--border-subtle); border-radius: 6px; background: var(--bg-base);" {
                        table style="width: 100%; border-collapse: collapse; text-align: left; font-family: var(--font-mono); font-size: 12px;" {
                            thead {
                                tr style="border-bottom: 1px solid var(--border-subtle); background: rgba(14, 18, 25, 0.9); color: var(--text-muted); font-size: 11px;" {
                                    th style="padding: 10px 16px;" { "TYPE" }
                                    th style="padding: 10px 16px;" { "NAME" }
                                    th style="padding: 10px 16px;" { "VALUE / DESTINATION" }
                                    th style="padding: 10px 16px;" { "TTL" }
                                }
                            }
                            tbody {
                                @for (idx, (typ, name, val, ttl)) in dns_records.iter().enumerate() {
                                    tr style=(if idx == dns_records.len() - 1 { "border-bottom: none;" } else { "border-bottom: 1px solid var(--border-subtle);" }) {
                                        td style="padding: 10px 16px; color: var(--accent-blue); font-weight: 600;" { (typ) }
                                        td style="padding: 10px 16px; color: var(--text-secondary);" { (name) }
                                        td style="padding: 10px 16px; color: var(--accent-green);" { (val) }
                                        td class="tabular-nums" style="padding: 10px 16px; color: var(--text-muted);" { (ttl) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
