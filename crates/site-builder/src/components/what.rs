use maud::{html, Markup};

pub fn render_what() -> Markup {
    html! {
        section class="section" id="what" {
            div class="container" {
                div class="section-head" {
                    span class="section-num" { "01" }
                    h2 { "What AIEN is" }
                }
                div class="two-col" {
                    div class="prose" {
                        p {
                            "AIENOS is the operating system at the centre of the stack. It is agent-first: the agent is the control plane and the user interface, and the model is never the kernel. Around it sit Omega, the language and GPU engine the stack is being rewritten into; the sovereign core, a Rust host runtime with the inference ABI and the verification pipeline; and the architecture, protocol and physics repositories that say what is being built and prove what has been brought up on the hardware."
                        }
                        p {
                            "Nothing in the trusted base depends on an outside service. Builds are offline and reproducible. There is no CUDA toolkit, no systemd and no Python anywhere in the project."
                        }
                    }
                    div class="prose" {
                        p {
                            "The project is led by one engineer, Drake Stapleton, working with AI agents as day-to-day collaborators. The agents write, test and review; the engineer decides. The architecture, the decision records and the execution plan are public and are the authority for what gets built."
                        }
                        p {
                            "The core components are moving into a single source tree, "
                            a href="https://github.com/aien-dev/aienix" rel="noopener" { "Aienix" }
                            ", so they move in step and are judged as one. That move is planned and recorded in the open, one component at a time."
                        }
                    }
                }
            }
        }
    }
}
