use maud::{html, Markup};

pub fn render_footer() -> Markup {
    html! {
        footer class="site-footer" {
            div class="container" {
                div {
                    strong style="color: var(--text);" { "AIENOS" }
                    " is part of AIEN, a sovereign computing stack built in the open."
                }
                ul {
                    li { a href="https://github.com/aien-dev" rel="noopener" { "GitHub" } }
                    li { a href="https://github.com/aien-dev/aienix" rel="noopener" { "Aienix, the core tree" } }
                    li { a href="https://github.com/aien-dev/aien-architecture" rel="noopener" { "Architecture and decisions" } }
                    li { a href="https://www.drakestapleton.com" rel="noopener" { "Drake Stapleton" } }
                    li { a href="/facts.json" { "This page's facts" } }
                }
                p class="legal" {
                    "Everything is licensed AGPL-3.0-or-later, including models and weights. This site is one page of static HTML built from Rust source; it loads no fonts, scripts or trackers from anyone. NVIDIA, DGX and Grace Blackwell are trademarks of NVIDIA Corporation."
                }
            }
        }
    }
}
