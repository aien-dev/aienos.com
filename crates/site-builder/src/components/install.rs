use maud::{html, Markup};

pub fn render_install() -> Markup {
    html! {
        section class="section" id="run" {
            div class="container" {
                div class="section-head" {
                    span class="section-num" { "05" }
                    div {
                        h2 { "Run it" }
                        p class="lede" {
                            "The sovereign core builds from source with a standard Rust toolchain. The installer below is a short shell script that fetches that source and runs its own build. Read it before you run it; it is forty lines."
                        }
                    }
                }
                div class="install-grid" {
                    div {
                        h3 { "Linux and macOS" }
                        div class="cmd" {
                            span class="prompt" { "$" }
                            code { "curl -fsSL https://www.aienos.com/install.sh | sh" }
                        }
                    }
                    div {
                        h3 { "Or clone it yourself" }
                        div class="cmd" {
                            span class="prompt" { "$" }
                            code { "git clone https://github.com/aien-dev/aien-sovereign-core" }
                        }
                    }
                    div {
                        h3 { "What to expect" }
                        p {
                            "A pre-alpha host runtime. The development machine is a single NVIDIA DGX Spark with the Grace Blackwell GB10; other hardware builds but is not qualified. When a component is qualified, the evidence bundle that proves it is linked from this page."
                        }
                        p class="small" {
                            "The installer source: " a href="/install.sh" { "install.sh" } ". Report problems on the "
                            a href="https://github.com/aien-dev/aien-sovereign-core/issues" rel="noopener" { "sovereign core issue tracker" } "."
                        }
                    }
                }
            }
        }
    }
}
