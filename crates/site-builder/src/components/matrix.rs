use maud::{html, Markup, PreEscaped};

pub fn render_matrix() -> Markup {
    let pillars = [
        (
            r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="16" x="4" y="4" rx="2"></rect><rect width="6" height="6" x="9" y="9" rx="1"></rect><path d="M15 2v2"></path><path d="M15 20v2"></path><path d="M2 15h2"></path><path d="M2 9h2"></path><path d="M20 15h2"></path><path d="M20 9h2"></path><path d="M9 2v2"></path><path d="M9 20v2"></path></svg>"#,
            "Coherent Unified Address Space",
            "Grace Blackwell GB10 pairs 20 ARM Cortex cores with Blackwell GPU silicon across 128 GB coherent LPDDR5x memory. Tensors and page tables are manipulated in-place with zero host-to-device PCIe copy penalty."
        ),
        (
            r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-green)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>"#,
            "Pure Native Compiled Systems",
            "Zero Python or Node interpreters in core agent services or background daemons. Rust handles high-throughput asynchronous networking and process isolation; Mojo handles accelerated SIMD mathematics and PagedAttention KV-caching."
        ),
        (
            r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-amber)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path><path d="m9 12 2 2 4-4"></path></svg>"#,
            "Hardware TPM 2.0 Key Vault",
            "Strict SECURE_TPM_ONLY policy bound to /dev/tpmrm0 ECDSA P-256 hardware. Plaintext .env secret files are strictly forbidden on disk. Keys resolve dynamically in-memory with real-time redaction interceptors."
        ),
        (
            r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" y1="12" x2="2" y2="12"></line><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"></path><line x1="6" y1="16" x2="6.01" y2="16"></line><line x1="10" y1="16" x2="10.01" y2="16"></line></svg>"#,
            "Canonical Memory Engine",
            "High-performance SQLite WAL storage engine operating at 0.50 ms p50 latency and 19,000 req/s. Retains verified operational post-mortems and structured lessons without memory bloat."
        ),
        (
            r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-green)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#,
            "Neural Syscall Dispatcher",
            "Replaces legacy POSIX interruptions with semantic dispatch: sys_telemetry, sys_alloc_kv, sys_simd_benchmark, and sys_vault, routing context directly to hardware-resident cognitive models."
        ),
        (
            r#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="18" r="3"></circle><circle cx="6" cy="6" r="3"></circle><path d="M6 21V9a9 9 0 0 0 9 9"></path></svg>"#,
            "Sovereign Dual-Remote Git Sync",
            "Autonomous change-management and recursive self-improvement pipelines sync commits to both public remotes and self-hosted Forgejo repositories, ensuring complete engineering sovereignty."
        ),
    ];

    html! {
        section id="architecture" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="margin-bottom: 48px;" {
                    div class="badge badge-blue" style="margin-bottom: 12px;" {
                        "HARDWARE-GROUNDED DESIGN"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Architecture: Beyond the von Neumann Bottleneck"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; max-width: 700px;" {
                        "Engineered for physical hardware reality. High-efficiency systems design removing four decades of legacy computing debt."
                    }
                }

                div class="responsive-card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 24px;" {
                    @for (icon_svg, title, desc) in pillars {
                        div class="glow-box" style="padding: 32px; border-radius: 8px; display: flex; flex-direction: column; gap: 16px;" {
                            div style="width: 44px; height: 44px; border-radius: 6px; background: rgba(28, 37, 54, 0.4); display: flex; align-items: center; justify-content: center;" {
                                (PreEscaped(icon_svg))
                            }
                            h3 style="font-size: 18px; font-weight: 700; color: var(--text-primary);" {
                                (title)
                            }
                            p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                (desc)
                            }
                        }
                    }
                }
            }
        }
    }
}
