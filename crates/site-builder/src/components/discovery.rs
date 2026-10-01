//! /discovery/ : "Scientific Discovery", the examinations of representation
//! discovery (synthesis handoff section 7). Each exam's badge comes from the
//! matching implementation row in data/research_status.json. An exam with no
//! row gets no badge and says so in plain words.

use maud::{html, Markup};

use super::narrative::class_badge;
use super::research_status::{data, ImplementationRow};

pub const TITLE: &str = "Scientific Discovery | AIENOS";
pub const DESCRIPTION: &str = "Physics Zero, Dirac, Zeta and supersymmetry as examinations of representation discovery. AIEN is never given the named answer; it gets generic machinery and must earn the structure.";
pub const URL: &str = "https://aienos.com/discovery/";

pub const NAMED_ANSWER_RULE: &str =
    "Never give AIEN the named answer when the experiment is intended to test discovery.";

/// Shown in place of a badge when the research status record has no row.
pub const NOT_ON_RECORD: &str = "Not on the status record yet.";

pub struct Exam {
    pub id: &'static str,
    pub name: &'static str,
    /// Implementation row id in the research status record, if it has one.
    pub record_id: Option<&'static str>,
    pub summary: &'static [&'static str],
    pub capabilities: &'static [&'static str],
}

pub const EXAMS: &[Exam] = &[
    Exam {
        id: "physics-zero",
        name: "Physics Zero",
        record_id: Some("Full Physics Zero"),
        summary: &[
            "Physics Zero comes last in the order of work. Like the other exams, it will follow the rule above.",
        ],
        capabilities: &[],
    },
    Exam {
        id: "dirac",
        name: "Dirac",
        record_id: None,
        summary: &[
            "There will be no Dirac primitive. The exam will provide generic capabilities and test whether the Dirac structure emerges because it earns explanatory power.",
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
        record_id: Some("SUSY / Zeta rediscovery"),
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
        record_id: Some("SUSY / Zeta rediscovery"),
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

fn record(exam: &Exam) -> Option<&'static ImplementationRow> {
    exam.record_id.and_then(|id| data().implementation_row(id))
}

pub fn render_discovery() -> Markup {
    html! {
        article class="narr-page" {
            div class="narr-column" {
                header class="narr-header" {
                    p class="narr-kicker" { "Scientific Discovery" }
                    h1 class="narr-title" { "Examinations of representation discovery" }
                    p class="narr-standfirst" {
                        "Physics Zero, Dirac, Zeta and supersymmetry are examinations the program means to run. None of them is an AIEN feature. Each one will test whether the machine can find a representation because it explains unseen observations better, after paying for its own complexity."
                    }
                }
                section id="rule" class="narr-section" aria-labelledby="rule-heading" {
                    h2 id="rule-heading" { "The rule" }
                    blockquote class="narr-rule" { p { (NAMED_ANSWER_RULE) } }
                    p {
                        "Each exam will supply generic mathematical machinery instead of the answer. A structure that emerges will count only if it earns positive Turings on held-out data. "
                        a href="/machine/#turing" { "How the Turing scores." }
                    }
                    p {
                        "Each exam carries its entry from the "
                        a href="/research/status/#implementation" { "research status record" }
                        ", where one exists."
                    }
                }
                @for exam in EXAMS.iter() {
                    section id=(exam.id) class="narr-section narr-exam" aria-labelledby=(format!("{}-heading", exam.id)) {
                        h2 id=(format!("{}-heading", exam.id)) {
                            (exam.name)
                            @if let Some(row) = record(exam) {
                                " " (class_badge(row))
                            }
                        }
                        @if record(exam).is_none() {
                            p class="narr-norecord" { (NOT_ON_RECORD) }
                        }
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
                        "The full design of these exams belongs to a later phase of the program, after the general mathematical machinery exists. Until then they remain designs and open questions. "
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
    use crate::components::narrative::check::stray_status_words;
    use crate::layouts::base::render_page_layout;

    fn page() -> String {
        render_page_layout(TITLE, DESCRIPTION, URL, "article", html! {}, render_discovery()).into_string()
    }

    /// Every exam that names a record id must find that row, so a typo or a
    /// renamed row cannot quietly drop a badge. Exams without a row show the
    /// plain "not on the record" line instead.
    #[test]
    fn exam_badges_come_from_the_data() {
        let p = page();
        for exam in EXAMS.iter() {
            match exam.record_id {
                Some(id) => {
                    let row = data()
                        .implementation_row(id)
                        .unwrap_or_else(|| panic!("{} names record id {id}, which the data does not have", exam.name));
                    assert!(p.contains(&class_badge(row).into_string()), "{} badge missing", exam.name);
                }
                None => {
                    let start = p.find(&format!("id=\"{}\"", exam.id)).expect("exam section missing");
                    let end = start + p[start..].find("</section>").expect("exam section not closed");
                    assert!(p[start..end].contains(NOT_ON_RECORD), "{} needs the not-on-record line", exam.name);
                }
            }
        }
    }

    /// No status word on /discovery/ may come from anywhere but the data.
    #[test]
    fn no_hand_written_status_words() {
        let body = render_discovery().into_string();
        let badges: Vec<String> = data().implementation_rows.iter().map(|r| class_badge(r).into_string()).collect();
        let stray = stray_status_words(&body, &badges);
        assert!(stray.is_empty(), "hand-written status words on /discovery/: {stray:?}");
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
