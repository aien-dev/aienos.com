use maud::{html, Markup, PreEscaped};

/// The mark: six blocks of one tree, one of them lit. Same motif as the Aienix wordmark.
pub const MARK_SVG: &str = r##"<svg class="brand-mark" viewBox="0 0 76 76" aria-hidden="true" focusable="false"><rect x="0" y="0" width="22" height="22" rx="4" fill="currentColor"/><rect x="27" y="0" width="22" height="22" rx="4" fill="currentColor"/><rect x="54" y="0" width="22" height="22" rx="4" fill="currentColor"/><rect x="0" y="27" width="22" height="22" rx="4" fill="currentColor"/><rect x="27" y="27" width="22" height="22" rx="4" fill="#e0b35a"/><rect x="54" y="27" width="22" height="22" rx="4" fill="currentColor"/><rect x="0" y="54" width="76" height="22" rx="4" fill="currentColor"/></svg>"##;

pub fn render_navbar() -> Markup {
    html! {
        header class="site-header" {
            div class="container" {
                a class="brand" href="/" aria-label="AIENOS home" {
                    (PreEscaped(MARK_SVG))
                    span { "AIENOS" }
                }
                nav class="site-nav" aria-label="Sections" {
                    a href="#components" { "Components" }
                    a href="#verification" { "Verification" }
                    a href="#principles" { "Principles" }
                    a href="#run" { "Run it" }
                    a href="https://github.com/aien-dev" rel="noopener" { "GitHub" }
                }
            }
        }
    }
}
