use maud::{html, Markup};

use crate::facts::{short_date, Facts};

/// The components table. Every cell that can change is read from `facts.json`.
pub fn render_status(facts: &Facts) -> Markup {
    html! {
        section class="section" id="components" {
            div class="container" {
                div class="section-head" {
                    span class="section-num" { "02" }
                    div {
                        h2 { "The components, as of today" }
                        p class="lede" {
                            "Six repositories make up the core. Each is pre-alpha and says so. The last two columns are read from GitHub when this page is built, not typed by hand."
                        }
                    }
                }
                table class="status-table" {
                    thead {
                        tr {
                            th { "Component" }
                            th { "What it is" }
                            th { "Last merged change" }
                            th style="text-align:right" { "Open changes" }
                        }
                    }
                    tbody {
                        @for c in &facts.components {
                            tr {
                                td class="name" {
                                    a href=(c.url) rel="noopener" { (c.name) }
                                    small { (c.repo.rsplit('/').next().unwrap_or(&c.repo)) }
                                }
                                td { (c.role) }
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
                p class="stamp" {
                    "Facts gathered from the public GitHub record at " (facts.generated_at) ". This page is rebuilt every day and on every change to the site."
                }
            }
        }
    }
}
