//! /owned/ : "Intelligence should be owned", Drake Stapleton's manifesto.
//!
//! A position page, listed under Philosophy. It argues a stance, so it
//! carries no status words. Every figure in it resolves to a receipt on
//! GitHub, and the receipts section says which of them are not on main yet.

use maud::{html, Markup};

use super::boxlink::{foot, source_href, Target};

pub const TITLE: &str = "Intelligence should be owned | AienOS";
pub const DESCRIPTION: &str = "Drake Stapleton on why language models belong to everyone who wrote the words they learned from, why running one yourself is a right and a responsibility, and what Aien does about it, with receipts.";
pub const URL: &str = "https://www.aienos.com/owned/";

/// This file, for a box whose text has no other home on GitHub.
const THIS_FILE: &str = "crates/site-builder/src/components/owned.rs";

/// The one sentence the page is built around.
pub const THESIS: &str = "Intelligence should be owned by the people who made it.";

/// Phrase the build checks for, so the thesis cannot be edited away.
pub const SIGNATURE_LINE: &str = "Drake Stapleton";

struct Receipt {
    title: &'static str,
    says: &'static str,
    href: &'static str,
    /// True when the evidence is on a branch that is not merged to main.
    on_branch: bool,
}

const RECEIPTS: &[Receipt] = &[
    Receipt {
        title: "The licence",
        says: "Omega, CUDApp, Omega Bridge, aien-crypto and physics-mv use Apache-2.0 WITH LLVM-exception. The exception lets anyone ship compiled kernels and binaries under terms they choose; the compiler source itself keeps Apache-2.0 attribution and NOTICE terms. interplane-mv and benchmarks-mv stay plain Apache-2.0.",
        href: "https://github.com/aien-dev/aien-sovereign-core/blob/main/LICENSE",
        on_branch: false,
    },
    Receipt {
        title: "A 27 billion parameter model on my own engine",
        says: "Qwen3.8-27B in 8-bit form, served by my own server on my own GPU kernels, on one DGX Spark. Measured on the chip alone, not end to end: 7.63 tokens a second of GPU time per token against an arithmetic memory ceiling of 10.15 for this machine and this model, which is 75.17 percent of the ceiling (calc:ce5d974e). Slower than I want.",
        href: "https://github.com/aien-dev/aien-sovereign-core-mv/tree/qwen38/serve-fast/evidence/qwen38-serve-fast-20261019",
        on_branch: true,
    },
    Receipt {
        title: "The ceiling that number is judged against",
        says: "How the 10.15 tokens a second limit was worked out from the machine's memory speed and the size of the model. Arithmetic, not a measurement.",
        href: "https://github.com/aien-dev/aien-sovereign-core-mv/blob/alpha/ceiling-and-model/docs/qwen38/CEILING.md",
        on_branch: true,
    },
    Receipt {
        title: "An operating system on a 2017 iMac",
        says: "The AienOS kernel, aienux, boots on an ordinary desktop computer, bare metal, with no Linux underneath.",
        href: "https://github.com/aien-dev/aienux/blob/main/evidence/imac_kernel1_2026-10-17",
        on_branch: false,
    },
    Receipt {
        title: "Hardware runs on a Raspberry Pi 5",
        says: "The AienOS kernel, aienux, running on a small, inexpensive board, with a receipt for each run.",
        href: "https://github.com/aien-dev/aienux/blob/main/evidence/pi5_hardware_runs_2026-10-13",
        on_branch: false,
    },
    Receipt {
        title: "The first outside-style install test failed",
        says: "A fresh user account followed the written guide for running the 27B model. The guide did not pass as written. The fixes are queued and the failure stays on the record.",
        href: "https://github.com/aien-dev/aien-sovereign-core-mv/tree/alpha/fresh-account-evidence/evidence/alpha-fresh-account-20261019",
        on_branch: true,
    },
];

pub fn render_owned() -> Markup {
    html! {
        article class="narr-page" {
            div class="narr-column" {
                header class="narr-header" {
                    p class="narr-kicker" { "Manifesto" }
                    h1 class="narr-title" { "Intelligence should be owned" }
                    p class="narr-standfirst" {
                        "I build Aien because I believe the intelligence in a language model belongs to everyone who helped make it. That is a very large group, and I am one of them. So are you."
                    }
                    p class="narr-byline" { (SIGNATURE_LINE) }
                }

                section id="inheritance" class="narr-section" aria-labelledby="inheritance-heading" {
                    h2 id="inheritance-heading" { "Where it came from" }
                    p {
                        "A language model learns from the written record of people. Books, papers, code, forum answers, recipes, letters, arguments, jokes. Billions of people wrote that record over thousands of years, mostly without knowing a machine would one day read it."
                    }
                    p {
                        "A company can pay for the computers, hire the engineers and spend the months of training. I respect that work. The knowledge it turned into a model was never theirs to start with. It was everyone's."
                    }
                    p {
                        "That makes a model a common inheritance. Something built from all human knowledge cannot honestly belong to one account holder."
                    }
                }

                section id="right" class="narr-section" aria-labelledby="right-heading" {
                    h2 id="right-heading" { "It is everyone's right" }
                    blockquote class="narr-rule" { p { (THESIS) } }
                    p {
                        "It is everyone's right to use the intelligence people created. Using it means more than asking it a question through someone else's window. It means holding a copy, running it on a machine you own, looking at how it works, and changing it."
                    }
                    p {
                        "A person who can run it themselves has the right to. Nobody should need permission, a subscription or an invitation to do that."
                    }
                }

                section id="responsibility" class="narr-section" aria-labelledby="responsibility-heading" {
                    h2 id="responsibility-heading" { "It is also everyone's responsibility" }
                    p {
                        "A right that nobody exercises does not last. If people only ever rent intelligence, the people who own the machines decide what they may ask, what it may answer, and what it costs. They would have given the inheritance back for a monthly fee."
                    }
                    p {
                        "So running it yourself comes with work. Keep your copy. Learn how it behaves. Write down what you measured, including the bad numbers. Share what you learn so the next person starts further along. That is how a common inheritance stays common."
                    }
                }

                section id="owned-not-rented" class="narr-section" aria-labelledby="owned-heading" {
                    h2 id="owned-heading" { "Owned, not rented" }
                    p {
                        "When you rent, the price can change, the rules can change and the service can end. Your work sits on someone else's floor."
                    }
                    p {
                        "When you own, it runs with the network cable pulled. It runs after the company that made it has moved on. The test I set for the whole project is simple: if a company disappears tomorrow, AienOS still boots and remains mine."
                    }
                    p {
                        "Owning does not mean building every part yourself. It means no outside organization is required for the machine to start, to keep its data, to change its model, or to keep operating."
                    }
                }

                section id="paywalls" class="narr-section" aria-labelledby="paywalls-heading" {
                    h2 id="paywalls-heading" { "Compute as a service is fine" }
                    p {
                        "Renting a powerful machine for an afternoon is a good deal, and selling that is an honest business. If your own computer is too small for a job, rent a bigger one."
                    }
                    p {
                        "What I object to is a paywall that is the only door. When the single way to touch a model is through a metered account, and the weights and the means to run them are kept out of reach, a service has turned into a gate on something that belongs to everybody."
                    }
                    p {
                        "A service should be a choice. The other door, running it yourself, has to stay open."
                    }
                }

                section id="aien" class="narr-section" aria-labelledby="aien-heading" {
                    h2 id="aien-heading" { "What Aien does about it" }
                    p {
                        "Aien is my attempt to keep that other door open. Everything I and my agents write is public under a licence that keeps it public. The stack is built to run on machines you own, with no outside service in the trusted core. Speed is reported against what the hardware could do at its limit."
                    }
                    p {
                        "Here is what exists today, with the evidence beside it. Where the evidence is not on the main branch yet, the box says so."
                    }
                    ul class="hub-cards" {
                        @for r in RECEIPTS {
                            li class="hub-card" {
                                a href=(r.href) target="_blank" rel="noopener noreferrer" { (r.title) }
                                p { (r.says) }
                                @if r.on_branch {
                                    p class="hub-note" { "This receipt is on a branch that is not merged to main yet." }
                                }
                                (foot(r.href, Target::Evidence))
                            }
                        }
                    }
                    h3 { "What is not true yet" }
                    p {
                        "Sovereign Intelligence, built on AienOS, is in alpha testing. The model I serve was trained by others and keeps its maker's licence. I have not trained a model of my own. The install guide failed its first test. Speed is below the ceiling. I would rather you read that here than discover it later."
                    }
                }

                section id="ask" class="narr-section" aria-labelledby="ask-heading" {
                    h2 id="ask-heading" { "The ask" }
                    p {
                        "Run it yourself. Whatever you run, on whatever machine you own, keep a copy and learn how it works."
                    }
                    p {
                        "If you want to help here, read the code at "
                        a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" { "github.com/aien-dev" }
                        ". Break the install guide and tell me where. Send a change. Publish what you measure, good or bad, with the receipt beside it."
                    }
                    p {
                        "Then tell someone else that they are allowed to do the same."
                    }
                    p class="narr-byline" { (SIGNATURE_LINE) }
                    ul class="hub-cards" {
                        li class="hub-card" {
                            a href="/philosophy/" { "More on the philosophy behind Aien" }
                            p { "The Stapleton Doctrine and the Post-LLM Case, argued as positions and not as experimental results." }
                            (foot(&source_href(THIS_FILE), Target::Source))
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::base::render_page_layout;

    fn page() -> String {
        render_page_layout(TITLE, DESCRIPTION, URL, "article", html! {}, render_owned()).into_string()
    }

    #[test]
    fn thesis_and_signature_are_present() {
        let p = page();
        assert!(p.contains(THESIS));
        assert!(p.contains(SIGNATURE_LINE));
    }

    #[test]
    fn no_em_or_en_dashes() {
        let p = page();
        assert!(!p.contains('\u{2014}'), "em dash in /owned/");
        assert!(!p.contains('\u{2013}'), "en dash in /owned/");
    }

    #[test]
    fn every_box_links_to_github() {
        let p = page();
        let n = crate::components::boxlink::check_boxes(&p).expect("a box lacks its GitHub link");
        assert_eq!(n, RECEIPTS.len() + 1);
    }

    #[test]
    fn receipts_all_point_at_github() {
        for r in RECEIPTS {
            assert!(r.href.starts_with("https://github.com/aien-dev/"), "{} -> {}", r.title, r.href);
        }
    }

    #[test]
    fn no_status_words_on_a_position_page() {
        let body = render_owned().into_string();
        for s in crate::components::research_status::ALLOWED_STATUSES {
            assert!(!body.contains(s), "status word {s} on /owned/");
        }
    }

    #[test]
    fn comparisons_are_dated_and_sourced_or_absent() {
        let body = render_owned().into_string();
        let problems = crate::components::machines::comparison_errors(&body);
        assert!(problems.is_empty(), "comparison problem on /owned/: {problems:?}");
        assert!(crate::components::machines::comparison_problem("faster than ChatGPT").is_some());
        assert!(crate::components::machines::comparison_problem(
            "On 2026-10-21 this was faster than the published ChatGPT figure (https://example.com/n)."
        ).is_some());
        assert!(crate::components::machines::comparison_problem(
            "On 2026-10-21 this was faster than the published ChatGPT figure (https://github.com/aien-dev/sovereign-intelligence)."
        ).is_none());
    }
}
