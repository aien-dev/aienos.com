//! Section hub pages: /experiments/, /evidence/, /philosophy/ (NARR-NAV).
//!
//! Short generated pages that share the site header and footer through
//! `render_page_layout`. No status word is written in this file. The
//! experiments hub renders its ladder and cards from the canonical status
//! source (`research_status`, data/research_status.json); the other hubs
//! carry no status values at all and only link to that record.

use std::fs;
use std::path::Path;

use maud::{html, Markup};

use crate::components::research_status as rs;
use crate::layouts::base::render_page_layout;

const PAPER: &str = "/research/computing-machinery-and-understanding/";
const QUALIFICATION_RECORD: &str =
    "https://github.com/aien-dev/omega/blob/main/docs/turing/TURING_SCIENTIFIC_QUALIFICATION_STATE.md";
const STATUS_PAGE: &str = "/research/status/";
const STATUS_JSON: &str = "/research-status.json";

/// Markup each hub must carry. verify_hub_pages and the tests check it.
pub const EXPERIMENTS_SLOT: &str = "<section id=\"experiment-cards\"";
/// The wrapper render_cards emits around the experiment cards.
pub const CARDS_MARKUP: &str = "<div class=\"rs-cards\">";
/// The full-record button on the experiments hub.
pub const STATUS_BUTTON: &str = "<a class=\"hub-button\" href=\"/research/status/\">";
pub const EVIDENCE_PHRASE: &str = "Failure is evidence.";
pub const STATUS_PAGE_LINK: &str = "href=\"/research/status/\"";
pub const STATUS_JSON_LINK: &str = "href=\"/research-status.json\"";
pub const PHILOSOPHY_PHRASE: &str = "Position, not evidence";

/// Where a hub's status values come from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusSource {
    /// Every status on the page is rendered from research_status data.
    Data,
    /// The page shows no status values at all.
    Absent,
}

pub struct Hub {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub required: &'static [&'static str],
    pub status: StatusSource,
    pub render: fn() -> Markup,
}

pub static HUBS: [Hub; 3] = [
    Hub {
        slug: "experiments",
        title: "Experiments | AIENOS",
        description: "Every experiment on the AIEN research ladder, failures included, rendered from one canonical status source.",
        required: &[EXPERIMENTS_SLOT, CARDS_MARKUP, STATUS_BUTTON],
        status: StatusSource::Data,
        render: render_experiments_hub,
    },
    Hub {
        slug: "evidence",
        title: "Evidence | AIENOS",
        description: "What counts as evidence for AIEN, which source wins when two disagree, and where to inspect the receipts, frozen profiles, research status record, and qualification record.",
        required: &[EVIDENCE_PHRASE, STATUS_PAGE_LINK, STATUS_JSON_LINK],
        status: StatusSource::Absent,
        render: render_evidence_hub,
    },
    Hub {
        slug: "philosophy",
        title: "Philosophy | AIENOS",
        description: "Position papers behind AIEN: the Stapleton Doctrine and the Post-LLM Case. These argue a stance and are not experimental results.",
        required: &[PHILOSOPHY_PHRASE],
        status: StatusSource::Absent,
        render: render_philosophy_hub,
    },
];

pub fn render_experiments_hub() -> Markup {
    let d = rs::data();
    html! {
        div class="container hub-wrap hub-wrap-wide" {
            p class="hub-kicker" { "Experiments" }
            h1 class="hub-title" { "Experiments" }
            p class="hub-standfirst" {
                "Every experiment on the AIEN research ladder, with its scope, its receipt and its outcome, failures included. Everything below is rendered from one data file, so this page cannot drift from the record."
            }
            section class="hub-data" aria-labelledby="experiments-ladder" {
                h2 id="experiments-ladder" class="hub-h2" { "Where the research stands" }
                (rs::render_research_status_summary())
            }
            section id="experiment-cards" class="hub-data" data-source="research_status.json" aria-labelledby="experiment-cards-title" {
                h2 id="experiment-cards-title" class="hub-h2" { "Experiment cards" }
                p class="hub-lede" {
                    "One card per result. The badge is the verdict; the lines below say what it covers, what it does not, and what it waits on."
                }
                (rs::render_cards(d))
                p class="hub-data-foot" {
                    a class="hub-button" href=(STATUS_PAGE) { "Open the full research status record" }
                }
            }
            section class="hub-section" aria-labelledby="experiments-more" {
                h2 id="experiments-more" class="hub-h2" { "Check it yourself" }
                p {
                    "Each experiment is frozen before it is scored, and its record stays public whatever the outcome. The same record is published as "
                    a href=(STATUS_JSON) { "machine-readable data" }
                    ", the protocols sit with the "
                    a href=(PAPER) { "Turing paper" }
                    ", and the "
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
                    li class="hub-card" {
                        a href=(STATUS_PAGE) { "Research status record" }
                        p { "Every experiment with its verdict, receipt, scope and limits, plus the research ladder. Rendered from one data file." }
                    }
                    li class="hub-card" {
                        a href=(STATUS_JSON) { "Research status data (JSON)" }
                        p { "The same record in machine-readable form: the file the status pages are built from." }
                    }
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

/// Fails the build if a hub page is missing, lacks its required markup,
/// lacks any of the six nav sections, contains an em or en dash, shows a
/// status word on a hub that must carry none, or (for the data hub) misses
/// a card for any row of the canonical status source.
pub fn verify_hub_pages(dist: &Path) {
    for hub in &HUBS {
        let path = dist.join(hub.slug).join("index.html");
        let page = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Missing hub page {}", path.display()));
        check_hub_document(hub, &page).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

fn check_hub_document(hub: &Hub, page: &str) -> Result<(), String> {
    for required in hub.required {
        if !page.contains(required) {
            return Err(format!("missing required markup {required:?}"));
        }
    }
    if page.contains('\u{2014}') || page.contains('\u{2013}') {
        return Err("forbidden em or en dash".to_string());
    }
    for (label, href) in crate::components::navbar::NAV_PRIMARY {
        if !page.contains(&crate::components::navbar::primary_link_markup(label, href)) {
            return Err(format!("nav item {label} missing"));
        }
    }
    match hub.status {
        StatusSource::Absent => {
            for status in rs::ALLOWED_STATUSES {
                if page.contains(status) {
                    return Err(format!("status value {status} on a hub that must carry none"));
                }
            }
        }
        StatusSource::Data => {
            for row in &rs::data().turing_rows {
                let card = format!("data-row=\"{}\" data-status=\"{}\"", row.id, row.status);
                if !page.contains(&card) {
                    return Err(format!("card for {} missing or not from data", row.id));
                }
            }
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

    /// The part of a source file before its test module.
    fn non_test_source(src: &str) -> &str {
        src.split("#[cfg(test)]").next().unwrap_or(src)
    }

    #[test]
    fn three_hubs_are_registered() {
        let slugs: Vec<&str> = HUBS.iter().map(|h| h.slug).collect();
        assert_eq!(slugs, ["experiments", "evidence", "philosophy"]);
    }

    #[test]
    fn experiments_hub_renders_the_cards_from_data() {
        let page = doc("experiments");
        assert!(page.contains(EXPERIMENTS_SLOT), "experiment-cards section missing");
        assert!(page.contains(CARDS_MARKUP), "render_cards markup missing");
        assert!(page.contains("data-row=\"EXP-001\""), "EXP-001 card missing");
        let d = rs::data();
        let cards = rs::render_cards(d).into_string();
        assert!(page.contains(&cards), "cards differ from research_status::render_cards");
        for row in &d.turing_rows {
            let card = format!("data-row=\"{}\" data-status=\"{}\"", row.id, row.status);
            assert!(page.contains(&card), "card for {} missing", row.id);
        }
    }

    #[test]
    fn experiments_hub_embeds_the_ladder_and_links_the_full_record() {
        let page = doc("experiments");
        assert!(page.contains(&rs::render_research_status_summary().into_string()), "ladder summary missing");
        assert!(page.contains(STATUS_BUTTON), "full record button missing");
        assert!(page.contains(STATUS_JSON_LINK), "data file link missing");
        let ladder = page.find("experiments-ladder").expect("ladder section");
        let cards = page.find(EXPERIMENTS_SLOT).expect("cards section");
        assert!(ladder < cards, "ladder comes before the cards");
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
    fn evidence_hub_links_the_status_record_and_its_data() {
        let page = doc("evidence");
        assert!(page.contains(STATUS_PAGE_LINK), "/research/status/ link missing");
        assert!(page.contains(STATUS_JSON_LINK), "/research-status.json link missing");
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
    fn nav_and_hub_source_has_no_hand_written_status() {
        let sources = [
            ("hubs.rs", include_str!("hubs.rs")),
            ("navbar.rs", include_str!("navbar.rs")),
            ("footer.rs", include_str!("footer.rs")),
        ];
        for (name, src) in sources {
            let body = non_test_source(src);
            for status in rs::ALLOWED_STATUSES {
                assert!(!body.contains(status), "{name} hand-writes the status word {status}");
            }
        }
    }

    #[test]
    fn build_check_rejects_bad_pages() {
        let evidence = &HUBS[1];
        let page = render_hub_document(evidence);
        assert!(check_hub_document(evidence, &page.replace("Failure is evidence.", "")).is_err());
        assert!(check_hub_document(evidence, &format!("{page}\u{2014}")).is_err());
        let status = rs::ALLOWED_STATUSES[0];
        assert!(check_hub_document(evidence, &format!("{page}{status}")).is_err());
        let experiments = &HUBS[0];
        let page = render_hub_document(experiments);
        assert!(check_hub_document(experiments, &page.replace("data-row=\"EXP-001\"", "data-row=\"X\"")).is_err());
    }
}
