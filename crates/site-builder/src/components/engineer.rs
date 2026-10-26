//! About the engineer (/engineer/): who builds AienOS. drakestapleton.com was
//! merged into this section on 2026-10-26; its old addresses redirect here.
//!
//! The pages live in `public/data/engineer.json` (published as
//! /data/engineer.json): each page has its path, the drakestapleton.com
//! paths that now point at it, a title, a description and its body. Two
//! placeholders in a body are filled from facts.json at build time:
//! `<!--engineer:live-->` (the AIEN-today block) and `<!--engineer:verdict-->`
//! (the Spark judge's verdict card). Styles are in /assets/engineer.css.

use std::fs;
use std::path::Path;

use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

use crate::facts::Facts;
use crate::layouts::base::render_page_layout;

pub const RAW: &str = include_str!("../../../../public/data/engineer.json");
const FACTS_URL: &str = "https://www.aienos.com/facts.json";
const LIVE: &str = "<!--engineer:live-->";
const VERDICT: &str = "<!--engineer:verdict-->";

#[derive(Deserialize)]
pub struct Page {
    pub path: String,
    pub name: String,
    pub old: Vec<String>,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub body_html: String,
}

#[derive(Deserialize)]
struct Data {
    pages: Vec<Page>,
}

pub fn pages() -> Vec<Page> {
    serde_json::from_str::<Data>(RAW).expect("public/data/engineer.json").pages
}

/// Subpages carry one quiet way back; there is no section bar.
fn back_link() -> Markup {
    html! {
        p class="engineer-back" { a href="/engineer/" { "About the engineer" } }
    }
}

/// The foot of /engineer/: every page of the section, from the data file.
fn index(all: &[Page]) -> Markup {
    html! {
        section class="engineer-index" id="index" aria-labelledby="engineer-index-title" {
            h2 id="engineer-index-title" { "Everything in this section" }
            ul {
                @for p in all.iter().filter(|p| p.kind != "home") {
                    li { a href=(p.path) { (p.name) } " " span { (p.description) } }
                }
            }
        }
    }
}

fn verdict_word(state: &str, description: &str) -> &'static str {
    match state {
        "success" => "PASS",
        "failure" if description.starts_with("BLOCKED") && description.contains("NOT_RUN") => "BLOCKED",
        "failure" => "FAIL",
        "pending" => "RUNNING",
        _ => "NO VERDICT",
    }
}

fn verdict_card(facts: &Facts) -> Markup {
    let r = &facts.receipt;
    html! {
        div class="stat-card" {
            p class={ "stat-value tabular-nums verdict-" (r.state) } { (verdict_word(&r.state, &r.description)) }
            p class="stat-unit" { "Spark judge, sovereign core" }
            p class="stat-label" {
                "Verdict for commit " (&r.sha[..r.sha.len().min(7)]) " of the host runtime: " (r.description) ". "
                @if !r.target_url.is_empty() { a href=(r.target_url) { "Receipt" } }
            }
        }
    }
}

fn live_block(facts: &Facts) -> Markup {
    html! {
        section class="live-record" id="live" {
            h2 { "AIEN today" }
            p {
                "This block is not typed by hand. It is read from the record "
                a href=(FACTS_URL) { "facts.json" }
                " that this site gathers from GitHub every hour, when the page is built."
            }
            div class="grid stats-grid" { (verdict_card(facts)) }
            @if !facts.components.is_empty() {
                table class="live-table" {
                    thead { tr { th { "Component" } th { "Last merged change" } th { "Open proposals" } } }
                    tbody {
                        @for c in &facts.components {
                            tr {
                                td { a href={ "https://github.com/" (c.repo) } { (c.name) } }
                                td {
                                    @match &c.last_merge {
                                        Some(m) if !m.url.is_empty() => a href=(m.url) { (m.title) },
                                        Some(m) => (m.title),
                                        None => "none recorded",
                                    }
                                }
                                td class="tabular-nums" { (c.open_prs) }
                            }
                        }
                    }
                }
            }
            p class="live-generated" { "Record generated " (facts.generated_at) "." }
        }
    }
}

pub fn render(page: &Page, all: &[Page], facts: &Facts) -> Markup {
    let body = page
        .body_html
        .replace(LIVE, &live_block(facts).into_string())
        .replace(VERDICT, &verdict_card(facts).into_string());
    html! {
        div class="engineer" {
            @if page.kind != "home" { (back_link()) }
            (PreEscaped(body))
            @if page.kind == "home" { (index(all)) }
        }
    }
}

/// Writes every page of the section into dist.
pub fn emit(dist: &Path, facts: &Facts) {
    let all = pages();
    for page in &all {
        let head = html! {
            link rel="stylesheet" href="/assets/engineer.css";
            @if page.kind == "infographic" { link rel="stylesheet" href="/assets/engineer-infographic.css"; }
            @if page.kind == "home" { script defer src="/js/engineer-universe.js" {} }
        };
        let og_type = if page.kind == "work" { "article" } else { "website" };
        let doc = render_page_layout(
            &page.title,
            &page.description,
            &format!("https://www.aienos.com{}", page.path),
            og_type,
            head,
            render(page, &all, facts),
        );
        let dir = dist.join(page.path.trim_start_matches('/'));
        fs::create_dir_all(&dir).expect("create engineer page directory");
        fs::write(dir.join("index.html"), doc.into_string()).expect("write engineer page");
    }
    println!("  [PAGE] Emitted /engineer/ ({} pages, merged from drakestapleton.com)", all.len());
}

/// The section is whole: every page written, both placeholders filled, no
/// outside resource, every old drakestapleton.com path mapped exactly once,
/// and every internal link inside the section resolving to a written file.
pub fn verify(dist: &Path) {
    let all = pages();
    let mut olds = std::collections::HashSet::new();
    for page in &all {
        assert!(page.path.starts_with("/engineer/") && page.path.ends_with('/'), "engineer path {}", page.path);
        assert!(!page.old.is_empty(), "{} lists no old drakestapleton.com path", page.path);
        for o in &page.old {
            assert!(olds.insert(o.clone()), "old path {o} maps to two engineer pages");
        }
        let html = fs::read_to_string(dist.join(page.path.trim_start_matches('/')).join("index.html"))
            .unwrap_or_else(|_| panic!("missing {}", page.path));
        for bad in [LIVE, VERDICT, "drakestapleton.com/", "i.ytimg.com", "formsubmit.co", "fonts.googleapis.com", "\u{2014}", "\u{2013}"] {
            assert!(!html.contains(bad), "{} contains {bad:?}", page.path);
        }
        assert!(page.kind == "home" || html.contains("class=\"engineer-back\""), "{} lacks its way back to /engineer/", page.path);
        for src in html.split("src=\"").skip(1).filter_map(|s| s.split('"').next()) {
            assert!(src.starts_with('/') && !src.starts_with("//"), "{} loads an outside resource: {src}", page.path);
        }
        for href in html.split("href=\"").skip(1).filter_map(|s| s.split('"').next()) {
            let target = href.split('#').next().unwrap_or("");
            if target.starts_with("/engineer/") {
                let file = dist.join(target.trim_start_matches('/'));
                let file = if target.ends_with('/') { file.join("index.html") } else { file };
                assert!(file.exists(), "{} links {href}, which was not written", page.path);
            }
        }
    }
    for must in ["/", "/glossary", "/works", "/path", "/symphony/map", "/works/aien", "/aien", "/infographics/chatgpt-vs-aien"] {
        assert!(olds.contains(must), "no engineer page takes over drakestapleton.com{must}");
    }
    let learn = fs::read_to_string(dist.join("learn/index.html")).expect("read learn page");
    assert!(learn.contains("href=\"/engineer/\""), "/learn/ must link About the engineer");
    println!("  About the engineer: {} pages, {} old drakestapleton.com paths mapped", all.len(), olds.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_parses_and_the_home_carries_the_live_block() {
        let all = pages();
        assert!(all.iter().any(|p| p.path == "/engineer/" && p.body_html.contains(LIVE)));
        assert!(all.iter().any(|p| p.path == "/engineer/evidence/" && p.body_html.contains(VERDICT)));
        assert!(all.iter().any(|p| p.path == "/engineer/glossary/"));
    }
}
