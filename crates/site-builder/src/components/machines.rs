//! The machines and the speeds: what the project runs on today and how
//! fast its own engine goes, with a receipt beside every number.
//!
//! One data file, `data/machines.json`, compiled in with `include_str!`
//! and validated at build time and in the tests. Every machine and every
//! speed row must carry a public link; a row without one fails the build.
//! Numbers stand beside the physical ceiling of the machine they ran on
//! and beside my own previous run. A comparison with another project is
//! allowed when it is honest, dated, and backed by a receipt or that
//! project's own published numbers. Overclaims and unsourced comparisons
//! are refused.
//! The homepage shows both sections; /machine/ shows the machines first,
//! before the theory, so the page describes real hardware.

use std::sync::OnceLock;

use maud::{html, Markup};
use serde::Deserialize;

use super::boxlink::{foot, Target};

/// The raw data file, compiled into the binary.
pub const RAW: &str = include_str!("../../data/machines.json");

/// Comparison claims. A field or sentence may use one only when that same
/// text carries an ISO date and a receipt, an https link, or the other
/// project's own published number.
pub const COMPARISON_CLAIMS: [&str; 6] = [
    "faster than", "x faster", "beats ", "beat the", "level with", "ahead of",
];

/// Overclaims. These stay banned even when a date and a link sit beside them.
/// Phrases are lowercase; callers compare a lowercased string.
pub const OVERCLAIMS: [&str; 5] = [
    "replacement for cuda",
    "everything cuda gives you",
    "omega's replacement and its status",
    "beating cuda",
    "matching and then",
];

fn has_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 10 {
        return false;
    }
    for i in 0..=b.len() - 10 {
        let w = &b[i..i + 10];
        if w[4] == b'-'
            && w[7] == b'-'
            && w[..4].iter().all(u8::is_ascii_digit)
            && w[5..7].iter().all(u8::is_ascii_digit)
            && w[8..10].iter().all(u8::is_ascii_digit)
        {
            return true;
        }
    }
    false
}

fn sourced_comparison(lower: &str) -> bool {
    has_iso_date(lower)
        && (lower.contains("https://") || lower.contains("receipt") || lower.contains("published"))
}

/// `None` when the text is not a comparison, or is an honest dated sourced
/// comparison. Overclaims and unsourced comparison claims return a reason.
pub fn comparison_problem(s: &str) -> Option<&'static str> {
    let lower = s.to_ascii_lowercase();
    for w in OVERCLAIMS {
        if lower.contains(w) {
            return Some("overclaim");
        }
    }
    if COMPARISON_CLAIMS.iter().any(|w| lower.contains(w)) && !sourced_comparison(&lower) {
        return Some("unsourced comparison");
    }
    None
}

/// Sentence-sized check for a rendered page, so a date elsewhere in the
/// document does not license a comparison in another sentence.
pub fn comparison_errors(text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    for sentence in text.split(|c| matches!(c, '\n' | '.' | '!' | '?')) {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }
        if let Some(kind) = comparison_problem(sentence) {
            errors.push(format!("{kind} in {sentence:?}"));
        }
    }
    errors
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub as_of: String,
    pub source: String,
    pub judging: String,
    pub ceilings: Vec<Ceiling>,
    pub machines: Vec<Machine>,
    pub speeds: Vec<Speed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ceiling {
    pub id: String,
    pub model: String,
    pub machine: String,
    pub tok_s: f64,
    pub how: String,
    pub link: String,
    pub receipt: String,
    pub branch: String,
    pub on_main: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub link: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub role: String,
    pub today: String,
    pub evidence_date: String,
    pub evidence: String,
    pub link: String,
    pub branch: String,
    pub on_main: bool,
    pub more: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speed {
    pub id: String,
    pub what: String,
    pub model: String,
    pub machine: String,
    pub engine: String,
    pub value: String,
    pub detail: String,
    pub tok_s: Option<f64>,
    pub ceiling: String,
    pub previous: String,
    pub date: String,
    pub link: String,
    pub receipt: String,
    pub branch: String,
    pub on_main: bool,
    pub verdict: String,
    pub note: String,
}

impl Data {
    pub fn ceiling(&self, id: &str) -> Option<&Ceiling> {
        if id.is_empty() {
            return None;
        }
        self.ceilings.iter().find(|c| c.id == id)
    }
}

fn has_dash(s: &str) -> bool {
    s.contains('\u{2014}') || s.contains('\u{2013}')
}

fn check_text(what: &str, s: &str, errors: &mut Vec<String>) {
    if has_dash(s) {
        errors.push(format!("{what}: em or en dash in {s:?}"));
    }
    if let Some(kind) = comparison_problem(s) {
        errors.push(format!("{what}: {kind} in {s:?}"));
    }
}

fn check_link(what: &str, link: &str, errors: &mut Vec<String>) {
    if !link.starts_with("https://") {
        errors.push(format!("{what}: link must be a public https URL, got {link:?}"));
    }
}

/// Parse and validate. Every error is listed, not just the first.
pub fn parse(raw: &str) -> Result<Data, String> {
    let d: Data = serde_json::from_str(raw).map_err(|e| format!("machines.json: {e}"))?;
    let mut errors = Vec::new();
    check_text("as_of", &d.as_of, &mut errors);
    check_text("source", &d.source, &mut errors);
    check_text("judging", &d.judging, &mut errors);
    if d.machines.is_empty() {
        errors.push("no machines".into());
    }
    if d.speeds.is_empty() {
        errors.push("no speeds".into());
    }
    for c in &d.ceilings {
        let what = format!("ceiling {}", c.id);
        check_link(&what, &c.link, &mut errors);
        for s in [&c.model, &c.machine, &c.how, &c.receipt, &c.branch] {
            check_text(&what, s, &mut errors);
        }
        if c.tok_s.is_nan() || c.tok_s <= 0.0 {
            errors.push(format!("{what}: ceiling must be positive"));
        }
    }
    for m in &d.machines {
        let what = format!("machine {}", m.id);
        check_link(&what, &m.link, &mut errors);
        for s in [&m.name, &m.detail, &m.role, &m.today, &m.evidence_date, &m.evidence, &m.branch] {
            check_text(&what, s, &mut errors);
        }
        if m.evidence_date.len() != 10 {
            errors.push(format!("{what}: evidence_date must be YYYY-MM-DD"));
        }
        for l in &m.more {
            check_link(&what, &l.link, &mut errors);
            check_text(&what, &l.label, &mut errors);
        }
    }
    for s in &d.speeds {
        let what = format!("speed {}", s.id);
        check_link(&what, &s.link, &mut errors);
        for t in [
            &s.what, &s.model, &s.machine, &s.engine, &s.value, &s.detail, &s.previous, &s.date,
            &s.receipt, &s.branch, &s.verdict, &s.note,
        ] {
            check_text(&what, t, &mut errors);
        }
        if !s.ceiling.is_empty() && d.ceiling(&s.ceiling).is_none() {
            errors.push(format!("{what}: unknown ceiling {:?}", s.ceiling));
        }
        if !s.ceiling.is_empty() && s.tok_s.is_none() {
            errors.push(format!("{what}: a ceiling needs a numeric tok_s"));
        }
        if let (Some(t), Some(c)) = (s.tok_s, d.ceiling(&s.ceiling)) {
            if t > c.tok_s {
                errors.push(format!("{what}: {t} tok/s is above its ceiling {}", c.tok_s));
            }
        }
        if s.on_main && s.branch != "main" {
            errors.push(format!("{what}: on_main is true but branch is {:?}", s.branch));
        }
    }
    let ids: Vec<&str> = d.speeds.iter().map(|s| s.id.as_str()).collect();
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            errors.push(format!("speed {id}: duplicate id"));
        }
    }
    if errors.is_empty() {
        Ok(d)
    } else {
        Err(errors.join("\n"))
    }
}

static DATA: OnceLock<Data> = OnceLock::new();

/// The validated data. A bad file stops the build.
pub fn data() -> &'static Data {
    DATA.get_or_init(|| parse(RAW).unwrap_or_else(|e| panic!("{e}")))
}

/// "0.752" for 7.63 of 10.15.
pub fn fraction(tok_s: f64, ceiling: f64) -> String {
    format!("{:.3}", tok_s / ceiling)
}

/// "22.0 to 22.5 tok/s" as a big number and a small unit (the last word);
/// a value with no space stays whole.
fn big_value(value: &str) -> Markup {
    let (n, unit) = value.rsplit_once(' ').unwrap_or((value, ""));
    html! {
        span class="mx-hero-value mono" { (n) @if !unit.is_empty() { " " span class="mx-unit" { (unit) } } }
    }
}

fn where_badge(on_main: bool, branch: &str) -> Markup {
    html! {
        @if on_main {
            span class="mx-where mx-where-main" { "on main" }
        } @else {
            span class="mx-where mx-where-branch" title=(format!("branch {branch}")) { "branch " (branch) }
        }
    }
}

// ---------------------------------------------------------------- machines

fn machine_card(m: &Machine) -> Markup {
    html! {
        article class="mx-card" id=(format!("machine-{}", m.id)) {
            header class="mx-card-head" {
                h3 class="mx-name" { (m.name) }
                p class="mx-detail" { (m.detail) }
            }
            p class="mx-role" { (m.role) }
            dl class="mx-facts" {
                dt { "Today" }
                dd { (m.today) }
                dt { "Last evidence" }
                dd {
                    span class="mx-date mono" { (m.evidence_date) }
                    " " (m.evidence)
                }
            }
            footer class="mx-card-foot" {
                (where_badge(m.on_main, &m.branch))
                @if !m.more.is_empty() {
                    span class="mx-more" {
                        @for (i, l) in m.more.iter().enumerate() {
                            @if i > 0 { " · " }
                            a href=(l.link) target="_blank" rel="noopener noreferrer" { (l.label) }
                        }
                    }
                }
            }
            (foot(&m.link, Target::Evidence))
        }
    }
}

/// The grid of machine cards, shared by the homepage and /machine/.
pub fn render_machine_cards(d: &Data) -> Markup {
    html! {
        div class="mx-grid" {
            @for m in &d.machines { (machine_card(m)) }
        }
    }
}

/// Homepage section: one card per machine.
pub fn render_machines_section() -> Markup {
    let d = data();
    html! {
        section id="machines" class="mx-section" aria-labelledby="machines-title" {
            div class="container" {
                div class="mx-head" {
                    p class="kicker" { "The machines" }
                    h2 id="machines-title" class="mx-h2" { "Five machines, one project" }
                    p class="mx-lead" {
                        "Aien is built and judged on a DGX Spark, runs bare metal on an Intel iMac and a Raspberry Pi 5, passes every kernel gate in QEMU first, and is being built for a Windows PC with an RTX 5070. Each card says what runs on that machine today, when the last evidence landed, and where the receipt is."
                    }
                }
                (render_machine_cards(d))
                p class="mx-stamp" {
                    "Inventory as of " span class="mono" { (d.as_of) } ". A machine without a public receipt is not on this list."
                }
            }
        }
    }
}

/// The compact version for the top of /machine/: the same cards under a
/// shorter introduction, so the page names real hardware before the theory.
pub fn render_real_machines() -> Markup {
    let d = data();
    html! {
        section id="real-machines" class="narr-section mx-on-narr" aria-labelledby="real-machines-heading" {
            h2 id="real-machines-heading" { "The bodies of one machine" }
            p {
                "One machine, five physical bodies. Aien is one system whose work moves between them: a DGX Spark for development and judgment, an Intel iMac and a Raspberry Pi 5 that run the kernel bare metal, QEMU for every gate before hardware, and a Windows PC with an RTX 5070 as the next target. Each card says what runs on that body today, the last evidence for it, and where the receipt is. The loop below is what they are for."
            }
            (render_machine_cards(d))
            p class="narr-more" {
                a href="/#speed" { "The speeds measured on them, with receipts, are on the homepage." }
            }
        }
    }
}

// ---------------------------------------------------------------- speeds

fn speed_row(d: &Data, s: &Speed) -> Markup {
    let ceiling = d.ceiling(&s.ceiling);
    html! {
        tr class="mx-row" id=(format!("speed-{}", s.id)) {
            td class="mx-td-what" data-label="What" {
                span class="mx-what" { (s.what) }
                span class="mx-model" { (s.model) }
            }
            td class="mx-td-machine" data-label="Machine and engine" {
                (s.machine)
                span class="mx-sub" { (s.engine) }
            }
            td class="mx-td-value" data-label="Measured" {
                span class="mx-value mono" { (s.value) }
                @if !s.detail.is_empty() { span class="mx-sub" { (s.detail) } }
                @if !s.verdict.is_empty() { span class="mx-verdict" { (s.verdict) } }
            }
            td class="mx-td-ceiling" data-label="Ceiling" {
                @if let (Some(t), Some(c)) = (s.tok_s, ceiling) {
                    span class="mono" { (fraction(t, c.tok_s)) }
                    span class="mx-sub" {
                        "of "
                        a href=(c.link) target="_blank" rel="noopener noreferrer" { (format!("{:.2} tok/s", c.tok_s)) }
                    }
                } @else {
                    span class="mx-sub mx-none" { "none shown" }
                }
            }
            td class="mx-td-prev" data-label="My previous run" {
                @if s.previous.is_empty() {
                    span class="mx-sub mx-none" { "first of its kind" }
                } @else {
                    (s.previous)
                }
            }
            td class="mx-td-date mono" data-label="Date" { (s.date) }
            td class="mx-td-receipt" data-label="Receipt" {
                span class="mx-receipt" { (s.receipt) }
                (where_badge(s.on_main, &s.branch))
                @if !s.note.is_empty() { span class="mx-sub" { (s.note) } }
                (foot(&s.link, Target::Evidence))
            }
        }
    }
}

/// Homepage section: the speed table, the 35B end to end first.
pub fn render_speeds_section() -> Markup {
    let d = data();
    let first = d.speeds.first().expect("at least one speed row");
    let first_ceiling = d.ceiling(&first.ceiling);
    html! {
        section id="speed" class="mx-section mx-speed" aria-labelledby="speed-title" {
            div class="container" {
                div class="mx-head" {
                    p class="kicker" { "Speed, with receipts" }
                    h2 id="speed-title" class="mx-h2" { "The 35B end to end, on a test server" }
                    p class="mx-lead" {
                        "Qwen3.6-35B-A3B, a mixture-of-experts with about 3B parameters active per token, answers through my own C GPU stack, on the Spark. The number below is end-to-end wall time from a test server running a branch build, prompt reading included. Chip time for the same model is decode only. The gap between chip time and this end-to-end time comes from prompt reading. The full table holds the 27B chip-time rows, the engine alone, the smaller Rig engine and the long-context cost, each labelled end to end or chip time, and every row links to the folder its number came from."
                    }
                }
                div class="mx-hero" {
                    div class="mx-hero-n" {
                        (big_value(&first.value))
                        span class="mx-hero-l" { (first.model) " · " (first.engine) " · " (first.machine) }
                    }
                    @if let (Some(t), Some(c)) = (first.tok_s, first_ceiling) {
                        div class="mx-hero-c" {
                            span class="mx-hero-value mono mx-hero-frac" { (fraction(t, c.tok_s)) }
                            span class="mx-hero-l" {
                                "of the " a href=(c.link) target="_blank" rel="noopener noreferrer" { (format!("{:.2} tok/s", c.tok_s)) " memory ceiling" }
                            }
                        }
                    }
                    @if !first.previous.is_empty() {
                        div class="mx-hero-p mx-hero-prev" {
                            (big_value(first.previous.split(',').next().unwrap_or("")))
                            span class="mx-hero-l" { "my previous run" }
                        }
                    }
                    a class="mx-hero-r" href=(first.link) target="_blank" rel="noopener noreferrer" {
                        "Receipt: " (first.receipt) " " (where_badge(first.on_main, &first.branch))
                    }
                }
                // One number leads; every other row stays one click away in a
                // native disclosure, so the section reads as one idea.
                details class="mx-receipts" {
                    summary class="mx-receipts-summary" { "All " (d.speeds.len()) " speed receipts, ceilings and how they are judged" }
                    div class="mx-table-wrap" {
                        table class="mx-table" {
                            thead {
                                tr {
                                    th { "What" }
                                    th { "Machine and engine" }
                                    th { "Measured" }
                                    th { "Ceiling" }
                                    th { "My previous run" }
                                    th { "Date" }
                                    th { "Receipt" }
                                }
                            }
                            tbody {
                                @for s in &d.speeds { (speed_row(d, s)) }
                            }
                        }
                    }
                    p class="mx-judging" { (d.judging) }
                    @for c in &d.ceilings {
                        p class="mx-stamp" {
                            "Ceiling for " (c.model) " on the " (c.machine) ": " span class="mono" { (format!("{:.2} tok/s", c.tok_s)) } ". " (c.how) " "
                            a href=(c.link) target="_blank" rel="noopener noreferrer" { (c.receipt) }
                            " " (where_badge(c.on_main, &c.branch))
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

    /// A speed row whose receipt is chip time: a chip folder, milliseconds per
    /// token, or GPU time per token. End-to-end wall time and cold time to
    /// first token are not chip time.
    fn is_chip_time(s: &Speed) -> bool {
        let blob = format!("{} {} {}", s.receipt, s.detail, s.value).to_ascii_lowercase();
        blob.contains("chip") || blob.contains("ms per token") || blob.contains("gpu time")
    }

    #[test]
    fn real_data_parses_and_validates() {
        let d = parse(RAW).expect("machines.json must validate");
        assert_eq!(d.machines.len(), 5, "five machines: Spark, iMac, Pi 5, QEMU, RTX 5070 PC");
        for want in ["Spark", "iMac", "Raspberry Pi 5", "QEMU", "RTX 5070"] {
            assert!(d.machines.iter().any(|m| m.name.contains(want)), "machine {want} missing");
        }
        assert!(d.speeds.first().unwrap().model.starts_with("Qwen3.6-35B-A3B"), "the 35B end to end comes first");
        let spark = d.machines.iter().find(|m| m.id == "spark").unwrap();
        assert!(spark.today.contains("qwen3.6-35b-a3b-fp8"), "the Spark today line names the live model: {}", spark.today);
        assert!(!spark.today.contains("Qwen3.8-27B"), "the Spark today line must not say it serves the 27B");
        assert!(d.speeds.first().unwrap().what.starts_with("End to end"));
        assert!(d.speeds.first().unwrap().what.contains("test server") && d.speeds.first().unwrap().what.contains("branch build"));
        assert!(d.speeds.first().unwrap().engine.contains("Omega"));
        assert!(d.judging.contains("prompt reading"));
        assert!(!d.judging.to_ascii_lowercase().contains("without the server"));
        for s in d.speeds.iter().filter(|s| is_chip_time(s)) {
            assert!(s.what.to_ascii_lowercase().contains("chip time"), "{}: chip-time row must say so ({})", s.id, s.what);
        }
    }

    #[test]
    fn every_row_has_a_public_link() {
        let d = data();
        for m in &d.machines {
            assert!(m.link.starts_with("https://github.com/aien-dev/"), "{}: {}", m.id, m.link);
            for l in &m.more {
                assert!(l.link.starts_with("https://github.com/aien-dev/"), "{}: {}", m.id, l.link);
            }
        }
        for s in &d.speeds {
            assert!(s.link.starts_with("https://github.com/aien-dev/"), "{}: {}", s.id, s.link);
        }
        for c in &d.ceilings {
            assert!(c.link.starts_with("https://github.com/aien-dev/"), "{}: {}", c.id, c.link);
        }
    }

    #[test]
    fn fraction_matches_the_inventory() {
        assert_eq!(fraction(7.63, 10.15), "0.752");
        assert_eq!(fraction(7.657, 10.15), "0.754");
    }

    #[test]
    fn rejects_missing_link_dashes_and_comparisons() {
        let no_link = RAW.replacen(
            "\"link\": \"https://github.com/aien-dev/aien-sovereign-core-mv/tree/qwen38/serve-fast/evidence/qwen38-serve-fast-20261019\"",
            "\"link\": \"\"",
            1,
        );
        assert!(parse(&no_link).unwrap_err().contains("public https URL"));
        let dashed = RAW.replacen("\"what\": \"Live server", "\"what\": \"Live \u{2014} server", 1);
        assert!(parse(&dashed).unwrap_err().contains("em or en dash"));
        let compared = RAW.replacen("\"note\": \"Nothing has run on the 5070 yet.\"", "\"note\": \"2x faster than llama.cpp\"", 1);
        let err = parse(&compared).unwrap_err();
        assert!(err.contains("unsourced comparison"), "{err}");
        let honest = RAW.replacen(
            "\"note\": \"Nothing has run on the 5070 yet.\"",
            "\"note\": \"On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (https://example.com/bench).\"",
            1,
        );
        assert!(parse(&honest).is_ok(), "an honest dated sourced comparison is allowed");
        let over = RAW.replacen(
            "\"note\": \"Nothing has run on the 5070 yet.\"",
            "\"note\": \"A replacement for CUDA, dated 2026-10-21, receipt https://example.com/r.\"",
            1,
        );
        let err = parse(&over).unwrap_err();
        assert!(err.contains("overclaim"), "{err}");
        let above = RAW.replacen("\"tok_s\": 7.63,", "\"tok_s\": 70.63,", 1);
        assert!(parse(&above).unwrap_err().contains("above its ceiling"));
        let extra = RAW.replacen("\"as_of\"", "\"surprise\": 1, \"as_of\"", 1);
        assert!(parse(&extra).is_err(), "unknown fields must be refused");
    }

    #[test]
    fn sections_render_clean() {
        for page in [
            render_machines_section().into_string(),
            render_speeds_section().into_string(),
            render_real_machines().into_string(),
        ] {
            assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'), "dash on the page");
            let problems = comparison_errors(&page);
            assert!(problems.is_empty(), "comparison problem on the page: {problems:?}");
        }
        let speeds = render_speeds_section().into_string();
        assert!(speeds.contains("7.63 tok/s"));
        assert!(speeds.contains("0.752"));
        assert!(speeds.contains("10.15 tok/s"));
        assert!(speeds.contains("id=\"speed-27b-live\""));
        assert!(speeds.contains("id=\"speed-35b-e2e\""));
        assert!(speeds.contains("22.0 to 22.5"));
        assert!(speeds.contains("test server running a branch build"));
        assert!(speeds.contains("comes from prompt reading"));
        assert!(!speeds.to_ascii_lowercase().contains("without the server"));
        assert!(!speeds.to_ascii_lowercase().contains("leaves out the server"));
        assert!(speeds.contains(&data().judging));
        let machines = render_machines_section().into_string();
        for id in ["machine-spark", "machine-imac", "machine-pi5", "machine-qemu", "machine-rig"] {
            assert!(machines.contains(&format!("id=\"{id}\"")), "{id} missing");
        }
        assert_eq!(machines.matches("class=\"box-link\"").count(), 5, "one evidence link per machine");
        assert_eq!(crate::components::boxlink::check_boxes(&machines), Ok(5), "every machine card links to GitHub");
        let rows = data().speeds.len();
        assert_eq!(crate::components::boxlink::check_boxes(&speeds), Ok(rows), "every speed row links to GitHub");
    }
}
