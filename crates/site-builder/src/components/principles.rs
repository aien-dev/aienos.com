use maud::{html, Markup};

pub fn render_principles() -> Markup {
    html! {
        section class="section" id="principles" {
            div class="container" {
                div class="section-head" {
                    span class="section-num" { "04" }
                    h2 { "Principles" }
                }
                div class="two-col" {
                    ul class="list-plain" {
                        li {
                            strong { "Closest to the metal." }
                            "The fastest correct path wins. Software stays out of the hardware's way, and headline numbers are reported as a fraction of the machine's physical limit."
                        }
                        li {
                            strong { "Sovereign." }
                            "No outside dependencies in the trusted base. Builds are offline and reproducible. No CUDA toolkit, no systemd, no Python anywhere in the project."
                        }
                        li {
                            strong { "Rust is scaffolding, Omega is the destination." }
                            "Rust carries the stack today; the long-term home for every component is Omega. C only where the hardware justifies it, assembly only where measured."
                        }
                    }
                    ul class="list-plain" {
                        li {
                            strong { "Evidence or nothing." }
                            "A result is not demonstrated until a qualification receipt for it exists. A target is never evidence of itself."
                        }
                        li {
                            strong { "Compare only with ourselves." }
                            "Progress is measured against our own previous run, never against other people's systems."
                        }
                        li {
                            strong { "Open." }
                            "Everything is public and licensed AGPL-3.0-or-later, including models and weights."
                        }
                    }
                }
            }
        }
    }
}
