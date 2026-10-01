//! /machine/ : "The Machine", the main research narrative.
//!
//! Wording comes from the PRE-APPROVED synthesis handoff
//! (~/handoffs/2026-10-01-research-synthesis-handoff.md). Every level,
//! verdict and implementation class on this page is rendered from the one
//! canonical record, data/research_status.json, through
//! components::research_status. No status word is written here by hand.

use maud::{html, Markup};

use super::narrative::{class_badge, status_badge, SLOGAN, TURING_FORMULA};
use super::research_status::{
    data, render_implementation, render_research_status_summary, ty2_headline, ImplementationRow,
    TuringRow,
};

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

/// Record ids this page reads. A missing id fails the build run and every
/// test, so a renamed row cannot silently drop a badge.
const TY2: &str = "TY-2";
const YIELD_TJ: &str = "TURING-YIELD-TJ";
const ENERGY: &str = "ENERGY-ATTRIBUTION";
const EXP_003: &str = "EXP-003";
const CEGIS: &str = "CEGIS";
const BAYES: &str = "Bayesian cost prediction (belief/estimation)";
const EGRAPH: &str = "Equality saturation";

fn trow(id: &str) -> &'static TuringRow {
    data().turing_row(id).unwrap_or_else(|| panic!("research status row {id} missing"))
}

fn irow(id: &str) -> &'static ImplementationRow {
    data()
        .implementation_row(id)
        .unwrap_or_else(|| panic!("research status implementation row {id} missing"))
}

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
    let ty2 = trow(TY2);
    let yield_tj = trow(YIELD_TJ);
    let energy = trow(ENERGY);
    html! {
        section id="quantities" class="narr-section" aria-labelledby="quantities-heading" {
            h2 id="quantities-heading" { "Three quantities" }
            ul class="narr-cards" {
                li class="narr-card" {
                    h3 { "Turing gain (T)" }
                    p {
                        "Did the machine learn explanatory structure? Gain is net held-out explanatory compression after paying for the explanation."
                    }
                    p class="narr-figure" {
                        (status_badge(ty2)) " Run TY-2 recorded " (ty2_headline()) " against an order-1 baseline on held-out seeds."
                    }
                    p {
                        "The re-check used the same tool, so it is a self re-derivation and still awaits an independent one. This result certifies the instrument; it does not show that AIEN discovered anything."
                    }
                }
                li class="narr-card" {
                    h3 { "Turing yield (T/J) " (status_badge(yield_tj)) }
                    p {
                        "What physical resources did that gain cost? Yield will divide Turings by joules, and may separate evaluation yield from discovery yield, the more important of the two."
                    }
                    p { (yield_tj.plain) }
                    p { (status_badge(energy)) " Energy attribution. " (energy.plain) }
                    p {
                        "No yield figure exists, and none will be claimed until energy attribution and the Turing receipt are joined under the qualifying protocol."
                    }
                }
                li class="narr-card" {
                    h3 { "Search and verification gap" }
                    p {
                        "How hard was the explanation to discover compared with verifying it once found? The idea to test: discovered explanatory structure reduces the cost of future search, through prediction, compression, abstraction, reusable procedure, reduced search and faster discovery."
                    }
                    p { "This is a research direction. No result on it exists yet." }
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
                "Today Cortex is an append-only store that records what the World did. Search history, counterexamples and costs are what it is meant to hold next; its current standing is listed under "
                a href="#today" { "What exists today" }
                "."
            }
            p { "With that evidence in place, AIEN will be able to ask:" }
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
        section id="research-ladder" class="narr-section" data-source="research_status.json" aria-labelledby="ladder-heading" {
            h2 id="ladder-heading" { "The discovery ladder" }
            p {
                "Each rung is a stronger claim about what the machine has shown, and each rests on the rung below. The ladder is drawn from the research status record, the same record behind the full status page, so this page never states a result by hand."
            }
            (render_research_status_summary())
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
            p { "Each of these is a connection or an opportunity. Where the research status record has a matching entry, its badge sits beside the name." }
            dl class="narr-connections" {
                dt { "Petri nets and reaction systems" }
                dd {
                    "The World lifecycle work already has a state model, reachable-state exploration, invariants, shortest counterexample traces, mutants and differential checking against the real World code. Formal concurrency theory may offer reachability, liveness and deadlock analysis, partial-order reduction and invariant proofs. The World keeps its own name; it is not being relabelled a Petri net."
                }
                dt { "Counterexample-guided synthesis (CEGIS) " (class_badge(irow(CEGIS))) }
                dd {
                    "Omega's search already runs candidate, transform, verify, accept or reject, and records the trace and cost. The opportunity: turn a failed candidate into a semantic counterexample, generalize it into a constraint, eliminate a whole family of candidates, and measure how much search that saves."
                }
                dt { "Bayesian adaptive search " (class_badge(irow(BAYES))) }
                dd {
                    "A Bayesian cost model for choosing between verified realizations is being tried against recorded signals, and its failed attempts stay on the record. A future direction is safe contextual exploration. Exploration must stay explicitly authorized, and production authority is never inferred from prediction confidence."
                }
                dt { "Active causal discovery " (status_badge(trow(EXP_003))) }
                dd {
                    "EXP-003 is designed as the move from passive prediction to active inquiry: hold competing explanations, predict their consequences, choose the intervention that best separates them, run one authorized experiment and update belief. "
                    (trow(EXP_003).plain)
                }
                dt { "E-graphs and equality saturation " (class_badge(irow(EGRAPH))) }
                dd {
                    "A research reference for treating rewritten expressions, basis changes, coordinate systems and equivalent implementations as one discovery. Omega has no e-graph code."
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
                "This list is drawn from the research status record, built from a check of the code on the main branches of Omega and AIENOS. Built work and open research questions are kept apart, and each entry opens to show its limits and evidence."
            }
            (render_implementation(data()))
            p class="narr-more" {
                a href="/research/status/#implementation" { "The same list on the research status page, beside every experiment." }
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
            p { a href="/discovery/" { "Next: Scientific Discovery, the examinations ahead." } }
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
    use crate::components::narrative::check::stray_status_words;
    use crate::layouts::base::render_page_layout;

    fn page() -> String {
        render_page_layout(TITLE, DESCRIPTION, URL, "article", html! {}, render_machine()).into_string()
    }

    /// Every piece of markup on /machine/ that is allowed to carry a status
    /// word: the shared components and every data-driven badge and sentence.
    fn data_renderings() -> Vec<String> {
        let d = data();
        let mut v = vec![
            render_research_status_summary().into_string(),
            render_implementation(d).into_string(),
        ];
        for r in &d.turing_rows {
            v.push(status_badge(r).into_string());
            v.push(html! { (r.plain) }.into_string());
        }
        for r in &d.implementation_rows {
            v.push(class_badge(r).into_string());
        }
        v.push(html! { (ty2_headline()) }.into_string());
        v
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
    /// mention yield. The figure itself comes from the data.
    #[test]
    fn gain_figure_is_never_called_yield() {
        let p = page();
        let fig = ty2_headline();
        let mut found = false;
        let mut from = 0;
        while let Some(rel) = p[from..].find(fig) {
            found = true;
            let at = from + rel;
            let start = ["<p", "<li"].iter().filter_map(|t| p[..at].rfind(*t)).max().unwrap_or(0);
            let end = ["</p>", "</li>"]
                .iter()
                .filter_map(|t| p[at..].find(*t).map(|e| at + e))
                .min()
                .unwrap_or(p.len());
            let block = p[start..end].to_lowercase();
            assert!(!block.contains("yield"), "TY-2 gain figure sits next to the word yield: {block}");
            from = at + fig.len();
        }
        assert!(found, "TY-2 gain figure missing from /machine/");
    }

    /// The research-ladder section holds the ladder rendered by
    /// research_status::render_ladder (class rs-ladder, one rs-step per level)
    /// and the link to the full status page.
    #[test]
    fn research_ladder_section_renders_the_canonical_ladder() {
        let p = page();
        let start = p.find("id=\"research-ladder\"").expect("research-ladder section missing");
        let end = start + p[start..].find("</section>").expect("research-ladder section not closed");
        let section = &p[start..end];
        assert!(section.contains(&render_research_status_summary().into_string()), "ladder markup differs from render_research_status_summary");
        assert!(section.contains("class=\"rs-ladder\""), "rs-ladder markup missing from research-ladder");
        assert_eq!(section.matches("class=\"rs-step ").count(), data().ladder.len(), "ladder steps missing");
        assert!(section.contains("href=\"/research/status/\""), "link to the full status page missing");
    }

    /// What exists today is the canonical implementation list, verbatim.
    #[test]
    fn today_section_renders_the_canonical_implementation_list() {
        let p = page();
        assert!(p.contains(&render_implementation(data()).into_string()));
        assert!(!p.contains("status-badge"), "old hand-kept badge markup is back");
    }

    /// No status word on /machine/ may come from anywhere but the data.
    #[test]
    fn no_hand_written_status_words() {
        let body = render_machine().into_string();
        let stray = stray_status_words(&body, &data_renderings());
        assert!(stray.is_empty(), "hand-written status words on /machine/: {stray:?}");
    }

    #[test]
    fn no_em_or_en_dashes() {
        let p = page();
        assert!(!p.contains('\u{2014}'), "em dash in /machine/");
        assert!(!p.contains('\u{2013}'), "en dash in /machine/");
    }
}
