//! The live part of the status section: facts read from the public GitHub record at build
//! time (`facts.json`, written by `site-builder facts`). Nothing here is typed by hand.

use maud::{html, Markup};

use crate::facts::{short_date, Facts};

use super::boxlink::{foot, Target};

fn short_sha(sha: &str) -> &str {
    if sha.len() >= 12 {
        &sha[..12]
    } else {
        sha
    }
}

/// The receipt on GitHub: the check run itself when GitHub hosts it, else
/// the commit it judged.
fn receipt_link(r: &crate::facts::Receipt) -> String {
    if r.target_url.starts_with("https://github.com/") {
        r.target_url.clone()
    } else if !r.sha.is_empty() {
        format!("https://github.com/{}/commit/{}", r.repo, r.sha)
    } else {
        format!("https://github.com/{}/commits/main", r.repo)
    }
}

/// A real verification receipt for the latest commit on the sovereign core's main branch,
/// as published by the Spark judge's check run. Set like a till print, because it is one.
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
                p class="receipt-note" { (r.description) }
            }
            div class="foot" {
                @if !r.target_url.is_empty() {
                    @if !r.updated_at.is_empty() { "Published " (short_date(&r.updated_at)) ". " }
                    @if !r.target_url.starts_with("https://github.com/") {
                        a href=(r.target_url) rel="noopener" { "Open the receipt" } " "
                    }
                } @else {
                    "No verification has been published for this commit. "
                }
                (foot(&receipt_link(r), Target::Evidence))
            }
        }
    }
}

/// The core components with their last merged change and open changes, plus the newest
/// decision records. Rendered inside the status section.
pub fn render_live(facts: &Facts) -> Markup {
    html! {
        div class="live" id="live" {
            div class="live-head" {
                div class="badge badge-blue" { "READ FROM THE PUBLIC RECORD" }
                h3 { "The components, as of the last build" }
                p {
                    "Six repositories make up the core. The last two columns are read from GitHub when this page is built, not typed by hand. Facts gathered at "
                    span class="mono" { (facts.generated_at) }
                    "; the page is rebuilt every hour."
                }
            }
            div class="live-grid" {
                table class="status-table" {
                    thead {
                        tr {
                            th { "Component" }
                            th { "What it is" }
                            th { "Last merged change" }
                            th class="num" { "Open" }
                        }
                    }
                    tbody {
                        @for c in &facts.components {
                            tr class="live-row" {
                                td class="name" {
                                    a href=(c.url) rel="noopener" { (c.name) }
                                    small { (c.repo.rsplit('/').next().unwrap_or(&c.repo)) }
                                    (foot(&c.url, Target::Evidence))
                                }
                                td {
                                    (c.role)
                                    @if c.name == "Omega" {
                                        " " a class="omega-link" href="/omega/" { "What Omega is" }
                                    }
                                }
                                td {
                                    @if let Some(m) = &c.last_merge {
                                        a class="merge" href=(m.url) rel="noopener" { (m.title) }
                                        span class="when" { (short_date(&m.merged_at)) }
                                    } @else {
                                        span class="merge" { "No merged changes on record" }
                                    }
                                }
                                td class="num" { (c.open_prs) }
                            }
                        }
                    }
                }
                (render_receipt(facts))
            }
            @if !facts.decisions.is_empty() {
                div class="decisions" {
                    h4 { "Newest decision records" }
                    ul {
                        @for d in &facts.decisions {
                            li {
                                a href=(d.url) rel="noopener" {
                                    span class="mono" { "ADR " (d.id) }
                                    " " (d.title)
                                }
                            }
                        }
                    }
                    p class="small" {
                        a href="https://github.com/aien-dev/aien-architecture/tree/main/docs/adr" rel="noopener" { "All decision records" }
                        " · "
                        a href="/facts.json" { "facts.json" }
                        ", the data behind this block."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_and_component_rows_link_to_github() {
        let facts: Facts = serde_json::from_str(include_str!("../../../../facts.json"))
            .expect("facts.json parses");
        let page = render_live(&facts).into_string();
        let rows = facts.components.len();
        assert!(rows >= 1);
        assert_eq!(
            crate::components::boxlink::check_boxes(&page),
            Ok(rows + 1),
            "every component row and the receipt link to GitHub"
        );
        let mut r = facts.receipt.clone();
        r.target_url = "https://spark.example/receipt".into();
        r.sha = "abc123".into();
        assert_eq!(
            receipt_link(&r),
            format!("https://github.com/{}/commit/abc123", r.repo)
        );
        r.target_url.clear();
        r.sha.clear();
        assert!(receipt_link(&r).ends_with("/commits/main"));
    }
}
