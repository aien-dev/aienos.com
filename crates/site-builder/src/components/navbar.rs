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

/// Secondary links: behind the "More" menu button at the end of the row.
pub const NAV_SECONDARY: [(&str, &str); 11] = [
    ("Community", "/community/"),
    ("Sovereign Intelligence", "/alpha/"),
    ("Lab", "/lab/"),
    ("Start here", "/start/"),
    ("Omega", "/omega/"),
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

/// The exact anchor markup for one primary nav link. Hand-written pages
/// copy this markup byte for byte so the tests can find it.
pub fn primary_link_markup(label: &str, href: &str) -> String {
    format!("<a class=\"aien-nav-link\" href=\"{href}\">{label}</a>")
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
                nav class="aien-nav aien-nav-bar" aria-label="Primary navigation" {
                    ul class="aien-nav-primary" {
                        @for (label, href) in NAV_PRIMARY {
                            li { a class="aien-nav-link" href=(href) { (label) } }
                        }
                    }
                    // One button holds everything that used to wrap onto
                    // further rows. A native <details>: it opens and closes
                    // by click, Enter or Space and needs no script. Below
                    // the narrow breakpoint the six sections move in here
                    // too (aien-nav-sections), so the header stays one row.
                    details class="aien-nav-more" {
                        summary class="aien-nav-more-button" {
                            span class="aien-nav-more-wide" { "More" }
                            span class="aien-nav-more-narrow" { "Menu" }
                        }
                        div class="aien-nav-panel" {
                            ul class="aien-nav-sections" aria-label="Sections" {
                                @for (label, href) in NAV_PRIMARY {
                                    li { a class="aien-nav-minor" href=(href) { (label) } }
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
                        }
                    }
                    (crate::components::contact::nav_reach(false))
                }
            }
        }
    }
}

/// Folds an older wrapping header nav (the translated /zh-CN/ pages carry
/// their own copy) into the one-row shape: the six sections stay in the
/// row, every later list moves behind the More button, and a copy of the
/// sections joins them for narrow screens. Idempotent; pages without the
/// shared nav pass through unchanged. Runs before contact::enrich, so the
/// X button lands after the button, as on generated pages.
pub fn fold_legacy(page: String, zh: bool) -> String {
    let Some(nav) = page.find("class=\"aien-nav\"") else { return page };
    let Some(header_end) = page.find("</header>") else { return page };
    if nav > header_end {
        return page;
    }
    let Some(open_end) = page[nav..].find('>').map(|i| nav + i + 1) else { return page };
    let primary_close = "</ul>";
    let Some(primary_end) = page[open_end..].find(primary_close).map(|i| open_end + i + primary_close.len()) else { return page };
    let Some(nav_end) = page[primary_end..].find("</nav>").map(|i| primary_end + i) else { return page };
    let primary = &page[open_end..primary_end];
    if !primary.trim_start().starts_with("<ul class=\"aien-nav-primary\"") {
        return page;
    }
    let rest = &page[primary_end..nav_end];
    let sections = primary
        .replacen("<ul class=\"aien-nav-primary\">", "<ul class=\"aien-nav-sections\">", 1)
        .replace("class=\"aien-nav-link\"", "class=\"aien-nav-minor\"");
    let (wide, narrow) = if zh { ("更多", "菜单") } else { ("More", "Menu") };
    let mut out = String::with_capacity(page.len() + sections.len() + 256);
    out.push_str(&page[..nav]);
    out.push_str("class=\"aien-nav aien-nav-bar\"");
    out.push_str(&page[nav + "class=\"aien-nav\"".len()..primary_end]);
    out.push_str(&format!(
        "<details class=\"aien-nav-more\"><summary class=\"aien-nav-more-button\"><span class=\"aien-nav-more-wide\">{wide}</span><span class=\"aien-nav-more-narrow\">{narrow}</span></summary><div class=\"aien-nav-panel\">{sections}{rest}</div></details>"
    ));
    out.push_str(&page[nav_end..]);
    out
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
    const HAND_WRITTEN: [&str; 4] = [
        "public/turing/index.html",
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
            assert!(page.contains("href=\"/alpha/\""), "{rel}: missing alpha testing entry");
            assert_six_in_order(&page, rel);
        }
    }

    #[test]
    fn header_contact_is_one_x_button_and_the_rest_sits_behind_more() {
        let html = render_navbar().into_string();
        assert!(
            html.contains("class=\"aien-x-link aien-x-button\" href=\"https://x.com/mdrakestapleton\""),
            "header must carry the X contact button"
        );
        assert!(html.contains("aria-label=\"Contact me on X (@mdrakestapleton)\""), "the X button needs a name");
        assert!(!html.contains("href=\"/contact/\""), "Contact and X are one button");
        assert!(!html.contains("<script"), "the X link is a plain link and the menu is native");
        let more = html.find("<details class=\"aien-nav-more\">").expect("one More button");
        assert_eq!(html.matches("<details").count(), 1);
        for (label, href) in NAV_SECONDARY {
            assert!(html[more..].contains(&format!("href=\"{href}\">{label}<")), "{label} belongs behind More");
        }
        let reach = html.find("aien-nav-reach").expect("X button group");
        assert!(reach > html.find("</details>").unwrap(), "X button follows the More button");
    }

    #[test]
    fn fold_legacy_moves_a_translated_wrapping_nav_into_one_row() {
        let page = "<header class=\"aien-header\"><nav aria-label=\"主要导航\" class=\"aien-nav\"><ul class=\"aien-nav-primary\"><li><a class=\"aien-nav-link\" href=\"/zh-CN/machine/\">机器</a></li></ul><ul class=\"aien-nav-secondary\"><li><a class=\"aien-nav-minor\" href=\"/zh-CN/blog\">博客</a></li></ul></nav></header><main><nav class=\"aien-nav\"></nav></main>".to_string();
        let out = fold_legacy(page, true);
        assert!(out.contains("class=\"aien-nav aien-nav-bar\""));
        assert!(out.contains("<span class=\"aien-nav-more-wide\">更多</span><span class=\"aien-nav-more-narrow\">菜单</span>"));
        let more = out.find("<details").unwrap();
        assert!(out[..more].contains("class=\"aien-nav-link\" href=\"/zh-CN/machine/\""), "sections stay in the row");
        assert!(out[more..].contains("<ul class=\"aien-nav-sections\"><li><a class=\"aien-nav-minor\" href=\"/zh-CN/machine/\""), "narrow copy of the sections");
        assert!(out[more..out.find("</details>").unwrap()].contains("/zh-CN/blog"), "later lists sit behind the button");
        assert!(out.contains("</details></nav></header>"));
        assert!(out.ends_with("<main><nav class=\"aien-nav\"></nav></main>"), "only the header nav is folded");
        assert_eq!(fold_legacy(out.clone(), true), out, "idempotent");
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
