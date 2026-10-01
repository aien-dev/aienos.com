//! /discovery/ : "Scientific Discovery", the planned examinations of
//! representation discovery (synthesis handoff section 7). Status labels come
//! only from ~/handoffs/2026-10-01-NARR-IMPL-scout.md: Physics Zero and the
//! Dirac test are PLANNED (the Dirac oracle was never built); Zeta and SUSY
//! rediscovery are HYPOTHESIS (synthesis handoff section 19).

use maud::{html, Markup};

use super::narrative::{badge, status_legend, Status};

pub const TITLE: &str = "Scientific Discovery | AIENOS";
pub const DESCRIPTION: &str = "Physics Zero, Dirac, Zeta and supersymmetry as planned examinations of representation discovery. AIEN is never given the named answer; it gets generic machinery and must earn the structure.";
pub const URL: &str = "https://aienos.com/discovery/";

pub const NAMED_ANSWER_RULE: &str =
    "Never give AIEN the named answer when the experiment is intended to test discovery.";

pub struct Exam {
    pub id: &'static str,
    pub name: &'static str,
    pub status: Status,
    pub summary: &'static [&'static str],
    pub capabilities: &'static [&'static str],
}

pub const EXAMS: &[Exam] = &[
    Exam {
        id: "physics-zero",
        name: "Physics Zero",
        status: Status::Planned,
        summary: &[
            "Physics Zero is a planned examination and comes last in the order of work. Like the other exams, it will follow the rule above.",
        ],
        capabilities: &[],
    },
    Exam {
        id: "dirac",
        name: "Dirac",
        status: Status::Planned,
        summary: &[
            "There will be no Dirac primitive. The exam will provide generic capabilities and test whether the Dirac structure emerges because it earns explanatory power.",
            "A reference oracle for this test has been written but never built or run.",
        ],
        capabilities: &[
            "complex values",
            "multi-component state",
            "linear operators",
            "matrix composition",
            "noncommutative composition",
            "commutators and anticommutators",
            "metric and signature",
            "differential operators",
            "constraints",
            "basis transformations",
        ],
    },
    Exam {
        id: "zeta",
        name: "Zeta and spectral geometry",
        status: Status::Hypothesis,
        summary: &[
            "The machine will not be taught the Riemann hypothesis, Zeta Space or the critical line. The exam will test representation transitions: analytic, then operator, then spectral, then geometric or topological.",
            "A transition will count only if it improves withheld explanatory performance after its complexity is charged.",
        ],
        capabilities: &[
            "complex analysis",
            "operators",
            "spectra",
            "graphs",
            "topology",
            "group action",
            "noncommutative composition",
            "integral transforms",
            "symmetry constraints",
        ],
    },
    Exam {
        id: "susy",
        name: "Supersymmetry",
        status: Status::Hypothesis,
        summary: &[
            "There will be no hard-coded supersymmetry theory. The exam will ask whether the system discovers a relationship between classes of state because that relationship compresses unseen behavior.",
        ],
        capabilities: &[
            "graded structures",
            "multiple state sectors",
            "graded commutators",
            "Grassmann-style machinery",
            "symmetry generators",
            "field and operator relations",
        ],
    },
];

pub fn render_discovery() -> Markup {
    html! {
        article class="narr-page" {
            div class="narr-column" {
                header class="narr-header" {
                    p class="narr-kicker" { "Scientific Discovery" }
                    h1 class="narr-title" { "Examinations of representation discovery" }
                    p class="narr-standfirst" {
                        "Physics Zero, Dirac, Zeta and supersymmetry are planned examinations. None of them is an AIEN feature. Each one will test whether the machine can find a representation because it explains unseen observations better, after paying for its own complexity."
                    }
                }
                section id="rule" class="narr-section" aria-labelledby="rule-heading" {
                    h2 id="rule-heading" { "The rule" }
                    blockquote class="narr-rule" { p { (NAMED_ANSWER_RULE) } }
                    p {
                        "Each exam will supply generic mathematical machinery instead of the answer. A structure that emerges will count only if it earns positive Turings on held-out data. "
                        a href="/machine/#turing" { "How the Turing scores." }
                    }
                    (status_legend())
                }
                @for exam in EXAMS.iter() {
                    section id=(exam.id) class=(format!("narr-section narr-exam {}", exam.status.class())) aria-labelledby=(format!("{}-heading", exam.id)) {
                        h2 id=(format!("{}-heading", exam.id)) { (exam.name) " " (badge(exam.status)) }
                        @for para in exam.summary.iter() {
                            p { (para) }
                        }
                        @if !exam.capabilities.is_empty() {
                            h3 { "Generic capabilities it will receive" }
                            ul class="narr-caps" {
                                @for cap in exam.capabilities.iter() {
                                    li { (cap) }
                                }
                            }
                        }
                    }
                }
                section id="later" class="narr-section" aria-labelledby="later-heading" {
                    h2 id="later-heading" { "When" }
                    p {
                        "The full design of these exams belongs to a later phase of the program, after the general mathematical machinery exists. Until then they stay planned work and open questions. "
                        a href="/machine/#today" { "See what exists today." }
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
        render_page_layout(TITLE, DESCRIPTION, URL, "article", html! {}, render_discovery()).into_string()
    }

    /// Every exam is planned or hypothesis per the NARR-IMPL scout table;
    /// none may be promoted to implemented, partial or experimental.
    #[test]
    fn exams_are_only_planned_or_hypothesis() {
        for exam in EXAMS.iter() {
            assert!(
                matches!(exam.status, Status::Planned | Status::Hypothesis),
                "{} must be badged planned or hypothesis",
                exam.name
            );
        }
        let p = page();
        assert!(!p.contains("narr-exam status-implemented"));
    }

    #[test]
    fn named_answer_rule_is_present() {
        assert!(page().contains(NAMED_ANSWER_RULE));
    }

    #[test]
    fn no_em_or_en_dashes() {
        let p = page();
        assert!(!p.contains('\u{2014}'), "em dash in /discovery/");
        assert!(!p.contains('\u{2013}'), "en dash in /discovery/");
    }
}
