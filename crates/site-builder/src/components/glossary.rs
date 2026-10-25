use maud::{html, Markup};

pub const TITLE: &str = "Glossary | AienOS";
pub const DESCRIPTION: &str = "The terms AienOS uses in its own sense: the Turing, receipts, crumbs, the loop, the agents that surround it, and the five machines.";
pub const URL: &str = "https://www.aienos.com/glossary/";

fn term(name: &str, definition: Markup) -> Markup {
    html! {
        dt style="font-family: var(--font-mono); font-size: 14px; font-weight: 700; color: var(--accent-green); margin-top: 20px;" { (name) }
        dd style="color: var(--text-secondary); font-size: 16px; line-height: 1.7; margin: 4px 0 0 0;" { (definition) }
    }
}

pub fn render_glossary() -> Markup {
    html! {
        section id="glossary" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" style="max-width: 820px;" {
                div class="badge badge-blue" style="margin-bottom: 12px;" { "TERMINOLOGY" }
                h1 style="font-size: clamp(32px, 5vw, 48px); font-weight: 800; letter-spacing: -0.02em; color: var(--text-primary); margin: 0 0 16px 0;" {
                    "Glossary"
                }
                p style="color: var(--text-secondary); font-size: 17px; line-height: 1.7; margin-bottom: 12px;" {
                    "The terms this site uses in its own sense. Each definition points at the page where the term does real work, so a reader can check the usage rather than trust the summary."
                }
                p style="color: var(--text-muted); font-size: 14px; line-height: 1.6;" {
                    "Status: experimental, pre-alpha. Definitions change when the evidence changes."
                }

                h2 style="font-size: 24px; font-weight: 700; color: var(--text-primary); margin: 44px 0 4px 0;" { "The system" }
                dl style="margin: 0;" {
                    (term("AienOS", html! {
                        "The experimental, open source machine this site documents. A sovereign substrate that owns the hardware and keeps a persistent intelligence alive and continuous. Pre-alpha. See "
                        a href="/machine/" { "The Machine" } "."
                    }))
                    (term("AIEN", html! {
                        "The persistent intelligence that learns. Weights suggest, programs explain, verification decides, evidence teaches. See "
                        a href="/machine/" { "The Machine" } " and " a href="/research/" { "Research" } "."
                    }))
                    (term("Atlas", html! {
                        "The awakening layer, planned. The seed that wakes AienOS on new hardware. Doctrine is published; implementation is ahead."
                    }))
                    (term("Omega", html! {
                        "The language for all computing, GPU to CPU. It turns intent into explicit programs that can be verified before they are trusted. See "
                        a href="/omega/" { "What Omega is" } "."
                    }))
                    (term("CUDA++", html! {
                        "The GPU half of Omega, MIT licensed, for the NVIDIA GPUs people already own. See "
                        a href="/cudaplusplus/" { "CUDA++" } "."
                    }))
                    (term("Turing", html! {
                        "The unit of net held-out explanatory compression. The site uses it to measure what a change actually explained. See "
                        a href="/turing/" { "the Turing field guide" } " and "
                        a href="/research/computing-machinery-and-understanding/" { "the preprint" } "."
                    }))
                    (term("Held-out", html! {
                        "Data not used while fitting, so a result on it measures transfer rather than recall."
                    }))
                    (term("The loop", html! {
                        "Propose, verify, measure, learn. The closed epistemic loop described on the front page."
                    }))
                }

                h2 style="font-size: 24px; font-weight: 700; color: var(--text-primary); margin: 44px 0 4px 0;" { "Around the loop" }
                dl style="margin: 0;" {
                    (term("Cortex", html! { "The canonical memory. It keeps what the machine has established." }))
                    (term("Aegis", html! { "The verifier. It checks generated code and changes nothing." }))
                    (term("Forge", html! { "Realizes what Omega verified." }))
                    (term("Argus", html! { "Watches capability enforcement, so authority stays where it was granted." }))
                }

                h2 style="font-size: 24px; font-weight: 700; color: var(--text-primary); margin: 44px 0 4px 0;" { "Evidence and coordination" }
                dl style="margin: 0;" {
                    (term("Receipt", html! {
                        "A public link to the evidence behind a claim: a pull request, a commit, a measured data file, or a record. See "
                        a href="/evidence/" { "Evidence" } "."
                    }))
                    (term("Crumb", html! {
                        "A durable coordination note under RFC-0001. Sniff before edits, respect unexpired locks, claim multi-step work, close with the result. No receipt, no crumb."
                    }))
                    (term("Research status", html! {
                        "The qualification ladder and every experiment with its verdict, receipt and limits. See "
                        a href="/research/status/" { "Research status" } "."
                    }))
                    (term("Post-LLM", html! {
                        "The position that a learning machine can keep knowledge as inspectable programs, with verification deciding what is trusted. See "
                        a href="/post-llm-case/" { "The Post-LLM Case" } "."
                    }))
                }

                h2 style="font-size: 24px; font-weight: 700; color: var(--text-primary); margin: 44px 0 4px 0;" { "The machines" }
                dl style="margin: 0;" {
                    (term("DGX Spark", html! { "NVIDIA Grace Blackwell GB10 with 128 GB of unified memory. The development machine and the judge." }))
                    (term("iMac 2017", html! { "The x86_64 port of the native C kernel, running on bare metal." }))
                    (term("Raspberry Pi 5", html! { "The small machine." }))
                    (term("QEMU", html! { "The laboratory where every kernel boots first." }))
                    (term("The rig", html! { "The owned inference box." }))
                }

                p style="color: var(--text-secondary); font-size: 16px; line-height: 1.7; margin-top: 40px;" {
                    "The same list is available to machines in "
                    a href="/llms.txt" { "llms.txt" } " and "
                    a href="/sitemap.md" { "sitemap.md" } "."
                }
            }
        }
    }
}
