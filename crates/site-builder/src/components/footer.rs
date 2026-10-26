use maud::{html, Markup, PreEscaped};

/// One footer link: (label, href).
pub type FooterLink = (&'static str, &'static str);

/// The whole footer menu. The six sections already sit in the header, and
/// every other page is one click away in the site map, so the footer keeps
/// only the few routes a reader looks for at the bottom of a page.
pub const FOOTER_LINKS: [FooterLink; 7] = [
    ("Start here", "/start/"),
    ("Evidence", "/evidence/"),
    ("Blog", "/blog"),
    ("Contact", "/contact/"),
    ("Privacy", "/privacy/"),
    ("Site map", "/sitemap.md"),
    ("About the engineer", "/engineer/"),
];

pub fn render_footer() -> Markup {
    html! {
        footer class="aien-footer" {
            div class="container" {
                div class="aien-footer-row" {
                    a class="aien-footer-brand" href="/" {
                        (PreEscaped(r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#))
                        span { "AienOS" }
                    }
                    nav class="aien-footer-links" aria-label="Footer" {
                        ul {
                            @for (label, href) in FOOTER_LINKS {
                                li { a href=(href) { (label) } }
                            }
                        }
                    }
                }
                div class="aien-footer-legal" {
                    (crate::components::contact::footer_reach(false))
                    p {
                        "Aien © 2026 · "
                        a href="/licensing/" { "Apache-2.0 WITH LLVM-exception" }
                        " · No trackers unless you say hello to Aien."
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
    fn footer_stays_short() {
        let html = render_footer().into_string();
        assert!(FOOTER_LINKS.len() <= 7, "the footer menu is a handful of links, not a site map");
        assert!(!html.contains("<h2"), "the footer carries no headed link columns");
        for (_, href) in FOOTER_LINKS {
            assert!(html.contains(&format!("href=\"{href}\"")), "footer lacks {href}");
        }
    }

    #[test]
    fn footer_reaches_drake() {
        let html = render_footer().into_string();
        for needle in ["href=\"https://x.com/mdrakestapleton\"", "href=\"mailto:drake@aienos.com\"", "href=\"/contact/\"", "href=\"/privacy/\"", "href=\"/sitemap.md\""] {
            assert!(html.contains(needle), "footer lacks {needle}");
        }
        assert!(html.contains(&format!("href=\"{}\"", crate::components::contact::ALLEN)), "footer lacks the Aien route");
    }

    #[test]
    fn footer_has_no_em_or_en_dash() {
        let html = render_footer().into_string();
        assert!(!html.contains('\u{2014}') && !html.contains('\u{2013}'));
    }
}
