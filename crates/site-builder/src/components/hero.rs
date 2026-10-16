use maud::{html, Markup};

use crate::facts::{short_date, Facts};

fn short_sha(sha: &str) -> &str {
    if sha.len() >= 12 { &sha[..12] } else { sha }
}

/// The one memorable element on the page: a real verification receipt for the latest commit
/// on the sovereign core's main branch, published by the Spark judge. Not a mock-up.
pub fn render_receipt(facts: &Facts) -> Markup {
    let r = &facts.receipt;
    let (class, verdict) = match r.state.as_str() {
        "success" => ("pass", "PASS"),
        "failure" | "error" => ("fail", "FAIL"),
        "pending" => ("pending", "PENDING"),
        _ => ("pending", "NO RECEIPT YET"),
    };
    let repo_short = r.repo.rsplit('/').next().unwrap_or(&r.repo);
    html! {
        aside class="receipt" id="receipt" aria-label="Latest verification receipt" {
            h3 { "AIEN VERIFICATION" }
            div class="rule" {}
            div class="row" { span { "repository" } span { (repo_short) } }
            div class="row" { span { "branch" } span { "main" } }
            div class="row" { span { "commit" } span { (short_sha(&r.sha)) } }
            @if !r.committed_at.is_empty() {
                div class="row" { span { "committed" } span { (short_date(&r.committed_at)) } }
            }
            div class="row" { span { "judge" } span { "the Spark itself" } }
            div class="rule" {}
            @if !r.commit_title.is_empty() {
                p { (r.commit_title) }
                div class="rule" {}
            }
            div class=(format!("verdict {class}")) { (verdict) }
            @if !r.description.is_empty() {
                p class="small" style="color:#5a5a5a; margin-top:4px;" { (r.description) }
            }
            div class="foot" {
                @if !r.target_url.is_empty() {
                    @if !r.updated_at.is_empty() { "Published " (short_date(&r.updated_at)) ". " }
                    a href=(r.target_url) rel="noopener" { "Open the receipt" }
                } @else {
                    "No verification has been published for this commit."
                }
            }
        }
    }
}

pub fn render_hero(facts: &Facts) -> Markup {
    html! {
        section class="hero" {
            div class="container" {
                div class="hero-copy" {
                    h1 { "Persistent machine intelligence on a machine you own." }
                    p {
                        "AIEN is a sovereign computing stack: an operating system, a language with its own GPU engine, a host runtime, and a pipeline that verifies every change. It is written to run as close to the hardware as the hardware allows, and it is built in the open on one NVIDIA DGX Spark."
                    }
                    p class="status-line" {
                        "Pre-alpha. Nothing on this page is a release. Every changing figure here comes from a public record and links to it."
                    }
                    div class="hero-actions" {
                        a class="button primary" href="https://github.com/aien-dev" rel="noopener" { "Read the source" }
                        a class="button" href="#verification" { "How a change gets in" }
                    }
                }
                (render_receipt(facts))
            }
        }
    }
}
