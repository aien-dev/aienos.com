use maud::{html, Markup, PreEscaped};

/// One footer link: (label, href, opens in a new tab).
pub type FooterLink = (&'static str, &'static str, bool);

/// Footer link groups, one per top-level section (handoff section 21),
/// in the same order as the primary nav. Each group links its hub first.
pub const FOOTER_GROUPS: [(&str, &[FooterLink]); 6] = [
    (
        "The Machine",
        &[
            ("The Machine", "/machine/", false),
            ("AIENOS source", "https://github.com/aien-dev/aienos", true),
            ("Omega source", "https://github.com/aien-dev/omega", true),
            ("Architecture", "https://github.com/aien-dev/aien-architecture", true),
            ("Physics", "https://github.com/aien-dev/physics", true),
        ],
    ),
    (
        "The Turing",
        &[
            ("The Turing", "/turing/", false),
            ("The Turing paper", "/research/computing-machinery-and-understanding/", false),
        ],
    ),
    (
        "Experiments",
        &[
            ("Experiments", "/experiments/", false),
            ("Research index", "/research/", false),
        ],
    ),
    (
        "Scientific Discovery",
        &[("Scientific Discovery", "/discovery/", false)],
    ),
    (
        "Evidence",
        &[
            ("Evidence", "/evidence/", false),
            (
                "Qualification record",
                "https://github.com/aien-dev/omega/blob/main/docs/turing/TURING_SCIENTIFIC_QUALIFICATION_STATE.md",
                true,
            ),
        ],
    ),
    (
        "Philosophy",
        &[
            ("Philosophy", "/philosophy/", false),
            ("The Stapleton Doctrine", "/research/the-stapleton-doctrine/", false),
            ("The Post-LLM Case", "/post-llm-case/", false),
        ],
    ),
];

/// Secondary links, after the six sections.
const FOOTER_MORE: [FooterLink; 5] = [
    ("Blog", "/blog", false),
    ("Waitlist", "/#waitlist", false),
    ("Licensing", "/licensing/", false),
    ("GitHub", "https://github.com/aien-dev", true),
    ("Drake Stapleton", "https://www.drakestapleton.com", true),
];

fn footer_link(link: &FooterLink) -> Markup {
    let (label, href, external) = *link;
    html! {
        li {
            @if external {
                a href=(href) target="_blank" rel="noopener noreferrer" { (label) }
            } @else {
                a href=(href) { (label) }
            }
        }
    }
}

pub fn render_footer() -> Markup {
    html! {
        footer class="aien-footer" {
            div class="container" {
                div class="aien-footer-top" {
                    div class="aien-footer-about" {
                        div class="aien-footer-brand" {
                            (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#))
                            span { "AIENOS" }
                        }
                        p {
                            "Persistent machine intelligence on hardware you own. Experimental, open source, and verified in public, receipts and all."
                        }
                    }
                    nav class="aien-footer-sections" aria-label="Site sections" {
                        @for (heading, links) in FOOTER_GROUPS {
                            div class="aien-footer-group" {
                                h2 class="aien-footer-heading" { (heading) }
                                ul {
                                    @for link in links {
                                        (footer_link(link))
                                    }
                                }
                            }
                        }
                        div class="aien-footer-group aien-footer-more" {
                            h2 class="aien-footer-heading" { "More" }
                            ul {
                                @for link in &FOOTER_MORE {
                                    (footer_link(link))
                                }
                            }
                        }
                    }
                }

                div class="aien-footer-legal" {
                    div {
                        "AIEN © 2026. Licensed under "
                        a href="/licensing/" { "Apache-2.0 WITH LLVM-exception" }
                        "."
                    }
                    div class="aien-footer-mottos" {
                        span { "RECEIPTS OVER CLAIMS" }
                        span { "ZERO_UNSOLICITED_TELEMETRY" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_HEADINGS: [&str; 6] = [
        "The Machine",
        "The Turing",
        "Experiments",
        "Scientific Discovery",
        "Evidence",
        "Philosophy",
    ];

    #[test]
    fn footer_groups_follow_the_six_sections_in_order() {
        let html = render_footer().into_string();
        let mut last = 0usize;
        for heading in EXPECTED_HEADINGS {
            let needle = format!("<h2 class=\"aien-footer-heading\">{heading}</h2>");
            let pos = html[last..]
                .find(&needle)
                .unwrap_or_else(|| panic!("footer heading {heading} missing or out of order"));
            last += pos + needle.len();
        }
        let more = html.find(">More</h2>").expect("More group present");
        assert!(more > last, "secondary group must follow the six sections");
    }

    #[test]
    fn each_footer_group_links_its_hub_first() {
        let hubs = ["/machine/", "/turing/", "/experiments/", "/discovery/", "/evidence/", "/philosophy/"];
        for ((heading, links), hub) in FOOTER_GROUPS.iter().zip(hubs) {
            assert_eq!(links[0].1, hub, "footer group {heading} must link {hub} first");
        }
    }

    #[test]
    fn footer_has_no_em_or_en_dash() {
        let html = render_footer().into_string();
        assert!(!html.contains('\u{2014}') && !html.contains('\u{2013}'));
    }
}
