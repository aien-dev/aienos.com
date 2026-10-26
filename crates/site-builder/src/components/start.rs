//! /start/ : the shortest route through the project for a new reader.
//!
//! The page does not restate status figures. It links to the canonical
//! implementation list, the evidence record and the release record, and it
//! names the boundary between AienOS, the author's personal record and
//! Interplane.

use maud::{html, Markup};

use super::narrative::SLOGAN;

pub const TITLE: &str = "Start here | AienOS";
pub const DESCRIPTION: &str = "The shortest route through AienOS: what it is, what exists today, how to check the evidence, how to reproduce or contribute, and where the author and Interplane fit.";
pub const URL: &str = "https://www.aienos.com/start/";

pub fn render_start_page() -> Markup {
    html! {
        article class="narr-page" {
            div class="narr-column" {
                header class="narr-header" {
                    p class="narr-kicker" { "New to AienOS" }
                    h1 class="narr-title" { "Start here" }
                    p class="narr-standfirst" {
                        "AienOS is an experimental machine in alpha testing for turning search into verified understanding. This page is the shortest route from what it is to what exists today, how to check the evidence, and how to take part."
                    }
                    p class="narr-standfirst" {
                        "Here to learn to build with AI? "
                        a href="/learn/" { "Start with a practical working method" }
                        ", then visit "
                        a href="/hive/" { "Hive" }
                        " to watch Drake and the coding agents use it, including the corrections and failed attempts."
                    }
                }

                section class="narr-section" id="what" {
                    h2 { "1. What it is" }
                    p {
                        "AIEN proposes structure. Omega makes that structure explicit and checkable. The machine tests it against reality under authority. The Turing measures what was learned. Cortex keeps the evidence, and the next search begins from what survived."
                    }
                    p { (SLOGAN) }
                    p {
                        "The full argument is on "
                        a href="/machine/" { "The Machine" }
                        "."
                    }
                }

                section class="narr-section" id="today" {
                    h2 { "2. What exists today" }
                    p {
                        "The canonical list is the \"What exists today\" section of The Machine. It is rendered from the research status record and it separates built work from open questions. The same list sits beside every experiment on the research status page."
                    }
                    p {
                        a href="/machine/#today" { "What exists today" }
                        " and "
                        a href="/research/status/#implementation" { "the implementation list beside every experiment" }
                        "."
                    }
                    p {
                        "Release state: alpha testing for Linux arm64 and NVIDIA GB10. The installer and its evidence boundaries are at "
                        a href="/alpha/" { "/alpha/" }
                        ". The canonical layer record remains "
                        a href="/status.json" { "status.json" }
                        "."
                    }
                }

                section class="narr-section" id="evidence" {
                    h2 { "3. Check the evidence" }
                    p {
                        "Every claim is meant to end at a public receipt. Start with the evidence hub, then the experiments, then the interactive Turing guide if you want to feel the measurement rather than read it."
                    }
                    ul {
                        li {
                            a href="/evidence/" { "Evidence" }
                            ": what counts, which source wins when two disagree, and where the receipts live."
                        }
                        li {
                            a href="/experiments/" { "Experiments" }
                            ": the research ladder, failures included."
                        }
                        li {
                            a href="/research/status/" { "Research status" }
                            ": every experiment with its verdict, receipt and limits."
                        }
                        li {
                            a href="/turing/" { "The Turing" }
                            ": an interactive field guide with games and live calculations."
                        }
                    }
                }

                section class="narr-section" id="reproduce" {
                    h2 { "4. Reproduce or contribute" }
                    p {
                        "The site is built by the Rust builder in this repository. "
                        a href="/AGENTS.md" { "AGENTS.md" }
                        " carries the installation, configuration and usage commands. The source, issues and public record are at "
                        a href="https://github.com/aien-dev/aienos.com" { "aien-dev/aienos.com" }
                        "."
                    }
                    p {
                        "To put your own coding agent to work on any AIEN repository, start at "
                        a href="/agents/" { "Agents" }
                        ": the rules my agents follow and each repository's own instructions."
                    }
                    p {
                        "The experimental installer targets Linux arm64 and NVIDIA GB10 only; read its limits on the alpha page. To reproduce a result, follow the receipt on its evidence card. To reproduce the site, build it from the repository."
                    }
                }

                section class="narr-section" id="boundaries" {
                    h2 { "5. Who and what is around it" }
                    p {
                        "AienOS is built by M. Drake Stapleton. His "
                        a href="https://www.drakestapleton.com" { "personal site" }
                        " is the record of the person, not the project."
                    }
                    p {
                        a href="https://interplane.io/" { "Interplane" }
                        " is an open interoperability protocol between a model and the program that runs it. A model expresses intent; the runtime keeps authority. It is not AienOS, not the operating system, and not the model."
                    }
                }

                p class="narr-more" {
                    a href="/machine/" { "Next: The Machine" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> String {
        render_start_page().into_string()
    }

    #[test]
    fn links_the_canonical_surfaces() {
        let s = page();
        for needle in [
            "href=\"/machine/#today\"",
            "href=\"/research/status/#implementation\"",
            "href=\"/evidence/\"",
            "href=\"/experiments/\"",
            "href=\"/turing/\"",
            "href=\"/AGENTS.md\"",
            "href=\"/status.json\"",
            "href=\"https://github.com/aien-dev/aienos.com\"",
        ] {
            assert!(s.contains(needle), "start page is missing {needle}");
        }
    }

    #[test]
    fn names_the_cross_project_boundaries() {
        let s = page();
        assert!(s.contains("https://www.drakestapleton.com"), "author link missing");
        assert!(s.contains("not the project"), "author boundary missing");
        assert!(s.contains("https://interplane.io/"), "Interplane link missing");
        assert!(s.contains("not AienOS"), "Interplane boundary missing");
    }

    #[test]
    fn has_no_em_or_en_dash() {
        let s = page();
        assert!(!s.contains('\u{2014}') && !s.contains('\u{2013}'), "start page must not use em or en dashes");
    }
}
