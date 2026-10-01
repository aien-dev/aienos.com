//! /machine/ : "The Machine", the main research narrative.
//!
//! Wording comes from the PRE-APPROVED synthesis handoff
//! (~/handoffs/2026-10-01-research-synthesis-handoff.md). Status labels in
//! "What exists today" come only from ~/handoffs/2026-10-01-NARR-IMPL-scout.md.

use maud::{html, Markup};

use super::narrative::{badge, status_legend, Status, SLOGAN, TURING_FORMULA, TY2_GAIN_BITS};

pub const TITLE: &str = "The Machine | AIENOS";
pub const DESCRIPTION: &str = "AIENOS is an owned experimental machine for turning search into verified understanding. The closed epistemic loop, the Turing as its accounting layer, and an honest list of what exists today.";
pub const URL: &str = "https://aienos.com/machine/";

/// Anchor of the homepage loop diagram (NARR-HOME, components/home_loop.rs).
pub const HOME_LOOP_HREF: &str = "/#loop-title";

const MISSION: &str = "AIENOS is an owned experimental machine for turning search into verified understanding. AIEN proposes structure. Omega makes that structure explicit and checkable. The machine tests it against reality under authority. The Turing measures what was actually learned. Cortex keeps the evidence. Then the next search begins from what survived.";

const LOOP_STAGES: [&str; 11] = [
    "Observe",
    "Propose",
    "Formalize",
    "Verify",
    "Authorize",
    "Execute",
    "Measure",
    "Score",
    "Preserve evidence",
    "Learn",
    "Search again",
];

/// One row of the "What exists today" list. `sentence` is the safe public
/// sentence from the NARR-IMPL scout table, kept with its stated limits.
pub struct StatusItem {
    pub name: &'static str,
    pub status: Status,
    pub sentence: &'static str,
}

pub const STATUS_ITEMS: &[StatusItem] = &[
    StatusItem {
        name: "Resident World and reactions",
        status: Status::Implemented,
        sentence: "A resident runtime of reactions over a shared World runs on the machine; its current living build has not yet been re-qualified.",
    },
    StatusItem {
        name: "World lifecycle model",
        status: Status::Implemented,
        sentence: "A small formal model of the World's lifecycle exists as a test, alongside the real code.",
    },
    StatusItem {
        name: "State-space exploration",
        status: Status::Implemented,
        sentence: "Exhaustive exploration of a small World model checks four safety rules and prints the shortest failing trace; it currently shows two known gaps in the real World (no cancel, deadlines only checked at admission).",
    },
    StatusItem {
        name: "Differential checking",
        status: Status::Implemented,
        sentence: "The model is replayed against the real World code for every short operation sequence and must match it. This runs on a host computer only.",
    },
    StatusItem {
        name: "Omega program identities",
        status: Status::Implemented,
        sentence: "Omega programs get identities bound to their canonical body and contract.",
    },
    StatusItem {
        name: "Action graph",
        status: Status::Implemented,
        sentence: "Goals can be compiled into a typed action graph, demonstrated in a qualification test. The live loop does not call it yet.",
    },
    StatusItem {
        name: "Typed result contracts",
        status: Status::Implemented,
        sentence: "Results can be checked against typed contracts before they are published, demonstrated in a qualification test. The live loop does not call it yet.",
    },
    StatusItem {
        name: "Verification infrastructure",
        status: Status::Implemented,
        sentence: "Changes are checked by verifiers and recorded with receipts tied to the exact code version.",
    },
    StatusItem {
        name: "AEGIS",
        status: Status::Implemented,
        sentence: "AEGIS, the part that decides what is allowed, is in the runtime and passed its qualification gate.",
    },
    StatusItem {
        name: "Evidence receipts",
        status: Status::Implemented,
        sentence: "Results are kept as receipts bound to the exact code version; they are not yet cryptographically signed.",
    },
    StatusItem {
        name: "Turing scorer",
        status: Status::Implemented,
        sentence: "The Turing measurement has a frozen scoring profile and a second, independently written scorer that agreed on all 312 checked values.",
    },
    StatusItem {
        name: "SearchTrace",
        status: Status::Partial,
        sentence: "Omega records every step of its program search; learning from those records is not built yet.",
    },
    StatusItem {
        name: "Bayesian cost prediction",
        status: Status::Partial,
        sentence: "A Bayesian cost model that picks among verified realizations from measured runs passed its qualification test; it is not yet used by the live runtime.",
    },
    StatusItem {
        name: "Capabilities and authority",
        status: Status::Partial,
        sentence: "Authority is held as unforgeable capabilities, tested on the host and in an emulated machine; not yet on real hardware boots.",
    },
    StatusItem {
        name: "ARGUS",
        status: Status::Partial,
        sentence: "ARGUS, the defensive observer, exists and can revoke one narrow permission in an emulated machine; wider defence is still ahead.",
    },
    StatusItem {
        name: "FORGE",
        status: Status::Partial,
        sentence: "FORGE, the layer that turns verified programs into work on the chip, passed its gates as seen from Omega; its own repository was not audited here.",
    },
    StatusItem {
        name: "Cortex",
        status: Status::Partial,
        sentence: "Cortex today is an append-only typed memory that can record what the World did and what failed; storing search history, counterexamples and costs for future search is planned.",
    },
    StatusItem {
        name: "Energy attribution",
        status: Status::Partial,
        sentence: "Energy attribution is designed and unit-tested; no measured bits-per-joule figure exists yet.",
    },
    StatusItem {
        name: "ATLAS",
        status: Status::Planned,
        sentence: "ATLAS is the designed boot seed of the stack; this check did not verify its code, so it is listed as planned.",
    },
    StatusItem {
        name: "Turing yield (T/J)",
        status: Status::Planned,
        sentence: "A measured Turings-per-joule figure has not been started; it will need the energy attribution above joined to a Turing receipt.",
    },
    StatusItem {
        name: "Active experiment choice (EXP-003)",
        status: Status::Planned,
        sentence: "Choosing experiments that best separate competing explanations is the next planned test; it has not run.",
    },
    StatusItem {
        name: "Physics Zero and the Dirac test",
        status: Status::Planned,
        sentence: "Physics Zero and the Dirac test are planned examinations; a reference oracle has been written but never built or run.",
    },
    StatusItem {
        name: "Counterexample generalization (CEGIS style)",
        status: Status::Hypothesis,
        sentence: "Turning failed candidates into general rules that prune future search is a research direction, not a feature.",
    },
    StatusItem {
        name: "E-graphs and equality saturation",
        status: Status::Hypothesis,
        sentence: "E-graphs are a research reference for treating equivalent forms as one discovery; Omega does not use them.",
    },
    StatusItem {
        name: "Self-improvement through measured abstraction",
        status: Status::Hypothesis,
        sentence: "Whether verified abstractions make later discovery cheaper is an open research question; only a small abstraction-finding component exists today.",
    },
];

fn section_lead() -> Markup {
    html! {
        header class="narr-header" {
            p class="narr-kicker" { "The Machine" }
            h1 class="narr-title" { "Turning search into verified understanding" }
            blockquote class="narr-mission" {
                p { (MISSION) }
            }
            p class="narr-slogan" { (SLOGAN) }
            p {
                "Suggestions and explanations have different epistemic roles. Neural systems generate hypotheses. Explicit artifacts suit inspection and verification. Physical execution provides contact with reality, measurement determines gain, and evidence provides continuity."
            }
        }
    }
}

fn section_loop() -> Markup {
    html! {
        section id="loop" class="narr-section" aria-labelledby="loop-heading" {
            h2 id="loop-heading" { "The closed loop" }
            p {
                "The meaningful artifact of this project is a closed epistemic loop. Each pass runs through eleven steps, and the last step feeds the first."
            }
            ol class="narr-loop" aria-label="The closed epistemic loop, in order" {
                @for stage in LOOP_STAGES.iter() {
                    li { (stage) }
                }
            }
            p {
                "In the design, each step has an owner. AIEN suggests. Omega formalizes, synthesizes and verifies. FORGE realizes the verified program, and the physical machine executes it."
            }
            p {
                "The Turing then measures explanatory gain, Cortex preserves the evidence, and AIEN and Omega use that evidence to improve the next search. ATLAS carries the lineage across hardware, and AIENOS provides the owned, persistent substrate underneath."
            }
            p {
                "AEGIS and ARGUS sit around the loop. AEGIS guards the boundary of proof and authority, so nothing executes without permission. ARGUS observes what actually happens. Neither one is a step that work passes through in sequence."
            }
            p {
                "This describes the design. The list under What exists today says which parts run now. "
                a href=(HOME_LOOP_HREF) { "See the loop diagram on the homepage." }
            }
        }
    }
}

fn section_turing() -> Markup {
    html! {
        section id="turing" class="narr-section" aria-labelledby="turing-heading" {
            h2 id="turing-heading" { "The Turing keeps the accounts" }
            p {
                "The Turing is the epistemic accounting layer of AIEN. It measures net held-out explanatory compression: one operational component of machine understanding. It is not a general intelligence score."
            }
            p {
                "For a baseline B, a candidate explanation M, sealed observations D and a scoring profile P, with L meaning description length:"
            }
            figure class="narr-formula" aria-label="The Turing formula" {
                pre { code { (TURING_FORMULA) } }
            }
            p {
                "A candidate earns positive Turings only if it shrinks the description of unseen data by more than its own complexity costs."
            }
            ul class="narr-ledger" {
                li { strong { "Charged: " } "memorization, unnecessary complexity and bad predictions." }
                li { strong { "Prohibited: " } "post-hoc selection. Test contamination invalidates the measurement." }
                li { strong { "Rewarded: " } "compact structure that generalizes." }
            }
            p {
                "The question it answers: what did the machine discover that continued to work on reality it had not seen? "
                a href="/turing/" { "Explore the Turing." }
            }
        }
    }
}

fn section_quantities() -> Markup {
    html! {
        section id="quantities" class="narr-section" aria-labelledby="quantities-heading" {
            h2 id="quantities-heading" { "Three quantities" }
            ul class="narr-cards" {
                li class="narr-card" {
                    h3 { "Turing gain (T)" }
                    p {
                        "Did the machine learn explanatory structure? Gain is net held-out explanatory compression after paying for the explanation."
                    }
                    p {
                        "Run TY-2 recorded a gain of +" (TY2_GAIN_BITS) " bits against an order-1 baseline on held-out seeds. The re-check used the same tool, so it is a self re-derivation and still awaits an independent one. This result certifies the instrument; it does not show that AIEN discovered anything."
                    }
                }
                li class="narr-card" {
                    h3 { "Turing yield (T/J) " (badge(Status::Planned)) }
                    p {
                        "What physical resources did that gain cost? Yield will divide Turings by joules, and may separate evaluation yield from discovery yield, the more important of the two."
                    }
                    p {
                        "Not started. Energy attribution is partial: designed and unit-tested, with no timed run yet. No yield figure exists, and none will be claimed until energy attribution and the Turing receipt are joined under the qualifying protocol."
                    }
                }
                li class="narr-card" {
                    h3 { "Search and verification gap " (badge(Status::Hypothesis)) }
                    p {
                        "How hard was the explanation to discover compared with verifying it once found? The hypothesis: discovered explanatory structure reduces the cost of future search, through prediction, compression, abstraction, reusable procedure, reduced search and faster discovery."
                    }
                    p { "This is a research direction. No such result exists yet." }
                }
            }
        }
    }
}

fn section_cortex() -> Markup {
    html! {
        section id="cortex" class="narr-section" aria-labelledby="cortex-heading" {
            h2 id="cortex-heading" { "Evidence that teaches: Cortex" }
            p {
                "Cortex exists to preserve the evidence needed to make future search better. That means evidence continuity: verified explanations, failed hypotheses, counterexamples, validity regions, measurement receipts, experimental conditions, search traces, physical costs, uncertainty and provenance."
            }
            p {
                "Today Cortex is an append-only typed memory that records what the World did and what failed. Storing search history, counterexamples and costs is planned."
            }
            p { "Once that is built, AIEN will be able to ask:" }
            ul class="narr-questions" {
                li { "What did we already try?" }
                li { "Why did it fail?" }
                li { "Under what regime did it work?" }
                li { "What evidence supports this abstraction?" }
                li { "What representation changes reduced search before?" }
            }
        }
    }
}

fn section_ladder() -> Markup {
    html! {
        section id="research-ladder" class="narr-section" data-source="LT-TRUTH status source (pending)" aria-labelledby="ladder-heading" {
            h2 id="ladder-heading" { "The discovery ladder" }
            p {
                "The research ladder will render here from the canonical status source, so this page never states a level's result by hand. Until then, see "
                a href="/experiments/" { "the experiments" }
                "."
            }
        }
    }
}

fn section_rsi() -> Markup {
    html! {
        section id="self-improvement" class="narr-section" aria-labelledby="rsi-heading" {
            h2 id="rsi-heading" { "Self-improvement, measured" }
            p {
                "The interesting version of recursive self-improvement is narrower than a machine that modifies itself. Experience leads to a measured explanation, then a reusable abstraction, reduced search, better candidate selection, a verified improvement and the next generation."
            }
            p { "A self-modification will count only if it survives verification and measurement." }
            p class="narr-keystone" {
                "The keystone question: can a persistent machine accumulate verified explanatory structure in a way that measurably improves its ability to discover further structure?"
            }
            ul class="narr-questions" {
                li { "Does T increase?" }
                li { "Does T/J improve?" }
                li { "Does search cost fall?" }
                li { "Does verification remain trustworthy?" }
                li { "Do representation changes transfer?" }
                li { "Do interventions become more informative?" }
                li { "Does learned structure survive new domains?" }
                li { "Does the system preserve its failures?" }
                li { "Can independent scorers reproduce the evidence?" }
            }
        }
    }
}

fn section_connections() -> Markup {
    html! {
        section id="connections" class="narr-section" aria-labelledby="connections-heading" {
            h2 id="connections-heading" { "Connections to other research" }
            p { "Each of these is a connection or an opportunity. None of them describes a present capability of AIEN or Omega." }
            dl class="narr-connections" {
                dt { "Petri nets and reaction systems" }
                dd {
                    "The World lifecycle work already has a state model, reachable-state exploration, invariants, shortest counterexample traces, mutants and differential checking against the real World code. Formal concurrency theory may offer reachability, liveness and deadlock analysis, partial-order reduction and invariant proofs. The World keeps its own name; it is not being relabelled a Petri net."
                }
                dt { "Counterexample-guided synthesis (CEGIS)" }
                dd {
                    "Omega's search already runs candidate, transform, verify, accept or reject, and records the trace and cost. The opportunity: turn a failed candidate into a semantic counterexample, generalize it into a constraint, eliminate a whole family of candidates, and measure how much search that saves. Omega does not implement CEGIS today."
                }
                dt { "Bayesian adaptive search" }
                dd {
                    "A Bayesian cost model for choosing between verified realizations passed its qualification test. A future direction is safe contextual exploration. Exploration must stay explicitly authorized, and production authority is never inferred from prediction confidence."
                }
                dt { "Active causal discovery" }
                dd {
                    "EXP-003 is planned as the move from passive prediction to active inquiry: hold competing explanations, predict their consequences, choose the intervention that best separates them, run one authorized experiment and update belief. It has not run."
                }
                dt { "E-graphs and equality saturation" }
                dd {
                    "A research reference for treating rewritten expressions, basis changes, coordinate systems and equivalent implementations as one discovery. Omega does not use them."
                }
            }
        }
    }
}

fn section_today() -> Markup {
    html! {
        section id="today" class="narr-section" aria-labelledby="today-heading" {
            h2 id="today-heading" { "What exists today" }
            p {
                "Every label below comes from a check of the code on the main branches of Omega and AIENOS on October 1, 2026. Planned and hypothesis items do not exist yet."
            }
            (status_legend())
            ul class="narr-status-list" {
                @for item in STATUS_ITEMS.iter() {
                    li class=(format!("narr-status-item {}", item.status.class())) {
                        div class="narr-status-head" {
                            (badge(item.status))
                            span class="narr-status-name" { (item.name) }
                        }
                        p { (item.sentence) }
                    }
                }
            }
        }
    }
}

fn section_nature() -> Markup {
    html! {
        section id="nature" class="narr-section" aria-labelledby="nature-heading" {
            h2 id="nature-heading" { "Nature invented the loop" }
            p {
                "Evolution runs variation, selection and retention. The immune system runs variation, binding and amplification. Science runs hypothesis, experiment and theory. AIEN runs proposal, verification, evidence and memory."
            }
            p {
                "These are analogies and not equivalences. What they share is a pattern: expensive generation paired with decisive filtering, where retained structure changes what gets generated next."
            }
            p {
                "Nature invented the loop. This work proposes a unit for one measurable thing the loop can produce: explanatory structure that survives unseen reality. The aim is to make the loop explicit, persistent, owned, auditable, metered, reproducible and bounded by authority."
            }
            p { a href="/discovery/" { "Next: Scientific Discovery, the planned examinations." } }
        }
    }
}

pub fn render_machine() -> Markup {
    html! {
        article class="narr-page" {
            div class="narr-column" {
                (section_lead())
                (section_loop())
                (section_turing())
                (section_quantities())
                (section_cortex())
                (section_ladder())
                (section_rsi())
                (section_connections())
                (section_today())
                (section_nature())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::base::render_page_layout;

    /// Items allowed to carry the Implemented badge. Source of truth:
    /// ~/handoffs/2026-10-01-NARR-IMPL-scout.md (rows classed IMPLEMENTED).
    /// Change this list only when that table changes.
    const ALLOWED_IMPLEMENTED: &[&str] = &[
        "Resident World and reactions",
        "World lifecycle model",
        "State-space exploration",
        "Differential checking",
        "Omega program identities",
        "Action graph",
        "Typed result contracts",
        "Verification infrastructure",
        "AEGIS",
        "Evidence receipts",
        "Turing scorer",
    ];

    fn page() -> String {
        render_page_layout(TITLE, DESCRIPTION, URL, "article", html! {}, render_machine()).into_string()
    }

    #[test]
    fn slogan_is_present() {
        assert!(page().contains(SLOGAN), "slogan missing from /machine/");
    }

    #[test]
    fn turing_formula_is_present() {
        let p = page();
        assert!(
            p.contains("T(M; B, D, P) = [L(B) + L(D | B)] - [L(M) + L(D | M)]"),
            "Turing formula line missing from /machine/"
        );
        assert!(p.contains("one operational component of machine understanding"));
        assert!(!p.contains("how much a machine understood"));
    }

    /// The TY-2 figure is a gain. The block (p or li) that holds it must not
    /// mention yield.
    #[test]
    fn gain_figure_is_never_called_yield() {
        let p = page();
        let mut found = false;
        let mut from = 0;
        while let Some(rel) = p[from..].find(TY2_GAIN_BITS) {
            found = true;
            let at = from + rel;
            let start = ["<p", "<li"].iter().filter_map(|t| p[..at].rfind(*t)).max().unwrap_or(0);
            let end = ["</p>", "</li>"]
                .iter()
                .filter_map(|t| p[at..].find(*t).map(|e| at + e))
                .min()
                .unwrap_or(p.len());
            let block = p[start..end].to_lowercase();
            assert!(
                !block.contains("yield"),
                "TY-2 gain figure sits next to the word yield: {block}"
            );
            from = at + TY2_GAIN_BITS.len();
        }
        assert!(found, "TY-2 gain figure missing from /machine/");
    }

    #[test]
    fn implemented_badges_match_scout_table() {
        for item in STATUS_ITEMS.iter().filter(|i| i.status == Status::Implemented) {
            assert!(
                ALLOWED_IMPLEMENTED.contains(&item.name),
                "{} is badged Implemented but is not IMPLEMENTED in the NARR-IMPL scout table",
                item.name
            );
        }
        let rendered = page().matches("narr-status-item status-implemented").count();
        let declared = STATUS_ITEMS.iter().filter(|i| i.status == Status::Implemented).count();
        assert_eq!(rendered, declared, "rendered Implemented rows differ from the item list");
    }

    #[test]
    fn research_ladder_slot_is_present() {
        let p = page();
        assert!(p.contains("id=\"research-ladder\""), "research-ladder slot missing");
        assert!(p.contains("data-source=\"LT-TRUTH status source (pending)\""));
    }

    #[test]
    fn no_em_or_en_dashes() {
        let p = page();
        assert!(!p.contains('\u{2014}'), "em dash in /machine/");
        assert!(!p.contains('\u{2013}'), "en dash in /machine/");
    }
}
