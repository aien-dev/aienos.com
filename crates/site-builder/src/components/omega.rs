//! /omega/: what Omega is, in the same words as the omega repository's
//! `docs/OMEGA.md`.
//!
//! Omega is AIEN's language for all computing, GPU to CPU; on the GPU it is
//! the replacement for CUDA. The page renders, from `data/omega.json`:
//! the parts of Omega, the CUDA stack piece by piece with Omega's
//! replacement and its status, the one thing that is true today with its
//! receipt, what is not true yet, and the bar C1 to C6 as a checklist.
//! Every row links to its source or receipt on GitHub. A row whose status
//! is MISSING is drawn like any other row; nothing is filtered out.
//!
//! The data file is checked at build time: unknown status words, a bar row
//! marked PASS without a receipt, a box that does not link to GitHub, a
//! link in a form the expander does not know, or an em or en dash all fail
//! the build and the tests.

use std::sync::OnceLock;

use maud::{html, Markup};
use serde::Deserialize;

use super::boxlink::{foot_labelled, Target};

pub const TITLE: &str = "What Omega is | AienOS";
pub const DESCRIPTION: &str = "Omega is Aien's language for all computing, GPU to CPU. On the GPU it is the replacement for CUDA. The CUDA stack piece by piece, the receipt for what is true today, what is not true yet, and the bar C1 to C6.";
pub const URL: &str = "https://www.aienos.com/omega/";

/// The data file, compiled in and published as /omega.json.
pub const RAW: &str = include_str!("../../data/omega.json");

/// Omega on GitHub. The repository aien-dev/omega was renamed to
/// aien-dev/omega-mv; GitHub still redirects the old name, but the links
/// name the repository as it is now.
pub const OMEGA_BLOB: &str = "https://github.com/aien-dev/omega-mv/blob/main/";
pub const OMEGA_TREE: &str = "https://github.com/aien-dev/omega-mv/tree/main/";

/// Status words a CUDA row may carry, in the order the legend shows them.
pub const STATUSES: [&str; 4] = ["PASS", "PARTIAL", "MISSING", "NOT REPLACEABLE"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmegaData {
    pub source: Source,
    pub claim: String,
    pub claim_rest: Vec<String>,
    pub true_today: String,
    pub not_true_yet: String,
    pub parts: Vec<Card>,
    pub below: Card,
    pub layers: Vec<Layer>,
    pub receipt: Receipt,
    pub not_yet: Vec<Gap>,
    pub bar: Vec<Bar>,
    pub beyond: Vec<Gap>,
    pub performance: Performance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Performance {
    pub stand: String,
    pub how: String,
    #[serde(rename = "box")]
    pub box_: String,
    pub rule: String,
    pub bar: Vec<Bar>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub doc: String,
    pub branch: String,
    pub commit: String,
    pub checked: String,
    pub ledger: String,
    pub issue: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub name: String,
    pub what: String,
    pub links: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub cuda: String,
    pub job: String,
    pub omega: String,
    pub status: String,
    pub note: String,
    pub links: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub title: String,
    pub taken: String,
    pub what: String,
    pub facts: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gap {
    pub title: String,
    pub text: String,
    pub links: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    pub id: String,
    pub title: String,
    pub done_when: String,
    pub status: String,
    pub now: String,
    pub receipt: Option<String>,
    pub links: Vec<(String, String)>,
}

/// Expands a short link from the data file into a full address.
/// `o:` a file in omega, `o/:` a folder in omega, `ob:` an omega branch,
/// `p:` physics, `c:` aien-sovereign-core, `a:` aien-architecture, `b:` omega-bridge,
/// `issue:` an omega issue. A link starting with `/` or `#` stays on this site.
pub fn expand(link: &str) -> Result<String, String> {
    const MAP: [(&str, &str); 8] = [
        ("o/:", OMEGA_TREE),
        ("ob:", "https://github.com/aien-dev/omega-mv/tree/"),
        ("o:", OMEGA_BLOB),
        ("p:", "https://github.com/aien-dev/physics-mv/blob/main/"),
        ("c:", "https://github.com/aien-dev/aien-sovereign-core-mv/blob/main/"),
        ("a:", "https://github.com/aien-dev/aien-architecture-mv/blob/main/"),
        ("b:", "https://github.com/aien-dev/omega-bridge/blob/main/"),
        ("issue:", "https://github.com/aien-dev/omega-mv/issues/"),
    ];
    if link.starts_with('/') || link.starts_with('#') {
        return Ok(link.to_string());
    }
    for (prefix, base) in MAP {
        if let Some(rest) = link.strip_prefix(prefix) {
            if rest.is_empty() || rest.contains("..") || rest.contains(' ') {
                return Err(format!("bad link {link:?}"));
            }
            return Ok(format!("{base}{rest}"));
        }
    }
    Err(format!("unknown link form {link:?}"))
}

fn gh(link: &str) -> String {
    let h = expand(link).unwrap_or_else(|e| panic!("data/omega.json: {e}"));
    assert!(h.starts_with("https://github.com/"), "data/omega.json: box link must be on GitHub: {link}");
    h
}

pub fn parse(raw: &str) -> Result<OmegaData, String> {
    serde_json::from_str(raw).map_err(|e| format!("data/omega.json does not parse: {e}"))
}

pub fn validate(d: &OmegaData) -> Result<(), String> {
    if raw_has_dash(RAW) {
        return Err("data/omega.json has an em or en dash".into());
    }
    let mut boxes: Vec<&str> = vec![&d.source.doc, &d.source.issue, &d.below.box_, &d.receipt.box_];
    let mut links: Vec<&str> = Vec::new();
    for c in &d.parts {
        boxes.push(&c.box_);
        links.extend(c.links.iter().map(|l| l.1.as_str()));
    }
    links.extend(d.below.links.iter().map(|l| l.1.as_str()));
    if d.layers.len() < 13 {
        return Err(format!("the CUDA map has {} rows; docs/OMEGA.md has 13", d.layers.len()));
    }
    for l in &d.layers {
        if !STATUSES.contains(&l.status.as_str()) {
            return Err(format!("unknown status {:?} on {}", l.status, l.cuda));
        }
        if l.cuda.is_empty() || l.omega.is_empty() || l.job.is_empty() {
            return Err(format!("empty cell in the CUDA map row {:?}", l.cuda));
        }
        boxes.push(&l.box_);
        links.extend(l.links.iter().map(|x| x.1.as_str()));
    }
    for g in d.not_yet.iter().chain(d.beyond.iter()) {
        boxes.push(&g.box_);
        links.extend(g.links.iter().map(|x| x.1.as_str()));
    }
    let ids: Vec<&str> = d.bar.iter().map(|b| b.id.as_str()).collect();
    if ids != ["C1", "C2", "C3", "C4", "C5", "C6"] {
        return Err(format!("the bar must be C1 to C6 in order, found {ids:?}"));
    }
    let pids: Vec<&str> = d.performance.bar.iter().map(|b| b.id.as_str()).collect();
    if pids != ["P0", "P1", "P2", "P3", "P4"] {
        return Err(format!("the performance bar must be P0 to P4 in order, found {pids:?}"));
    }
    boxes.push(&d.performance.box_);
    for b in d.bar.iter().chain(d.performance.bar.iter()) {
        if !STATUSES[..3].contains(&b.status.as_str()) {
            return Err(format!("{}: unknown status {:?}", b.id, b.status));
        }
        match (&b.status[..], &b.receipt) {
            ("PASS", None) => return Err(format!("{} is PASS without a receipt", b.id)),
            ("PASS", Some(r)) if !r.starts_with("o:evidence/") && !r.starts_with("o/:evidence/") => {
                return Err(format!("{} receipt must be under omega evidence/: {r}", b.id))
            }
            (s, Some(_)) if s != "PASS" => return Err(format!("{} has a receipt but is {s}", b.id)),
            _ => {}
        }
        if let Some(r) = &b.receipt {
            boxes.push(r);
        }
        links.extend(b.links.iter().map(|x| x.1.as_str()));
    }
    for b in boxes {
        let h = expand(b)?;
        if !h.starts_with("https://github.com/") {
            return Err(format!("box link off GitHub: {b}"));
        }
    }
    for l in links {
        expand(l)?;
    }
    if !d.source.doc.starts_with("o:docs/OMEGA.md") {
        return Err("source.doc must be omega docs/OMEGA.md".into());
    }
    Ok(())
}

fn raw_has_dash(s: &str) -> bool {
    s.contains('\u{2014}') || s.contains('\u{2013}')
}

pub fn data() -> &'static OmegaData {
    static DATA: OnceLock<OmegaData> = OnceLock::new();
    DATA.get_or_init(|| {
        let d = parse(RAW).unwrap_or_else(|e| panic!("{e}"));
        validate(&d).unwrap_or_else(|e| panic!("data/omega.json: {e}"));
        d
    })
}

fn status_class(s: &str) -> &'static str {
    match s {
        "PASS" => "om-pass",
        "PARTIAL" => "om-partial",
        "MISSING" => "om-missing",
        _ => "om-fixed",
    }
}

fn status_label(s: &str) -> &'static str {
    match s {
        "PASS" => "PASS",
        "PARTIAL" => "PARTIAL",
        "MISSING" => "MISSING",
        _ => "NOT REPLACEABLE",
    }
}

/// Inline links inside a box; they sit above the box's stretched link.
fn inline_links(links: &[(String, String)]) -> Markup {
    html! {
        @if !links.is_empty() {
            ul class="om-links" {
                @for (label, link) in links {
                    @let h = expand(link).unwrap_or_else(|e| panic!("{e}"));
                    li {
                        @if h.starts_with("https://") {
                            a href=(h) target="_blank" rel="noopener noreferrer" { (label) }
                        } @else {
                            a href=(h) { (label) }
                        }
                    }
                }
            }
        }
    }
}

/// The label of a box link: a receipt in evidence/ is evidence, anything
/// else is the source file that holds the text.
fn box_foot(link: &str) -> Markup {
    let h = gh(link);
    let evidence = link.contains("evidence/");
    if evidence {
        foot_labelled(&h, "Receipt on GitHub", Target::Evidence)
    } else {
        foot_labelled(&h, "Source on GitHub", Target::Source)
    }
}

/// Counts of each status word in the CUDA map, in legend order.
pub fn layer_counts(d: &OmegaData) -> Vec<(&'static str, usize)> {
    STATUSES.iter().map(|s| (*s, d.layers.iter().filter(|l| l.status == *s).count())).collect()
}

pub fn bar_met(d: &OmegaData) -> usize {
    d.bar.iter().filter(|b| b.status == "PASS").count()
}

pub fn render_omega_page() -> Markup {
    let d = data();
    let doc = gh(&d.source.doc);
    let counts = layer_counts(d);
    let met = bar_met(d);
    let pmet = d.performance.bar.iter().filter(|b| b.status == "PASS").count();
    html! {
        div class="om-page" {
            header class="om-hero" {
                div class="container" {
                    p class="om-kicker" { "Omega" }
                    h1 class="om-title" { (d.claim) }
                    @for p in &d.claim_rest {
                        p class="om-lead" { (p) }
                    }
                    div class="om-verdicts" {
                        div class="om-verdict om-verdict-true" {
                            p class="om-verdict-k" { "True today, with a receipt" }
                            p class="om-verdict-s" { (d.true_today) }
                            a class="om-verdict-a" href="#today" { "See the receipt" }
                        }
                        div class="om-verdict om-verdict-not" {
                            p class="om-verdict-k" { "Not true yet" }
                            p class="om-verdict-s" { "\u{201C}" (d.not_true_yet) "\u{201D}" }
                            a class="om-verdict-a" href="#bar" { (met) " of " (d.bar.len()) " checks met" }
                        }
                    }
                    p class="om-source" {
                        "Written from "
                        a href=(doc) target="_blank" rel="noopener noreferrer" { "docs/OMEGA.md" }
                        " in the omega repository (branch " span class="mono" { (d.source.branch) } " at "
                        span class="mono" { (d.source.commit) } "), checked " (d.source.checked) ". Status words: PASS (a receipt exists on the named commit), PARTIAL, MISSING. Ledger item "
                        a href=(format!("/now/#{}", d.source.ledger)) { (d.source.ledger) }
                        " tracks the work; tracking issue "
                        a href=(gh(&d.source.issue)) target="_blank" rel="noopener noreferrer" { "omega#593" }
                        ". Data file: "
                        a href="/omega.json" { "/omega.json" }
                        "."
                    }
                    nav class="om-toc" aria-label="On this page" {
                        a href="#parts" { "The parts" }
                        a href="#cuda-map" { "CUDA, piece by piece" }
                        a href="#today" { "True today" }
                        a href="#not-yet" { "Not true yet" }
                        a href="#bar" { "The bar, C1 to C6" }
                        a href="#beyond" { "Beyond CUDA" }
                        a href="#performance" { "Performance" }
                    }
                    p class="om-source" {
                        "CUDA++, the GPU half of Omega, has its own page: "
                        a href="/cudaplusplus/" { "CUDA++" }
                        "."
                    }
                    p class="om-source" {
                        "Writing Omega yourself, or pointing an agent at it: the "
                        a href="/omega/reference/" { "Omega language reference" }
                        " has every compiler error, every trap and every gated worked example."
                    }
                }
            }

            section id="parts" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "1 \u{00B7} The parts" }
                    h2 class="om-h2" { "Four parts, and the driver below them" }
                    div class="om-parts" {
                        @for c in &d.parts {
                            article class="om-part" {
                                h3 class="om-part-name" { (c.name) }
                                p class="om-part-what" { (c.what) }
                                (inline_links(&c.links))
                                (box_foot(&c.box_))
                            }
                        }
                    }
                    article class="om-part om-below" {
                        h3 class="om-part-name" { (d.below.name) }
                        p class="om-part-what" { (d.below.what) }
                        (inline_links(&d.below.links))
                        (box_foot(&d.below.box_))
                    }
                }
            }

            section id="cuda-map" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "2 \u{00B7} CUDA, piece by piece" }
                    h2 class="om-h2" { "Each piece of CUDA and Omega's replacement" }
                    p class="om-sub" {
                        "\u{201C}CUDA\u{201D} is a stack of NVIDIA software. This is each piece, top of the stack to the chip, and Omega's replacement for it. Missing pieces are shown as missing."
                    }
                    div class="om-meter" role="img" aria-label=(meter_label(&counts)) {
                        @for l in &d.layers {
                            span class=(format!("om-seg {}", status_class(&l.status))) title=(format!("{}: {}", l.cuda, status_label(&l.status))) {}
                        }
                    }
                    ul class="om-legend" {
                        @for (s, n) in &counts {
                            li class=(status_class(s)) { span class="om-dot" aria-hidden="true" {} (s) " " span class="mono" { (n) } }
                        }
                    }
                    div class="om-map" role="table" aria-label="CUDA pieces and Omega's replacements" {
                        div class="om-map-head" role="row" {
                            span role="columnheader" { "CUDA piece" }
                            span role="columnheader" { "Its job" }
                            span role="columnheader" { "Omega's replacement" }
                            span role="columnheader" { "Status" }
                        }
                        @for l in &d.layers {
                            div class=(format!("om-layer {}", status_class(&l.status))) role="row" {
                                span class="om-cuda" role="cell" { (l.cuda) }
                                span class="om-job" role="cell" { (l.job) }
                                span class="om-repl" role="cell" {
                                    (l.omega)
                                    (inline_links(&l.links))
                                }
                                span class="om-status" role="cell" {
                                    span class="om-pill" { (status_label(&l.status)) }
                                    @if !l.note.is_empty() { span class="om-note" { (l.note) } }
                                }
                                (box_foot(&l.box_))
                            }
                        }
                    }
                }
            }

            section id="today" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "What is already true, with a receipt" }
                    h2 class="om-h2" { (d.receipt.title) }
                    article class="om-receipt" {
                        p class="om-receipt-what" { (d.receipt.what) }
                        dl class="om-facts" {
                            @for (k, v) in &d.receipt.facts {
                                div class=(if v == "0" { "om-fact om-fact-zero" } else { "om-fact" }) {
                                    dt { (k) }
                                    dd class="mono" { (v) }
                                }
                            }
                        }
                        p class="om-taken" { "Taken " span class="mono" { (d.receipt.taken) } ". The receipt holds the binary sha256, the mapped libraries, ldd, the driver version and the commands used." }
                        p class="om-true" { "So this sentence is true today: " strong { (d.true_today) } }
                        (box_foot(&d.receipt.box_))
                    }
                }
            }

            section id="not-yet" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "What is not true yet" }
                    h2 class="om-h2" { "\u{201C}" (d.not_true_yet) "\u{201D} is not true yet" }
                    div class="om-gaps" {
                        @for g in &d.not_yet {
                            article class="om-gap" {
                                h3 class="om-gap-title" { (g.title) }
                                p { (g.text) }
                                (inline_links(&g.links))
                                (box_foot(&g.box_))
                            }
                        }
                    }
                }
            }

            section id="bar" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "3 \u{00B7} The bar for \u{201C}Omega replaces CUDA\u{201D}" }
                    h2 class="om-h2" { (met) " of " (d.bar.len()) " met" }
                    p class="om-sub" {
                        "I say it when every row is PASS, each with a receipt pushed on the exact commit. No row is waived. A row is ticked here only when its receipt is in the data file; until then it shows where it stands on the CUDA map above."
                    }
                    (bar_list(&d.bar, "o:docs/OMEGA.md#3-the-bar-for-omega-replaces-cuda"))
                }
            }

            section id="beyond" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "4 \u{00B7} The rest of \u{201C}Omega replaces everything\u{201D}" }
                    h2 class="om-h2" { "Beyond the CUDA claim" }
                    p class="om-sub" { "These are not part of the CUDA claim. They are the rest of what Omega is for, and each has its own bar." }
                    div class="om-gaps" {
                        @for g in &d.beyond {
                            article class="om-beyond" {
                                h3 class="om-gap-title" { (g.title) }
                                p { (g.text) }
                                (inline_links(&g.links))
                                (box_foot(&g.box_))
                            }
                        }
                    }
                }

            section id="performance" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "5 \u{00B7} Performance: matching and then beating CUDA" }
                    h2 class="om-h2" { (pmet) " of " (d.performance.bar.len()) " met" }
                    article class="om-gap om-perf" {
                        p { (d.performance.stand) }
                        p { (d.performance.how) }
                        (box_foot(&d.performance.box_))
                    }
                    p class="om-sub om-perf-rule" { (d.performance.rule) }
                    (bar_list(&d.performance.bar, &d.performance.box_))
                }
            }
            }
        }
    }
}

/// A bar as a checklist: a row is ticked only when its receipt is in the data file.
fn bar_list(bar: &[Bar], fallback: &str) -> Markup {
    html! {
        ol class="om-bar" {
            @for b in bar {
                @let done = b.status == "PASS";
                li class=(format!("om-bar-row {}", if done { "om-done" } else { "om-open" })) id=(b.id.to_ascii_lowercase()) {
                    span class="om-check" aria-hidden="true" { @if done { "\u{2713}" } }
                    div class="om-bar-body" {
                        h3 class="om-bar-title" {
                            span class="om-bar-id mono" { (b.id) }
                            " " (b.title)
                            span class=(format!("om-pill {}", status_class(&b.status))) { @if done { "MET" } @else { "OPEN \u{00B7} " (b.status) } }
                        }
                        p class="om-bar-when" { strong { "Done when: " } (b.done_when) }
                        @if !b.now.is_empty() {
                            p class="om-bar-now" { strong { "Now: " } (b.now) }
                        }
                        (inline_links(&b.links))
                        @match &b.receipt {
                            Some(r) => { (box_foot(r)) }
                            None => { (box_foot(fallback)) }
                        }
                    }
                }
            }
        }
    }
}

fn meter_label(counts: &[(&str, usize)]) -> String {
    let parts: Vec<String> = counts.iter().map(|(s, n)| format!("{n} {s}")).collect();
    format!("The CUDA stack: {}", parts.join(", "))
}

/// Renders `text` with its first standalone "Omega" linked to /omega/.
/// Used wherever the site names Omega in running text.
pub fn link_first(text: &str) -> Markup {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(off) = text[from..].find("Omega") {
        let i = from + off;
        let j = i + "Omega".len();
        let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        let after_ok = j == text.len() || !bytes[j].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return html! { (&text[..i]) a class="omega-link" href="/omega/" { "Omega" } (&text[j..]) };
        }
        from = j;
    }
    html! { (text) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::boxlink::check_boxes;

    #[test]
    fn data_parses_and_validates() {
        let d = parse(RAW).unwrap();
        validate(&d).unwrap();
        assert_eq!(d.claim, "Omega is Aien's language for all computing, GPU to CPU.");
        assert_eq!(d.layers.len(), 13);
        assert_eq!(d.bar.len(), 6);
    }

    #[test]
    fn page_says_what_omega_is_first() {
        let s = render_omega_page().into_string();
        let h1 = s.find("<h1").unwrap();
        assert!(s[h1..].starts_with("<h1 class=\"om-title\">Omega is Aien&#39;s language for all computing, GPU to CPU.</h1>") || s[h1..].starts_with("<h1 class=\"om-title\">Omega is Aien's language for all computing, GPU to CPU.</h1>"), "{}", &s[h1..h1 + 120]);
        assert!(s.contains("On the GPU, that makes Omega the replacement for CUDA."));
        assert_eq!(s.matches("<h1").count(), 1);
    }

    #[test]
    fn missing_rows_are_shown_not_hidden() {
        let d = data();
        let s = render_omega_page().into_string();
        for l in &d.layers {
            assert!(s.contains(&format!("<span class=\"om-cuda\" role=\"cell\">{}</span>", l.cuda)), "row {} not rendered", l.cuda);
        }
        let missing = d.layers.iter().filter(|l| l.status == "MISSING").count();
        assert!(missing >= 1);
        assert_eq!(s.matches("om-layer om-missing").count(), missing);
        assert_eq!(s.matches("class=\"om-seg ").count(), d.layers.len());
    }

    #[test]
    fn every_box_links_to_github() {
        let s = render_omega_page().into_string();
        let n = check_boxes(&s).unwrap();
        let d = data();
        assert_eq!(n, d.parts.len() + 1 + d.layers.len() + 1 + d.not_yet.len() + d.bar.len() + d.beyond.len() + 1 + d.performance.bar.len());
        assert!(s.contains("https://github.com/aien-dev/omega-mv/blob/main/evidence/WHAT-OMEGA-IS-20261021/live-serve-no-cuda.txt"));
        assert!(s.contains("https://github.com/aien-dev/omega-mv/blob/main/docs/OMEGA.md"));
        assert!(!s.contains("https://github.com/aien-dev/omega/"));
    }

    #[test]
    fn bar_is_an_honest_checklist() {
        let s = render_omega_page().into_string();
        let d = data();
        let met = bar_met(d);
        let pmet = d.performance.bar.iter().filter(|b| b.status == "PASS").count();
        assert_eq!(s.matches("om-bar-row om-done").count(), met + pmet);
        assert_eq!(s.matches("om-bar-row om-open").count(), 6 - met + 5 - pmet);
        assert!(s.contains(&format!("{met} of 6 met")));
        for id in ["c1", "c2", "c3", "c4", "c5", "c6", "p0", "p1", "p2", "p3", "p4"] {
            assert!(s.contains(&format!("id=\"{id}\"")), "{id}");
        }
    }

    #[test]
    fn a_pass_without_receipt_is_refused() {
        let mut d = parse(RAW).unwrap();
        d.bar[0].status = "PASS".into();
        assert!(validate(&d).unwrap_err().contains("PASS without a receipt"));
        d.bar[0].receipt = Some("o:docs/OMEGA.md".into());
        assert!(validate(&d).unwrap_err().contains("under omega evidence/"));
        let mut d = parse(RAW).unwrap();
        d.performance.bar[0].receipt = Some("o:evidence/x".into());
        assert!(validate(&d).unwrap_err().contains("has a receipt but is MISSING"));
    }

    #[test]
    fn unknown_status_and_off_github_box_are_refused() {
        let d = parse(&RAW.replacen("\"status\": \"PARTIAL\"", "\"status\": \"ALMOST\"", 1)).unwrap();
        assert!(validate(&d).unwrap_err().contains("unknown status"));
        let d = parse(&RAW.replacen("\"box\": \"o/:evidence/E1-CLOSURE\"", "\"box\": \"/now/\"", 1)).unwrap();
        assert!(validate(&d).unwrap_err().contains("off GitHub"));
        assert!(expand("x:foo").is_err());
        assert!(expand("o:../etc").is_err());
    }

    #[test]
    fn link_expansion() {
        assert_eq!(expand("o:docs/OMEGA.md").unwrap(), "https://github.com/aien-dev/omega-mv/blob/main/docs/OMEGA.md");
        assert_eq!(expand("o/:src/checker2").unwrap(), "https://github.com/aien-dev/omega-mv/tree/main/src/checker2");
        assert_eq!(expand("ob:runtime/r2-hello").unwrap(), "https://github.com/aien-dev/omega-mv/tree/runtime/r2-hello");
        assert_eq!(expand("/now/#L332").unwrap(), "/now/#L332");
    }

    #[test]
    fn link_first_links_only_the_word() {
        assert_eq!(
            link_first("Aien proposes. Omega checks. Omega again.").into_string(),
            "Aien proposes. <a class=\"omega-link\" href=\"/omega/\">Omega</a> checks. Omega again."
        );
        assert_eq!(link_first("Omegese only").into_string(), "Omegese only");
        assert_eq!(link_first("Omega's tools").into_string(), "<a class=\"omega-link\" href=\"/omega/\">Omega</a>&#39;s tools".replace("&#39;", "'"));
    }

    #[test]
    fn no_dashes_and_plain_words() {
        let s = render_omega_page().into_string();
        assert!(!s.contains('\u{2014}') && !s.contains('\u{2013}'));
        let lower = s.to_ascii_lowercase();
        for w in ["revolutionary", "blazing", "cutting-edge", "seamless", "world-class", "powerful", "aienix"] {
            assert!(!lower.contains(w), "{w}");
        }
    }
}
