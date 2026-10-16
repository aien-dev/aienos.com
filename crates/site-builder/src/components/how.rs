use maud::{html, Markup};

use crate::facts::Facts;

pub fn render_how(facts: &Facts) -> Markup {
    html! {
        section class="section" id="verification" {
            div class="container" {
                div class="section-head" {
                    span class="section-num" { "03" }
                    div {
                        h2 { "How a change gets in" }
                        p class="lede" {
                            "Every pull request in every AIEN repository is judged on the Spark itself, by our own pipeline, not by a hosted runner. The receipt at the top of this page is one of its outputs."
                        }
                    }
                }
                div class="two-col" {
                    ol class="steps" {
                        li {
                            strong { "A gate manifest" }
                            " in the repository declares each test, the files it depends on, and what counts as a pass."
                        }
                        li {
                            strong { "The judge" }
                            " checks out the exact commit, reruns only the gates whose inputs changed, and writes an evidence bundle: per-gate verdicts, exit codes, and content-addressed logs."
                        }
                        li {
                            strong { "The verdict is signed and published" }
                            " to the pull request. The hosted GitHub check does one thing only: confirm that the published bundle matches the commit."
                        }
                        li {
                            strong { "A maintainer labels the change" }
                            " ready to merge. The pipeline merges it when the Spark bundle passes and the hosted check agrees. Nothing merges on a green badge alone."
                        }
                    }
                    div {
                        h3 { "Recent decision records" }
                        p class="small" style="margin: 8px 0 16px;" {
                            "Architecture Decision Records are how the project changes its mind in public. The newest five:"
                        }
                        ul class="list-plain" style="gap: 10px;" {
                            @for d in &facts.decisions {
                                li {
                                    a href=(d.url) rel="noopener" {
                                        span class="mono" style="color: var(--muted); margin-right: 10px;" { "ADR " (d.id) }
                                        (d.title)
                                    }
                                }
                            }
                        }
                        p class="small" style="margin-top: 18px;" {
                            a href="https://github.com/aien-dev/aien-architecture/tree/main/docs/adr" rel="noopener" { "All decision records" }
                        }
                    }
                }
            }
        }
    }
}
