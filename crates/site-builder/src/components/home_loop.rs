//! Homepage top (NARR-HOME): the central statement, the short public
//! thesis, the slogan, the canonical loop diagram and the three questions.
//!
//! Wording comes from the pre-approved research synthesis handoff
//! (2026-10-01), sections Mission, 2, 14 and 15. Do not add claims here.

use maud::{html, Markup};

/// Central statement, first sentence (the page h1).
pub const LEAD: &str =
    "AIENOS is an owned experimental machine for turning search into verified understanding.";

/// Central statement, the sentences after the lead.
pub const LEAD_REST: &str = "AIEN proposes structure. Omega makes that structure explicit and checkable. The machine tests it against reality under authority. The Turing measures what was actually learned. Cortex keeps the evidence. Then the next search begins from what survived.";

/// The short public thesis, split into readable paragraphs.
pub const THESIS: [&str; 3] = [
    "AIENOS is an attempt to build persistent machine intelligence as a scientific process rather than a model invocation. Neural machinery may propose, but proposals have no authority.",
    "Omega turns them into explicit programs and theories; verification determines whether they are internally valid; the owned machine tests them against physical reality; the Turing measures whether they explain unseen observations after paying for their own complexity; Cortex preserves the evidence; and that verified experience changes the next search.",
    "Atlas carries the lineage across hardware. The goal is not a machine that sounds as though it understands, but a machine whose growth in explanatory power can be measured, falsified, reproduced, and owned.",
];

pub const SLOGAN: &str =
    "Weights suggest. Programs explain. Verification decides. Evidence teaches. The Turing keeps score.";

/// Which band of the diagram a stage belongs to.
#[derive(Clone, Copy)]
pub enum Band {
    /// Generation path: lineage, substrate, proposal, formalization, realization.
    Generation,
    /// Contact with the physical world.
    Reality,
    /// Evidence feedback: scoring and preservation.
    Feedback,
}

impl Band {
    fn class(self) -> &'static str {
        match self {
            Band::Generation => "loop-stage loop-stage-gen",
            Band::Reality => "loop-stage loop-stage-reality",
            Band::Feedback => "loop-stage loop-stage-feedback",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Band::Generation => "Generation path",
            Band::Reality => "Contact with reality",
            Band::Feedback => "Evidence feedback",
        }
    }
}

/// The canonical loop (handoff section 14), in order.
pub const STAGES: [(&str, &str, Band); 8] = [
    ("ATLAS", "awakens / lineage", Band::Generation),
    ("AIENOS", "persistent owned substrate", Band::Generation),
    ("AIEN", "proposes / guides search", Band::Generation),
    ("OMEGA", "formalizes / searches / verifies meaning", Band::Generation),
    ("FORGE", "realizes", Band::Generation),
    ("REALITY", "executes / responds", Band::Reality),
    ("TURING", "scores explanatory gain", Band::Feedback),
    ("CORTEX", "preserves evidence", Band::Feedback),
];

/// What the last stage hands back to.
pub const RETURN_TO: &str = "next AIEN / Omega search";

/// The verification and authority perimeter. These surround the loop and
/// are never stages. Split into the band drawn above and below the loop.
pub const PERIMETER_TOP: [(&str, &str); 2] = [
    ("AEGIS", "authority + proof boundaries"),
    ("ARGUS", "observation / audit"),
];
pub const PERIMETER_BOTTOM: [(&str, &str); 2] = [
    ("Capability system", "controls what may happen"),
    ("Evidence receipts", "make outcomes reproducible"),
];

/// Status of a question, shown as a text badge plus a distinct style.
#[derive(Clone, Copy)]
pub enum QStatus {
    Experimental,
    NotYetMeasured,
    ResearchDirection,
}

impl QStatus {
    fn class(self) -> &'static str {
        match self {
            QStatus::Experimental => "home-q-badge status-experimental",
            QStatus::NotYetMeasured => "home-q-badge status-planned",
            QStatus::ResearchDirection => "home-q-badge status-hypothesis",
        }
    }
    fn label(self) -> &'static str {
        match self {
            QStatus::Experimental => "Experimental",
            QStatus::NotYetMeasured => "Not yet measured",
            QStatus::ResearchDirection => "Research direction",
        }
    }
}

/// The three questions (handoff section 15): question, measure, meaning,
/// status, optional status note.
pub const QUESTIONS: [(&str, &str, &str, QStatus, Option<&str>); 3] = [
    (
        "Did it learn anything?",
        "Turing gain",
        "Net explanatory structure that survives held-out reality.",
        QStatus::Experimental,
        None,
    ),
    (
        "What did it cost to learn?",
        "Turing yield",
        "Verified explanatory gain per measured physical resource.",
        QStatus::NotYetMeasured,
        Some("Yield (T/J) is not started in the Omega qualification record. No yield figure is claimed."),
    ),
    (
        "Did what it learned make future discovery easier?",
        "Search / verification gap",
        "Whether acquired abstraction would collapse future search complexity.",
        QStatus::ResearchDirection,
        Some("A research direction. Nothing here is measured yet."),
    ),
];

fn perimeter_band(items: &[(&str, &str)], label: &str) -> Markup {
    html! {
        ul class="loop-perimeter-band" aria-label=(label) {
            @for (name, role) in items {
                li class="loop-guard" {
                    span class="loop-guard-name" { (name) }
                    span class="loop-guard-role" { (role) }
                }
            }
        }
    }
}

pub fn render_home_loop() -> Markup {
    html! {
        section id="top" class="home-top bg-grid" aria-labelledby="home-lead" {
            div class="container" {
                div class="home-column" {
                    div class="home-badges" {
                        span class="badge badge-amber" { "Experimental · pre-alpha" }
                        span class="badge badge-blue" { "Open source · Apache-2.0 with LLVM-exception" }
                    }

                    h1 id="home-lead" class="home-lead" { (LEAD) }
                    p class="home-lead-rest" { (LEAD_REST) }

                    div class="home-thesis" {
                        @for para in THESIS.iter() {
                            p { (para) }
                        }
                    }
                }

                p class="home-slogan" { (SLOGAN) }

                figure class="loop" aria-labelledby="loop-title" {
                    h2 id="loop-title" class="loop-title" { "The loop" }
                    p class="loop-intro" {
                        "Proposals travel the generation path, meet reality, and are scored. The evidence that survives starts the next search. A perimeter of authority, observation and receipts surrounds the whole loop."
                    }

                    ul class="loop-legend" aria-label="How to read the diagram" {
                        li class="loop-key loop-key-gen" { "Generation path" }
                        li class="loop-key loop-key-feedback" { "Evidence feedback" }
                        li class="loop-key loop-key-perimeter" { "Verification and authority perimeter" }
                    }

                    div class="loop-perimeter" {
                        p class="loop-perimeter-label" {
                            "Perimeter: around the loop, never a stage"
                        }
                        (perimeter_band(&PERIMETER_TOP, "Perimeter, authority and observation"))

                        ol class="loop-stages" aria-label="Loop stages, in order" {
                            @for (name, role, band) in STAGES.iter() {
                                li class=(band.class()) {
                                    span class="loop-stage-name" { (name) }
                                    span class="loop-stage-role" { (role) }
                                    span class="loop-stage-band" { (band.label()) }
                                }
                            }
                        }

                        p class="loop-return" {
                            span class="loop-return-arrow" aria-hidden="true" { "\u{21BA}" }
                            span { "Back to the " strong { (RETURN_TO) } ": CORTEX evidence feeds the next round." }
                        }

                        (perimeter_band(&PERIMETER_BOTTOM, "Perimeter, capabilities and receipts"))
                    }
                    figcaption class="loop-caption" {
                        "ATLAS, AIENOS, AIEN, OMEGA, FORGE, REALITY, TURING and CORTEX are the stages. AEGIS, ARGUS, the capability system and evidence receipts form the perimeter around them."
                    }
                }

                div class="home-questions" {
                    h2 class="home-questions-title" { "Three questions under the loop" }
                    ol class="home-q-list" {
                        @for (question, measure, meaning, status, note) in QUESTIONS.iter() {
                            li class="home-q" {
                                p class="home-q-question" { (question) }
                                p class="home-q-measure" {
                                    span class="home-q-measure-name" { (measure) }
                                    " "
                                    span class=(status.class()) { (status.label()) }
                                }
                                p class="home-q-meaning" { (meaning) }
                                @if let Some(n) = note {
                                    p class="home-q-note" { (n) }
                                }
                            }
                        }
                    }
                }

                div class="home-actions" {
                    a class="home-cta home-cta-primary" href="#evidence" { "See the evidence" }
                    a class="home-cta" href="#architecture" { "Explore the architecture" }
                    a class="home-cta home-cta-ghost" href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" { "GitHub" }
                }
                ul class="home-checks" {
                    li { "C kernel boots via UEFI \u{2713}" }
                    li { "Every claim carries a receipt \u{2713}" }
                    li { "Failures published, not hidden \u{2713}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected strings are written out here on purpose, not read from the
    // constants above, so editing a constant cannot silently pass the test.

    fn page() -> String {
        render_home_loop().into_string()
    }

    fn stage_list(s: &str) -> &str {
        let start = s.find("<ol class=\"loop-stages\"").expect("stages must be an <ol>");
        let end = s[start..].find("</ol>").expect("stage list closes") + start;
        &s[start..end]
    }

    #[test]
    fn central_statement_leads_and_thesis_follows() {
        let s = page();
        let lead = s
            .find("AIENOS is an owned experimental machine for turning search into verified understanding.")
            .expect("central statement missing");
        assert!(s[..lead].contains("<h1"), "central statement must be the h1 lead");
        assert_eq!(s.matches("<h1").count(), 1, "exactly one h1");
        for needle in [
            "Then the next search begins from what survived.",
            "AIENOS is an attempt to build persistent machine intelligence as a scientific process rather than a model invocation.",
            "Neural machinery may propose, but proposals have no authority.",
            "Atlas carries the lineage across hardware.",
        ] {
            let at = s.find(needle).unwrap_or_else(|| panic!("missing: {needle}"));
            assert!(at > lead, "must come after the lead: {needle}");
        }
    }

    #[test]
    fn slogan_present_after_thesis_and_before_loop() {
        let s = page();
        let slogan = "Weights suggest. Programs explain. Verification decides. Evidence teaches. The Turing keeps score.";
        let at = s
            .find(&format!("<p class=\"home-slogan\">{slogan}</p>"))
            .expect("slogan missing or not in its display element");
        assert!(at > s.find("Atlas carries the lineage").unwrap());
        assert!(at < s.find("<ol class=\"loop-stages\"").unwrap());
    }

    #[test]
    fn eight_stages_in_canonical_order() {
        let s = page();
        let ol = stage_list(&s);
        let expected = [
            ("ATLAS", "awakens / lineage"),
            ("AIENOS", "persistent owned substrate"),
            ("AIEN", "proposes / guides search"),
            ("OMEGA", "formalizes / searches / verifies meaning"),
            ("FORGE", "realizes"),
            ("REALITY", "executes / responds"),
            ("TURING", "scores explanatory gain"),
            ("CORTEX", "preserves evidence"),
        ];
        assert_eq!(ol.matches("<li").count(), 8, "exactly 8 stages");
        let mut from = 0;
        for (name, role) in expected {
            let needle = format!(
                "<span class=\"loop-stage-name\">{name}</span><span class=\"loop-stage-role\">{role}</span>"
            );
            let at = ol[from..]
                .find(&needle)
                .unwrap_or_else(|| panic!("stage {name} missing or out of order"))
                + from;
            from = at + needle.len();
        }
    }

    #[test]
    fn return_arrow_follows_the_last_stage() {
        let s = page();
        let ol_end = s.find("<ol class=\"loop-stages\"").unwrap() + stage_list(&s).len();
        let after = &s[ol_end..];
        let ret = after.find("class=\"loop-return\"").expect("return marker missing");
        assert!(after[ret..].contains('\u{21BA}'), "return arrow glyph missing");
        assert!(after[ret..].contains("next AIEN / Omega search"), "return target missing");
    }

    #[test]
    fn perimeter_is_drawn_outside_the_stage_list() {
        let s = page();
        let ol = stage_list(&s);
        let perim_start = s.find("<div class=\"loop-perimeter\"").expect("perimeter frame missing");
        for name in ["AEGIS", "ARGUS", "Capability system", "Evidence receipts"] {
            let needle = format!("<span class=\"loop-guard-name\">{name}</span>");
            let at = s.find(&needle).unwrap_or_else(|| panic!("perimeter item {name} missing"));
            assert!(at > perim_start, "{name} must sit inside the perimeter frame");
            assert!(!ol.contains(name), "{name} must not be a stage");
        }
        assert!(!ol.contains("capability"), "capability system must not be a stage");
        assert!(!ol.contains("receipts"), "evidence receipts must not be a stage");
    }

    #[test]
    fn three_questions_directly_under_the_diagram() {
        let s = page();
        let fig_end = s.find("</figure>").expect("diagram figure missing");
        let mut from = fig_end;
        for q in [
            "Did it learn anything?",
            "What did it cost to learn?",
            "Did what it learned make future discovery easier?",
        ] {
            let at = s[from..].find(q).unwrap_or_else(|| panic!("question missing or out of order: {q}")) + from;
            from = at + q.len();
        }
        let checks = s.find("class=\"home-actions\"").unwrap();
        assert!(from < checks, "questions must sit directly under the diagram");
    }

    #[test]
    fn yield_and_gap_are_labelled_unmeasured() {
        let s = page();
        let y = s.find("Turing yield").expect("Turing yield missing");
        let gap = s.find("Search / verification gap").expect("gap missing");
        assert!(y < gap);
        let yield_block = &s[y..gap];
        assert!(yield_block.contains("status-planned"), "yield must carry the planned style");
        assert!(yield_block.contains("Not yet measured"), "yield must be labelled not yet measured");
        assert!(yield_block.contains("not started"), "yield must say not started");
        let gap_block = &s[gap..s.find("class=\"home-actions\"").unwrap()];
        assert!(gap_block.contains("status-hypothesis"));
        assert!(gap_block.contains("Research direction"));
        // Gain is never called yield.
        let gain = s.find("Turing gain").expect("Turing gain missing");
        assert!(!s[gain..y].contains("yield"), "gain block must not mention yield");
    }

    #[test]
    fn no_em_or_en_dashes_in_output() {
        let s = page();
        assert!(!s.contains('\u{2014}'), "em dash in homepage top");
        assert!(!s.contains('\u{2013}'), "en dash in homepage top");
        assert!(!s.contains("&mdash;") && !s.contains("&ndash;"), "dash entity in homepage top");
    }
}
