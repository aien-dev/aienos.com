use maud::{html, Markup, PreEscaped};

pub fn render_navbar() -> Markup {
    html! {
        header style="border-bottom: 1px solid var(--border-subtle); background: rgba(8, 9, 12, 0.85); backdrop-filter: blur(12px); position: sticky; top: 0; z-index: 50; padding: 16px 0;" {
            div class="container" style="display: flex; align-items: center; justify-content: space-between; gap: 16px;" {
                a href="#main-content" aria-label="AIEN OS home" style="display: flex; align-items: center; gap: 12px;" {
                    div style="width: 32px; height: 32px; borderRadius: 6px; background: linear-gradient(135deg, #131822 0%, #1e2638 100%); border: 1px solid var(--accent-blue); display: flex; align-items: center; justify-content: center; box-shadow: 0 0 12px var(--accent-blue-glow);" {
                        (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#))
                    }
                    div class="brand-copy" style="display: flex; flex-direction: column;" {
                        span style="font-weight: 700; font-size: 18px; letter-spacing: -0.02em; color: var(--text-primary);" {
                            "AIEN OS"
                        }
                        span style="font-size: 10px; font-family: var(--font-mono); color: var(--text-muted); letter-spacing: 0.08em;" {
                            "NEURAL OPERATING ENVIRONMENT"
                        }
                    }
                }

                nav aria-label="Primary navigation" class="site-nav" style="display: flex; align-items: center; gap: 24px; font-size: 14px; font-weight: 500;" {
                    a class="nav-section-link" href="#terminal" { "Terminal" }
                    a class="nav-section-link" href="#benchmarks" { "Benchmarks" }
                    a class="nav-section-link" href="#architecture" { "Architecture" }
                    a class="nav-section-link" href="#ecosystem" { "Ecosystem" }
                    a class="nav-section-link" href="#install" { "Install" }
                    a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="display: flex; align-items: center; gap: 6px; color: var(--text-primary); padding: 6px 12px; border-radius: 4px; border: 1px solid var(--border-subtle); background: var(--bg-surface); font-family: var(--font-mono); font-size: 12px;" {
                        (PreEscaped(r#"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"></path><path d="M9 18c-4.51 2-5-2-7-2"></path></svg>"#))
                        span { "GitHub" }
                    }
                }
            }
        }
    }
}
