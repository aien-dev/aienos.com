use maud::{html, Markup, PreEscaped};

/// The six top-level sections of the site, in reading order
/// (research synthesis handoff, section 21). Every page header uses this
/// list: the generated pages render it here, and the hand-written pages
/// under public/ carry the same links (checked by the tests below).
pub const NAV_PRIMARY: [(&str, &str); 6] = [
    ("The Machine", "/machine/"),
    ("The Turing", "/turing/"),
    ("Experiments", "/experiments/"),
    ("Scientific Discovery", "/discovery/"),
    ("Evidence", "/evidence/"),
    ("Philosophy", "/philosophy/"),
];

/// Secondary links: smaller, after a divider.
pub const NAV_SECONDARY: [(&str, &str); 4] = [
    ("Progress", "/progress/"),
    ("Blog", "/blog"),
    ("Waitlist", "/#waitlist"),
    ("GitHub", "https://github.com/aien-dev"),
];

/// The exact anchor markup for one primary nav link. Hand-written pages
/// copy this markup byte for byte so the tests can find it.
pub fn primary_link_markup(label: &str, href: &str) -> String {
    format!("<a class=\"aien-nav-link\" href=\"{href}\">{label}</a>")
}

pub fn render_navbar() -> Markup {
    html! {
        header class="aien-header" {
            div class="aien-header-inner" {
                a class="aien-brand" href="/" aria-label="AIENOS home" {
                    span class="aien-brand-mark" aria-hidden="true" {
                        (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#))
                    }
                    span class="aien-brand-copy" {
                        span class="aien-brand-name" { "AIENOS" }
                        span class="aien-brand-tag" { "PERSISTENT MACHINE INTELLIGENCE" }
                    }
                }
                nav class="aien-nav" aria-label="Primary navigation" {
                    ul class="aien-nav-primary" {
                        @for (label, href) in NAV_PRIMARY {
                            li { a class="aien-nav-link" href=(href) { (label) } }
                        }
                    }
                    ul class="aien-nav-secondary" aria-label="More" {
                        @for (label, href) in NAV_SECONDARY {
                            li { a class="aien-nav-minor" href=(href) { (label) } }
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
    use std::path::Path;

    /// Written out independently of NAV_PRIMARY so reordering or dropping
    /// an entry in the const makes these tests fail.
    const EXPECTED_PRIMARY: [(&str, &str); 6] = [
        ("The Machine", "/machine/"),
        ("The Turing", "/turing/"),
        ("Experiments", "/experiments/"),
        ("Scientific Discovery", "/discovery/"),
        ("Evidence", "/evidence/"),
        ("Philosophy", "/philosophy/"),
    ];

    /// Hand-written pages that must carry the same six links.
    const HAND_WRITTEN: [&str; 3] = [
        "public/post-llm-case/index.html",
        "public/research/the-stapleton-doctrine/index.html",
        "public/licensing/index.html",
    ];

    fn assert_six_in_order(haystack: &str, what: &str) {
        let mut last = 0usize;
        for (i, (label, href)) in EXPECTED_PRIMARY.iter().enumerate() {
            let needle = primary_link_markup(label, href);
            let pos = haystack[last..]
                .find(&needle)
                .unwrap_or_else(|| panic!("{what}: nav item {i} ({label}) missing or out of order"));
            last += pos + needle.len();
        }
    }

    #[test]
    fn primary_nav_const_matches_section_21() {
        assert_eq!(NAV_PRIMARY, EXPECTED_PRIMARY, "NAV_PRIMARY differs from section 21 order");
    }

    #[test]
    fn rendered_navbar_has_six_sections_in_order_before_secondary() {
        let html = render_navbar().into_string();
        assert_six_in_order(&html, "generated navbar");
        let primary_end = html.find("</ul>").expect("primary list closes");
        let secondary = html.find("aien-nav-secondary").expect("secondary list present");
        assert!(secondary > primary_end, "secondary links must come after the six sections");
        for (label, _) in NAV_SECONDARY {
            assert!(html[secondary..].contains(label), "secondary item {label} missing");
            assert!(!html[..primary_end].contains(&format!(">{label}<")), "{label} must not be a primary item");
        }
    }

    #[test]
    fn hand_written_pages_carry_the_same_six_links() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for rel in HAND_WRITTEN {
            let page = std::fs::read_to_string(root.join(rel))
                .unwrap_or_else(|_| panic!("cannot read {rel}"));
            assert!(page.contains("class=\"aien-header\""), "{rel}: missing shared header markup");
            assert!(page.contains("/assets/site-nav.css"), "{rel}: missing shared nav stylesheet");
            assert_six_in_order(&page, rel);
        }
    }

    #[test]
    fn navbar_has_no_em_or_en_dash() {
        let html = render_navbar().into_string();
        assert!(!html.contains('\u{2014}') && !html.contains('\u{2013}'));
    }
}
