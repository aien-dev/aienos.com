//! /cudaplusplus/: CUDA++, the GPU half of Omega, as its own open source
//! project (aien-dev/CUDApp, MIT; GitHub slugs cannot hold a plus sign).
//!
//! The page renders, from `data/cudaplusplus.json`: what CUDA++ is and what
//! it is for; the path from a program to the chip, each hop lit by whose
//! code runs there (mine with a receipt, mine in part, mine being built,
//! NVIDIA's for a physical reason); how I did it, step by step; CUDA's
//! pain points and the fix for each; and the swap that takes CUDA off a
//! machine down to the kernel driver.
//!
//! The data file is checked at build time: an unknown status word, a PASS
//! without a receipt under evidence/, an IN PROGRESS row without its ledger
//! id, a box that does not link to GitHub, a link to a file on a branch
//! that is not pinned to a full commit, or an em or en dash all fail the
//! build and the tests.

use std::sync::OnceLock;

use maud::{html, Markup};
use serde::Deserialize;

use super::boxlink::{foot_labelled, Target};

pub const TITLE: &str = "CUDA++: Open-Source GPU Computing Without CUDA | AienOS";
pub const DESCRIPTION: &str = "CUDA++ is an MIT-licensed GPU toolchain for AI inference without the CUDA toolkit. GB10-qualified kernels, runtime, source, quickstart and replacement limits.";
pub const URL: &str = "https://www.aienos.com/cudaplusplus/";

/// The data file, compiled in and published as /cudaplusplus.json.
pub const RAW: &str = include_str!("../../data/cudaplusplus.json");

/// Status words a hop or row may carry.
pub const STATUSES: [&str; 4] = ["PASS", "PARTIAL", "IN PROGRESS", "NVIDIA"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub source: Source,
    pub claim: String,
    pub tagline: String,
    pub tagline_link: String,
    pub claim_rest: Vec<String>,
    pub purpose_title: String,
    pub purpose: String,
    pub purpose_link: String,
    pub private_note: String,
    pub audit_link: String,
    pub hops: Vec<Hop>,
    pub how_lead: String,
    pub steps: Vec<Card>,
    pub did_not: Vec<String>,
    pub did_not_box: String,
    pub pain_lead: String,
    pub pains: Vec<Row>,
    pub swap_lead: String,
    pub swaps: Vec<Row>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub repo: String,
    pub readme: String,
    pub how: String,
    pub how_branch: String,
    pub how_commit: String,
    pub main_commit: String,
    pub omega_doc: String,
    pub omega_branch: String,
    pub omega_commit: String,
    pub checked: String,
    pub ledger: String,
    pub visibility: String,
    pub owner_ledger: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hop {
    pub step: String,
    pub cuda: String,
    pub ours: String,
    pub status: String,
    pub note: String,
    #[serde(default)]
    pub ledger: Option<String>,
    pub links: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub title: String,
    pub text: String,
    pub links: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub title: String,
    pub text: String,
    pub status: String,
    #[serde(default)]
    pub ledger: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub receipt: Option<String>,
    pub links: Vec<(String, String)>,
    #[serde(rename = "box")]
    pub box_: String,
}

/// Expands a short link from the data file into a full address.
/// `cp:` / `cp/:` a file / folder on CUDA++ main; `cp@:` a CUDA++ file at a
/// full commit; `om@:` an omega file at a full commit; `ax@:` an Aienux file
/// at a full commit; `sc@/:` an aien-sovereign-core folder at a full commit;
/// `s:` a file of this site on main. A link starting with `/` or `#` stays
/// on this site. Files on branches are always pinned to a full commit.
pub fn expand(link: &str) -> Result<String, String> {
    const MAP: [(&str, &str, bool); 7] = [
        ("cp/:", "https://github.com/aien-dev/CUDApp/tree/main/", false),
        ("cp@:", "https://github.com/aien-dev/CUDApp/blob/", true),
        ("cp:", "https://github.com/aien-dev/CUDApp/blob/main/", false),
        ("om@:", "https://github.com/aien-dev/omega-mv/blob/", true),
        ("ax@:", "https://github.com/aien-dev/aienux/blob/", true),
        ("sc@/:", "https://github.com/aien-dev/aien-sovereign-core-mv/tree/", true),
        ("s:", "https://github.com/aien-dev/aienos.com/blob/main/", false),
    ];
    if link.starts_with('/') || link.starts_with('#') {
        return Ok(link.to_string());
    }
    for (prefix, base, pinned) in MAP {
        if let Some(rest) = link.strip_prefix(prefix) {
            if rest.is_empty() || rest.contains("..") || rest.contains(' ') {
                return Err(format!("bad link {link:?}"));
            }
            if pinned {
                let sha = rest.split('/').next().unwrap_or("");
                if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) || !rest[40..].starts_with('/') {
                    return Err(format!("a file on a branch must be pinned to a full commit: {link:?}"));
                }
            }
            return Ok(format!("{base}{rest}"));
        }
    }
    Err(format!("unknown link form {link:?}"))
}

fn gh(link: &str) -> String {
    let h = expand(link).unwrap_or_else(|e| panic!("data/cudaplusplus.json: {e}"));
    assert!(h.starts_with("https://github.com/"), "data/cudaplusplus.json: box link must be on GitHub: {link}");
    h
}

pub fn parse(raw: &str) -> Result<Data, String> {
    serde_json::from_str(raw).map_err(|e| format!("data/cudaplusplus.json does not parse: {e}"))
}

fn check_status(what: &str, status: &str, ledger: &Option<String>, receipt: Option<&str>) -> Result<(), String> {
    if !STATUSES.contains(&status) {
        return Err(format!("{what}: unknown status {status:?}"));
    }
    if status == "IN PROGRESS" {
        match ledger {
            Some(l) if l.starts_with('L') && l[1..].bytes().all(|b| b.is_ascii_digit()) && l.len() > 1 => {}
            _ => return Err(format!("{what}: IN PROGRESS needs its ledger id")),
        }
    }
    if status == "PASS" {
        match receipt {
            Some(r) if r.contains("evidence/") => {}
            _ => return Err(format!("{what}: PASS needs a receipt under evidence/")),
        }
    }
    Ok(())
}

pub fn validate(d: &Data) -> Result<(), String> {
    if RAW.contains('\u{2014}') || RAW.contains('\u{2013}') {
        return Err("data/cudaplusplus.json has an em or en dash".into());
    }
    let mut boxes: Vec<&str> = vec![&d.source.readme, &d.source.how, &d.source.omega_doc, &d.purpose_link, &d.audit_link, &d.did_not_box];
    let mut links: Vec<&str> = vec![&d.tagline_link];
    if d.hops.len() < 8 {
        return Err(format!("the path has {} hops; it runs from the program to the firmware (8)", d.hops.len()));
    }
    for h in &d.hops {
        check_status(&h.step, &h.status, &h.ledger, Some(&h.box_))?;
        boxes.push(&h.box_);
        links.extend(h.links.iter().map(|l| l.1.as_str()));
    }
    for c in &d.steps {
        boxes.push(&c.box_);
        links.extend(c.links.iter().map(|l| l.1.as_str()));
    }
    for r in d.pains.iter().chain(d.swaps.iter()) {
        if r.status == "NVIDIA" || r.status == "PARTIAL" {
            return Err(format!("{}: a pain or swap row is PASS or IN PROGRESS", r.title));
        }
        check_status(&r.title, &r.status, &r.ledger, r.receipt.as_deref())?;
        if r.status == "PASS" && r.command.is_none() {
            return Err(format!("{}: a fixed row shows the command that proves it", r.title));
        }
        boxes.push(&r.box_);
        if let Some(rc) = &r.receipt {
            boxes.push(rc);
        }
        links.extend(r.links.iter().map(|l| l.1.as_str()));
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
    if d.source.visibility != "private" && d.source.visibility != "public" {
        return Err("source.visibility is private or public".into());
    }
    Ok(())
}

pub fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        let d = parse(RAW).unwrap_or_else(|e| panic!("{e}"));
        validate(&d).unwrap_or_else(|e| panic!("data/cudaplusplus.json: {e}"));
        d
    })
}

fn status_class(s: &str) -> &'static str {
    match s {
        "PASS" => "cp-pass",
        "PARTIAL" => "cp-partial",
        "IN PROGRESS" => "cp-building",
        _ => "cp-nvidia",
    }
}

fn status_word(s: &str, ledger: &Option<String>) -> String {
    match (s, ledger) {
        ("IN PROGRESS", Some(l)) => format!("IN PROGRESS \u{00B7} {l}"),
        ("NVIDIA", _) => "NVIDIA'S".to_string(),
        _ => s.to_string(),
    }
}

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

/// A receipt in evidence/ is evidence; anything else is the source that
/// holds the text.
fn box_foot(link: &str) -> Markup {
    let h = gh(link);
    if link.contains("evidence/") {
        foot_labelled(&h, "Receipt on GitHub", Target::Evidence)
    } else {
        foot_labelled(&h, "Source on GitHub", Target::Source)
    }
}

fn ext(href: &str, text: &str) -> Markup {
    html! { a href=(href) target="_blank" rel="noopener noreferrer" { (text) } }
}

fn rows(list: &[Row]) -> Markup {
    html! {
        div class="cp-rows" {
            @for r in list {
                article class=(format!("cp-row {}", status_class(&r.status))) {
                    div class="cp-row-head" {
                        h3 class="cp-row-title" { (r.title) }
                        span class="cp-pill" { (status_word(&r.status, &r.ledger)) }
                    }
                    p { (r.text) }
                    @if let Some(c) = &r.command {
                        pre class="cp-cmd" { code { (c) } }
                    }
                    (inline_links(&r.links))
                    @match &r.receipt {
                        Some(rc) => { (box_foot(rc)) }
                        None => { (box_foot(&r.box_)) }
                    }
                }
            }
        }
    }
}

pub fn render_cudaplusplus_page() -> Markup {
    let d = data();
    let ours = d.hops.iter().filter(|h| h.status == "PASS").count();
    let building = d.hops.iter().filter(|h| h.status == "IN PROGRESS").count();
    html! {
        div class="om-page cp-page" {
            section class="om-hero" aria-label="CUDA++ overview" {
                div class="container" {
                    p class="om-kicker" { "CUDA++ \u{00B7} open source, MIT" }
                    h1 class="om-title" { (d.claim) }
                    p class="om-lead" {
                        (d.tagline)
                        " "
                        a href=(gh(&d.tagline_link)) target="_blank" rel="noopener noreferrer" { "Receipt on GitHub" }
                    }
                    @for p in &d.claim_rest {
                        p class="om-lead" { (p) }
                    }
                    div class="om-verdicts" {
                        div class="om-verdict om-verdict-true" {
                            p class="om-verdict-k" { "What it is for" }
                            p class="om-verdict-s" { (d.purpose_title) }
                            p class="cp-purpose" { (d.purpose) }
                            a class="om-verdict-a" href=(gh(&d.purpose_link)) target="_blank" rel="noopener noreferrer" { "Omega docs/OMEGA.md" }
                        }
                        div class="om-verdict om-verdict-not" {
                            p class="om-verdict-k" { "The repository" }
                            p class="om-verdict-s mono" { (ext(&d.source.repo, "aien-dev/CUDApp")) }
                            p class="cp-purpose" { (d.private_note) }
                            a class="om-verdict-a" href=(gh(&d.audit_link)) target="_blank" rel="noopener noreferrer" { "The audit, docs/AUDIT.md" }
                        }
                    }
                    p class="om-source" {
                        "Written from the CUDA++ "
                        (ext(&gh(&d.source.readme), "README"))
                        " (main at " span class="mono" { (d.source.main_commit) } "), "
                        (ext(&gh(&d.source.how), "docs/HOW_I_DID_IT.md"))
                        @if d.source.how_branch == "main" {
                            " (main at " span class="mono" { (d.source.how_commit) } ") "
                        } @else {
                            " (branch " span class="mono" { (d.source.how_branch) } " at " span class="mono" { (d.source.how_commit) } ") "
                        }
                        "and Omega's "
                        (ext(&gh(&d.source.omega_doc), "docs/OMEGA.md"))
                        " (branch " span class="mono" { (d.source.omega_branch) } " at " span class="mono" { (d.source.omega_commit) } "), checked " (d.source.checked) ". "
                        "Ledger item " a href=(format!("/now/#{}", d.source.ledger)) { (d.source.ledger) } " tracks this page; "
                        a href=(format!("/now/#{}", d.source.owner_ledger)) { (d.source.owner_ledger) } " owns the repository. "
                        "Work that is not done shows as in progress with its ledger id. The rest of Omega: "
                        a class="omega-link" href="/omega/" { "What Omega is" } ". Data file: "
                        a href="/cudaplusplus.json" { "/cudaplusplus.json" } "."
                    }
                    nav class="om-toc" aria-label="On this page" {
                        a href="#cuda-faq" { "Try CUDA++ and practical answers" }
                        a href="#path" { "From your program to the chip" }
                        a href="#how" { "How I did it" }
                        a href="#pain-points" { "CUDA's pain points" }
                        a href="#swap" { "Swap CUDA out" }
                    }
                }
            }

            (super::gpu_discovery::render_answers())

            section id="path" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "1 \u{00B7} From your program to the chip" }
                    h2 class="om-h2" { "Whose code runs at each step" }
                    p class="om-sub" {
                        "Each step a GPU program goes through, top to bottom, with the NVIDIA part CUDA uses there and what CUDA++ uses instead. The line on the left is lit by whose code runs: green, my code with a receipt, blue, my code in part, amber, my code being built, grey NVIDIA's. Today "
                        span class="mono" { (ours) } " steps are my code with a receipt and " span class="mono" { (building) } " is being built."
                    }
                    ol class="cp-path" {
                        @for h in &d.hops {
                            li class=(format!("cp-hop {}", status_class(&h.status))) {
                                span class="cp-node" aria-hidden="true" {}
                                div class="cp-hop-body" {
                                    p class="cp-step" { (h.step) }
                                    p class="cp-cuda" { span class="cp-lane" { "CUDA" } " " (h.cuda) }
                                    p class="cp-ours" { span class="cp-lane cp-lane-ours" { "CUDA++" } " " (h.ours) }
                                    (inline_links(&h.links))
                                }
                                div class="cp-hop-status" {
                                    span class="cp-pill" { (status_word(&h.status, &h.ledger)) }
                                    @if !h.note.is_empty() { span class="cp-note" { (h.note) } }
                                }
                                (box_foot(&h.box_))
                            }
                        }
                    }
                }
            }

            section id="how" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "2 \u{00B7} How I did it" }
                    h2 class="om-h2" { "Skip every closed layer" }
                    p class="om-sub" { (d.how_lead) }
                    div class="cp-steps" {
                        @for c in &d.steps {
                            article class="cp-card" {
                                h3 class="om-gap-title" { (c.title) }
                                p { (c.text) }
                                (inline_links(&c.links))
                                (box_foot(&c.box_))
                            }
                        }
                    }
                    article class="cp-card cp-didnot" {
                        h3 class="om-gap-title" { "What I did not do" }
                        ul class="cp-list" {
                            @for t in &d.did_not { li { (t) } }
                        }
                        (box_foot(&d.did_not_box))
                    }
                }
            }

            section id="pain-points" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "3 \u{00B7} CUDA's pain points" }
                    h2 class="om-h2" { "Each pain point, its fix, and the command that shows it" }
                    p class="om-sub" { (d.pain_lead) }
                    (rows(&d.pains))
                }
            }

            section id="swap" class="om-section" {
                div class="container" {
                    p class="om-kicker" { "4 \u{00B7} Swap CUDA out" }
                    h2 class="om-h2" { "Down to the kernel driver" }
                    p class="om-sub" { (d.swap_lead) }
                    (rows(&d.swaps))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::boxlink::check_boxes;

    #[test]
    fn discovery_uses_public_repo_and_scopes_the_opener() {
        let s = render_cudaplusplus_page().into_string();
        assert!(s.contains("https://github.com/aien-dev/CUDApp"));
        assert!(!s.contains("repository is private"));
        assert!(!s.contains("everything CUDA gives you"));
        assert!(s.contains("not a drop-in replacement"));
        assert!(s.contains("make hello-gpu"));
        assert!(s.contains("/ai-inference-speed/"));
        assert!(s.contains("/cuda-alternative/"));
    }

    #[test]
    fn data_parses_and_validates() {
        let d = parse(RAW).unwrap();
        validate(&d).unwrap();
        assert!(d.claim.starts_with("CUDA++ is the GPU half of the Omega runtime"));
        assert_eq!(d.tagline, "Bringing CUDA performance to other hardware.");
        assert!(expand(&d.tagline_link).unwrap().contains("c1a35914b2099f040494fba215576dd136a9d02f"));
        assert_eq!(d.steps.len(), 4);
    }

    #[test]
    fn page_names_it_with_the_plus_signs() {
        let s = render_cudaplusplus_page().into_string();
        assert!(s.contains("<h1 class=\"om-title\">CUDA++ is the GPU half of the Omega runtime, under the MIT licence so anyone can use it.</h1>"));
        assert_eq!(s.matches("<h1").count(), 1);
        assert!(s.contains("Bringing CUDA performance to other hardware."));
        assert!(s.contains("c1a35914b2099f040494fba215576dd136a9d02f/evidence/targets/wolfram.txt"));
        for id in ["path", "how", "pain-points", "swap"] {
            assert!(s.contains(&format!("id=\"{id}\"")), "{id}");
        }
    }

    #[test]
    fn every_box_links_to_github() {
        let s = render_cudaplusplus_page().into_string();
        let n = check_boxes(&s).unwrap();
        let d = data();
        assert_eq!(n, d.hops.len() + d.steps.len() + 1 + d.pains.len() + d.swaps.len());
        assert!(s.contains("https://github.com/aien-dev/CUDApp/blob/232332422bfdc31c8918085b50f13c1e18ab702e/docs/HOW_I_DID_IT.md"));
        assert!(s.contains("aien-dev/CUDApp"));
        assert!(s.contains("aien-dev/CUDAplusplus redirects"));
    }

    #[test]
    fn pending_work_is_never_shown_as_done() {
        let d = data();
        let s = render_cudaplusplus_page().into_string();
        for r in d.pains.iter().chain(d.swaps.iter()) {
            if r.status == "IN PROGRESS" {
                assert!(s.contains(&format!("IN PROGRESS \u{00B7} {}", r.ledger.as_ref().unwrap())));
            }
        }
        let driver = d.hops.iter().find(|h| h.cuda.contains("nvidia.ko")).unwrap();
        assert_eq!(driver.status, "IN PROGRESS");
        assert_eq!(driver.ledger.as_deref(), Some("L332"));
    }

    #[test]
    fn refusals() {
        let d = parse(&RAW.replacen("\"status\": \"PARTIAL\"", "\"status\": \"ALMOST\"", 1)).unwrap();
        assert!(validate(&d).unwrap_err().contains("unknown status"));
        let mut d = parse(RAW).unwrap();
        d.pains[0].status = "PASS".into();
        assert!(validate(&d).unwrap_err().contains("PASS needs a receipt"));
        let mut d = parse(RAW).unwrap();
        d.swaps[0].ledger = None;
        assert!(validate(&d).unwrap_err().contains("ledger id"));
        let mut d = parse(RAW).unwrap();
        d.pains[0].status = "PASS".into();
        d.pains[0].receipt = Some("cp/:evidence/X".into());
        assert!(validate(&d).unwrap_err().contains("command"));
        assert!(expand("cp@:docs/how-we-did-it/docs/HOW_WE_DID_IT.md").unwrap_err().contains("pinned"));
        assert!(expand("om@:89aad7c/docs/OMEGA.md").is_err());
        assert!(expand("x:foo").is_err());
        assert!(expand("cp:../x").is_err());
    }

    #[test]
    fn no_dashes_and_plain_words() {
        let s = render_cudaplusplus_page().into_string();
        assert!(!s.contains('\u{2014}') && !s.contains('\u{2013}'));
        let lower = s.to_ascii_lowercase();
        for w in ["revolutionary", "blazing", "cutting-edge", "seamless", "world-class", "powerful", "aienix", "game-chang"] {
            assert!(!lower.contains(w), "{w}");
        }
        assert!(!s.contains("cudaplusplus is") && !s.contains("CUDA plus plus"));
    }
}
