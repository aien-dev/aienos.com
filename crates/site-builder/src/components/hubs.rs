//! Section hub pages: /experiments/, /evidence/, /philosophy/ (NARR-NAV).
//!
//! Short generated pages that share the site header and footer through
//! `render_page_layout`. They carry no experiment status values: those
//! will come from the canonical status source (LT-TRUTH), never from prose.

use std::fs;
use std::path::Path;

use maud::{html, Markup};

use crate::layouts::base::render_page_layout;

const PAPER: &str = "/research/computing-machinery-and-understanding/";
const QUALIFICATION_RECORD: &str =
    "https://github.com/aien-dev/omega/blob/main/docs/turing/TURING_SCIENTIFIC_QUALIFICATION_STATE.md";

/// The phrase each hub must carry. verify_hub_pages and the tests check it.
pub const EXPERIMENTS_SLOT: &str = "id=\"experiment-cards\"";
pub const EVIDENCE_PHRASE: &str = "Failure is evidence.";
pub const PHILOSOPHY_PHRASE: &str = "Position, not evidence";

pub struct Hub {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub required: &'static str,
    pub render: fn() -> Markup,
}

pub static HUBS: [Hub; 3] = [
    Hub {
        slug: "experiments",
        title: "Experiments | AIENOS",
        description: "Every experiment on the AIEN research ladder, failures included, rendered from one canonical status source.",
        required: EXPERIMENTS_SLOT,
        render: render_experiments_hub,
    },
    Hub {
        slug: "evidence",
        title: "Evidence | AIENOS",
        description: "What counts as evidence for AIEN, which source wins when two disagree, and where to inspect the receipts, frozen profiles, and qualification record.",
        required: EVIDENCE_PHRASE,
        render: render_evidence_hub,
    },
    Hub {
        slug: "philosophy",
        title: "Philosophy | AIENOS",
        description: "Position papers behind AIEN: the Stapleton Doctrine and the Post-LLM Case. These argue a stance and are not experimental results.",
        required: PHILOSOPHY_PHRASE,
        render: render_philosophy_hub,
    },
];

pub fn render_experiments_hub() -> Markup {
    html! {
        div class="container hub-wrap" {
            p class="hub-kicker" { "Experiments" }
            h1 class="hub-title" { "Experiments" }
            p class="hub-standfirst" {
                "Every experiment on the AIEN research ladder belongs here with its protocol, its scope, and its outcome, including the ones that failed. The list will render from one canonical status source, so this page cannot drift from the qualification record."
            }
            section id="experiment-cards" class="hub-slot" data-source="LT-TRUTH status source (pending)" aria-labelledby="experiment-cards-title" {
                h2 id="experiment-cards-title" class="hub-h2" { "Experiment cards" }
                p {
                    "The experiment list is being connected to the canonical status source. Until that lands, the current ladder lives on the Turing paper page."
                }
                a class="hub-button" href=(PAPER) { "See the current experiment ladder" }
            }
            section class="hub-section" aria-labelledby="experiments-more" {
                h2 id="experiments-more" class="hub-h2" { "Check it yourself" }
                p {
                    "Each experiment is frozen before it is scored, and its record stays public whatever the outcome. The "
                    a href="/evidence/" { "Evidence" }
                    " page explains which sources count and where to find them."
                }
            }
        }
    }
}

pub fn render_evidence_hub() -> Markup {
    let ladder: [(&str, &str); 7] = [
        ("Code", "The source that actually runs."),
        ("Tests", "Checks that run the code and are able to fail."),
        ("Receipts", "Hashed records of a specific run and what it produced."),
        ("Frozen profiles", "Experiment protocols fixed before any scoring happens."),
        ("Qualification records", "The running ledger of what has qualified, and in what scope."),
        ("Current docs", "Design notes and plans. They describe intent."),
        ("Website prose", "Including this page. The weakest source on the list."),
    ];
    let artifacts: [(&str, &str, String); 5] = [
        ("SHA256SUMS", "Checksums for every published artifact of the Turing paper.", format!("{PAPER}SHA256SUMS")),
        ("EXP-001 frozen profile", "The protocol for EXP-001, as frozen before scoring.", format!("{PAPER}profiles/EXP-001.json")),
        ("EXP-002 frozen profile", "The protocol for EXP-002, as frozen before scoring.", format!("{PAPER}profiles/EXP-002.json")),
        ("EXP-003 frozen profile", "The protocol for EXP-003, as published.", format!("{PAPER}profiles/EXP-003.json")),
        ("The Turing paper (PDF)", "Definitions, proofs, measurement protocol, and falsification criteria.", format!("{PAPER}paper.pdf")),
    ];
    html! {
        div class="container hub-wrap" {
            p class="hub-kicker" { "Evidence" }
            h1 class="hub-title" { "Evidence" }
            p class="hub-standfirst" {
                "Every claim on this site should trace back to something you can inspect yourself. When two sources disagree, the one higher on this list wins."
            }
            section class="hub-section" aria-labelledby="evidence-order" {
                h2 id="evidence-order" class="hub-h2" { "What counts as evidence" }
                ol class="hub-ladder" {
                    @for (name, what) in ladder {
                        li {
                            strong { (name) }
                            span { (what) }
                        }
                    }
                }
                p class="hub-note" { "Website prose never overrides code, tests, or receipts." }
            }
            section class="hub-section" aria-labelledby="evidence-where" {
                h2 id="evidence-where" class="hub-h2" { "Where to inspect it" }
                ul class="hub-cards" {
                    @for (name, what, href) in &artifacts {
                        li class="hub-card" {
                            a href=(href) { (name) }
                            p { (what) }
                        }
                    }
                    li class="hub-card" {
                        a href=(QUALIFICATION_RECORD) target="_blank" rel="noopener noreferrer" { "Omega qualification record" }
                        p { "The canonical record of what has qualified and in what scope, kept in the Omega repository on GitHub." }
                    }
                }
            }
            section class="hub-section hub-callout" aria-labelledby="evidence-failure" {
                h2 id="evidence-failure" class="hub-h2" { (EVIDENCE_PHRASE) }
                p {
                    "Failed and incomplete experiments stay on the record. They are never deleted, hidden, or rewritten into success. A meter you cannot fail is not a meter."
                }
            }
        }
    }
}

pub fn render_philosophy_hub() -> Markup {
    let cards: [(&str, &str, &str, bool); 3] = [
        (
            "The Stapleton Doctrine",
            "/research/the-stapleton-doctrine/",
            "English is the conversation, not the language. The universal artifact is the verified program.",
            false,
        ),
        (
            "The Post-LLM Case",
            "/post-llm-case/",
            "Why scaling centralized language prediction cannot be the only route to machine intelligence, and what a different learning machine would look like.",
            false,
        ),
        (
            "Drake Stapleton on philosophy",
            "https://www.drakestapleton.com/philosophy",
            "Longer essays on ownership, sovereignty, and how machines should earn trust, on drakestapleton.com.",
            true,
        ),
    ];
    html! {
        div class="container hub-wrap" {
            p class="hub-label hub-label-position" { (PHILOSOPHY_PHRASE) }
            h1 class="hub-title" { "Philosophy" }
            p class="hub-standfirst" {
                "These pages argue a stance and are not experimental results. For what has been measured, see "
                a href="/evidence/" { "Evidence" }
                "."
            }
            ul class="hub-cards" {
                @for (name, href, what, external) in cards {
                    li class="hub-card" {
                        @if external {
                            a href=(href) target="_blank" rel="noopener noreferrer" { (name) }
                        } @else {
                            a href=(href) { (name) }
                        }
                        p { (what) }
                    }
                }
            }
        }
    }
}

fn render_hub_document(hub: &Hub) -> String {
    render_page_layout(
        hub.title,
        hub.description,
        &format!("https://aienos.com/{}/", hub.slug),
        "website",
        html! {},
        (hub.render)(),
    )
    .into_string()
}

/// Writes every hub page to `<dist>/<slug>/index.html`.
pub fn emit_hub_pages(dist: &Path) {
    for hub in &HUBS {
        let dir = dist.join(hub.slug);
        fs::create_dir_all(&dir).unwrap_or_else(|_| panic!("Failed to create {}", dir.display()));
        fs::write(dir.join("index.html"), render_hub_document(hub))
            .unwrap_or_else(|_| panic!("Failed to write /{}/index.html", hub.slug));
        println!("  [PAGE] Emitted /{}/ (index.html)", hub.slug);
    }
}

/// Fails the build if a hub page is missing, lacks its required phrase,
/// lacks any of the six nav sections, or contains an em or en dash.
pub fn verify_hub_pages(dist: &Path) {
    for hub in &HUBS {
        let path = dist.join(hub.slug).join("index.html");
        let page = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Missing hub page {}", path.display()));
        check_hub_document(hub, &page).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

fn check_hub_document(hub: &Hub, page: &str) -> Result<(), String> {
    if !page.contains(hub.required) {
        return Err(format!("missing required phrase {:?}", hub.required));
    }
    if page.contains('\u{2014}') || page.contains('\u{2013}') {
        return Err("forbidden em or en dash".to_string());
    }
    for (label, href) in crate::components::navbar::NAV_PRIMARY {
        if !page.contains(&crate::components::navbar::primary_link_markup(label, href)) {
            return Err(format!("nav item {label} missing"));
        }
    }
    for status in ["PASS", "FAIL", "BLOCKED", "INCOMPLETE"] {
        if page.contains(status) {
            return Err(format!("hand-written status value {status} on a hub page"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(slug: &str) -> String {
        let hub = HUBS.iter().find(|h| h.slug == slug).expect("hub registered");
        render_hub_document(hub)
    }

    #[test]
    fn three_hubs_are_registered() {
        let slugs: Vec<&str> = HUBS.iter().map(|h| h.slug).collect();
        assert_eq!(slugs, ["experiments", "evidence", "philosophy"]);
    }

    #[test]
    fn experiments_hub_has_the_pending_card_slot() {
        let page = doc("experiments");
        assert!(page.contains("<section id=\"experiment-cards\""), "experiment-cards slot missing");
        assert!(page.contains("data-source=\"LT-TRUTH status source (pending)\""), "slot data-source missing");
        assert!(page.contains(PAPER), "link to the current ladder missing");
    }

    #[test]
    fn evidence_hub_says_failure_is_evidence() {
        let page = doc("evidence");
        assert!(page.contains("Failure is evidence."), "required phrase missing");
        assert!(page.contains("Failed and incomplete experiments stay on the record."));
        assert!(page.contains(QUALIFICATION_RECORD), "qualification record link missing");
        assert!(page.contains("SHA256SUMS"), "checksum link missing");
    }

    #[test]
    fn evidence_hierarchy_is_in_order() {
        let page = doc("evidence");
        let order = ["Code", "Tests", "Receipts", "Frozen profiles", "Qualification records", "Current docs", "Website prose"];
        let mut last = 0usize;
        for name in order {
            let needle = format!("<strong>{name}</strong>");
            let pos = page[last..]
                .find(&needle)
                .unwrap_or_else(|| panic!("evidence level {name} missing or out of order"));
            last += pos + needle.len();
        }
    }

    #[test]
    fn philosophy_hub_is_labelled_position_not_evidence() {
        let page = doc("philosophy");
        assert!(page.contains("Position, not evidence"), "required label missing");
        assert!(page.contains("are not experimental results"), "stance sentence missing");
        for href in ["/research/the-stapleton-doctrine/", "/post-llm-case/", "https://www.drakestapleton.com/philosophy"] {
            assert!(page.contains(&format!("href=\"{href}\"")), "philosophy card {href} missing");
        }
    }

    #[test]
    fn every_hub_passes_the_build_check() {
        for hub in &HUBS {
            let page = render_hub_document(hub);
            check_hub_document(hub, &page).unwrap_or_else(|e| panic!("/{}/: {e}", hub.slug));
        }
    }

    #[test]
    fn hubs_contain_no_em_or_en_dash() {
        for hub in &HUBS {
            let page = render_hub_document(hub);
            assert!(!page.contains('\u{2014}'), "/{}/ contains an em dash", hub.slug);
            assert!(!page.contains('\u{2013}'), "/{}/ contains an en dash", hub.slug);
        }
    }

    #[test]
    fn build_check_rejects_a_dash_and_a_missing_phrase() {
        let hub = &HUBS[1];
        let page = render_hub_document(hub);
        assert!(check_hub_document(hub, &page.replace("Failure is evidence.", "")).is_err());
        assert!(check_hub_document(hub, &format!("{page}\u{2014}")).is_err());
    }
}
