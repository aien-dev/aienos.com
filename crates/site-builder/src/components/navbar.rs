use maud::{html, Markup, PreEscaped};

/// The six destinations of the site, in reading order (the approved Omega
/// redesign, 2026-10-26): Sovereign Intelligence in its classic readable
/// form, then Omega, the Machine, Discoveries, Learn and Allen. The root is
/// Allen's native front door and the redesign has no top nav bar; every page
/// still links all six, in this order, somewhere a reader can reach (a quiet
/// index or footer). [`check_destinations`] holds every page in dist to that.
pub const NAV_PRIMARY: [(&str, &str); 6] = [
    ("SI", "/classic/"),
    ("Omega", "/omega/"),
    ("Machine", "/machine/"),
    ("Discoveries", "/discoveries/"),
    ("Learn", "/learn/"),
    ("Allen", "/allen/"),
];

/// Secondary links: smaller, after a divider.
pub const NAV_SECONDARY: [(&str, &str); 7] = [
    ("Start here", "/start/"),
    ("CUDA++", "/cudaplusplus/"),
    ("Now", "/now/"),
    ("Progress", "/progress/"),
    ("Blog", "/blog"),
    ("Follow", "/#follow"),
    ("GitHub", "https://github.com/aien-dev"),
];

/// Translated copies of the site, written into public/<code>/ by
/// aien-translate's scripts/translate_site.py (https://github.com/aien-dev/aien-translate).
/// (code, the language's own name). Each needs public/<code>/index.html.
pub const LANGUAGES: [(&str, &str); 1] = [("zh-CN", "简体中文")];

/// Every page links the six [`NAV_PRIMARY`] dashboards, in order, each an
/// `<a>` with exactly that href and that reader-visible label, among the
/// links a reader can reach (scripts, templates and `aria-hidden` content
/// do not count). Where they sit (a quiet index, a footer) and the markup
/// around them are free, and other links may come between them.
pub fn check_destinations(html: &str) -> Result<(), String> {
    let links = crate::oracle::anchors(&crate::oracle::reachable(html));
    let mut next = 0usize;
    for (label, href) in NAV_PRIMARY {
        let at = links[next..]
            .iter()
            .position(|(h, t)| h == href && t == label)
            .ok_or(format!("destination {label} ({href}) missing, hidden or out of order"))?;
        next += at + 1;
    }
    Ok(())
}

pub fn render_navbar() -> Markup {
    html! {
        header class="aien-header" {
            div class="aien-header-inner" {
                a class="aien-brand" href="/" aria-label="AienOS PERSISTENT MACHINE INTELLIGENCE: home" {
                    span class="aien-brand-mark" aria-hidden="true" {
                        (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>"#))
                    }
                    span class="aien-brand-copy" {
                        span class="aien-brand-name" { "AienOS" }
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
                    ul class="aien-nav-secondary aien-nav-lang" aria-label="Language" translate="no" {
                        li { a class="aien-nav-minor" href="/" hreflang="en" lang="en" { "English" } }
                        @for (code, name) in LANGUAGES {
                            li { a class="aien-nav-minor" href=(format!("/{code}/")) hreflang=(code) lang=(code) { (name) } }
                        }
                    }
                    (crate::components::contact::nav_reach(false))
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
        ("SI", "/classic/"),
        ("Omega", "/omega/"),
        ("Machine", "/machine/"),
        ("Discoveries", "/discoveries/"),
        ("Learn", "/learn/"),
        ("Allen", "/allen/"),
    ];

    fn assert_six_in_order(haystack: &str, what: &str) {
        let mut last = 0usize;
        for (i, (label, href)) in EXPECTED_PRIMARY.iter().enumerate() {
            let needle = format!("<a class=\"aien-nav-link\" href=\"{href}\">{label}</a>");
            let pos = haystack[last..]
                .find(&needle)
                .unwrap_or_else(|| panic!("{what}: nav item {i} ({label}) missing or out of order"));
            last += pos + needle.len();
        }
    }

    /// A page in the shape of the redesign: no top nav, the six
    /// destinations in a quiet footer index, each a glyph marked
    /// aria-hidden plus its label.
    fn quiet(items: &[(&str, &str)]) -> String {
        let mut s = String::from("<body><a class=\"site-brand\" href=\"/\"><span aria-hidden=\"true\">SI</span>Sovereign Intelligence</a><main>body</main><footer><nav class=\"quiet-index\" aria-label=\"Destinations\">");
        for (label, href) in items {
            s.push_str(&format!("<a href=\"{href}\"><span class=\"nav-glyph\" aria-hidden=\"true\">&#937;</span><span>{label}</span></a>"));
        }
        s.push_str("</nav></footer></body>");
        s
    }

    #[test]
    fn primary_nav_const_matches_the_six_dashboards() {
        assert_eq!(NAV_PRIMARY, EXPECTED_PRIMARY, "NAV_PRIMARY differs from the approved six dashboards");
    }

    #[test]
    fn rendered_navbar_has_six_sections_in_order_before_secondary() {
        let html = render_navbar().into_string();
        assert_six_in_order(&html, "generated navbar");
        check_destinations(&html).expect("the Rust navbar passes the dist check");
        let primary_end = html.find("</ul>").expect("primary list closes");
        let secondary = html.find("aien-nav-secondary").expect("secondary list present");
        assert!(secondary > primary_end, "secondary links must come after the six sections");
        for (label, _) in NAV_SECONDARY {
            assert!(html[secondary..].contains(label), "secondary item {label} missing");
            assert!(!html[..primary_end].contains(&format!(">{label}<")), "{label} must not be a primary item");
        }
    }

    #[test]
    fn quiet_index_passes_the_destination_check() {
        check_destinations(&quiet(&EXPECTED_PRIMARY)).expect("six in order, glyphs hidden");
        let mut extra = EXPECTED_PRIMARY.to_vec();
        extra.insert(2, ("Now", "/now/"));
        check_destinations(&quiet(&extra)).expect("other links between the six are allowed");
    }

    #[test]
    fn destination_check_negative_controls() {
        let six = EXPECTED_PRIMARY;
        let mut swapped = six.to_vec();
        swapped.swap(1, 2);
        let mut relabelled = six.to_vec();
        relabelled[3] = ("Scientific Discovery", "/discoveries/");
        let mut rerouted = six.to_vec();
        rerouted[3] = ("Discoveries", "/discovery/");
        let good = quiet(&six);
        let learn = "<a href=\"/learn/\"><span class=\"nav-glyph\" aria-hidden=\"true\">&#937;</span><span>Learn</span></a>";
        let cases = [
            ("one item dropped", quiet(&six[..5])),
            ("two items swapped", quiet(&swapped)),
            ("label changed", quiet(&relabelled)),
            ("href changed", quiet(&rerouted)),
            ("label only in the hidden glyph", good.replace(learn, "<a href=\"/learn/\"><span aria-hidden=\"true\">Learn</span></a>")),
            ("link inside an aria-hidden index", good.replace("aria-label=\"Destinations\"", "aria-hidden=\"true\"")),
            ("link only in a script", good.replace(learn, &format!("<script>document.write('{learn}')</script>"))),
            ("link only in a template", good.replace(learn, &format!("<template>{learn}</template>"))),
            ("hidden element never closes", good.replace("<nav class=\"quiet-index\"", "<div aria-hidden=\"true\"></div><nav class=\"quiet-index\"").replace("<div aria-hidden=\"true\"></div>", "<div aria-hidden=\"true\">")),
        ];
        for (what, html) in cases {
            assert!(check_destinations(&html).is_err(), "must reject: {what}");
        }
    }

    #[test]
    fn header_links_x_and_the_contact_page() {
        let html = render_navbar().into_string();
        assert!(html.contains("class=\"aien-x-link\" href=\"https://x.com/mdrakestapleton\""), "header must link X");
        assert!(html.contains(">Follow me on X<") && html.contains(">@mdrakestapleton<"));
        assert!(html.contains("href=\"/contact/\""), "header must link /contact/");
        assert!(!html.contains("<script"), "the X link is a plain link");
    }

    #[test]
    fn navbar_has_no_em_or_en_dash() {
        let html = render_navbar().into_string();
        assert!(!html.contains('\u{2014}') && !html.contains('\u{2013}'));
    }

    #[test]
    fn every_listed_language_has_its_pages_and_a_menu_link() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let html = render_navbar().into_string();
        assert!(html.contains(r#"hreflang="en""#), "menu must link back to English");
        for (code, name) in LANGUAGES {
            assert!(
                root.join("public").join(code).join("index.html").exists(),
                "language {code} is listed but public/{code}/index.html is missing"
            );
            assert!(html.contains(&format!(r#"href="/{code}/" hreflang="{code}""#)), "menu link for {code} missing");
            assert!(html.contains(name), "menu name for {code} missing");
        }
    }
}
