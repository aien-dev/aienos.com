//! Where Drake is and how to reach him: the /contact/ page, the "Follow me
//! on X" link in every header, and the contact line in every footer.
//!
//! Generated pages get the header and footer pieces from the navbar and the
//! footer components. Hand-written and translated pages under public/ carry
//! their own copy of the header, so [`enrich`] adds the same markup to them
//! in the shared document pass, and [`verify`] fails the build if any page
//! in dist lacks either piece. The X link is a plain link: no widget, no
//! script and nothing loaded from X.

use std::fs;
use std::path::Path;

use maud::{html, Markup, PreEscaped};

use crate::seo::{X_HANDLE, X_URL};

pub const TITLE: &str = "Where I am and how to reach me | AienOS";
pub const DESCRIPTION: &str = "How to reach Drake Stapleton: X @mdrakestapleton, email drake@aienos.com, Aien (who can pass a message to me or put you through when I'm online) and the aien-dev repositories on GitHub.";
pub const URL: &str = "https://www.aienos.com/contact/";
pub const PATH: &str = "/contact/";
pub const EMAIL: &str = "drake@aienos.com";
pub const ALLEN: &str = "/aien/";
pub const GITHUB: &str = "https://github.com/aien-dev";
/// The front door: install command, releases and a hub of every part.
pub const SI: &str = "https://github.com/aien-dev/sovereign-intelligence";

/// The X mark (Simple Icons, CC0), drawn in the link's text colour.
const X_MARK: &str = r#"<svg class="aien-x-mark" width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false"><path d="M18.901 1.153h3.68l-8.04 9.19L24 22.846h-7.406l-5.8-7.584-6.638 7.584H.474l8.6-9.83L0 1.154h7.594l5.243 6.932ZM17.61 20.644h2.039L6.486 3.24H4.298Z"/></svg>"#;

/// The words of the header and footer pieces, in English or the
/// Simplified Chinese of the /zh-CN/ mirror.
struct Words {
    follow: &'static str,
    contact: &'static str,
    nav_label: &'static str,
    reach: &'static str,
    email: &'static str,
    allen: &'static str,
}

const EN: Words = Words {
    follow: "Follow me on X",
    contact: "Contact",
    nav_label: "Reach me",
    reach: "Where I am and how to reach me",
    email: "Email",
    allen: "Talk to Aien",
};

const ZH: Words = Words {
    follow: "在 X 上关注我",
    contact: "联系我",
    nav_label: "联系我",
    reach: "我在哪里，如何联系我",
    email: "邮箱",
    allen: "和 Aien 聊聊",
};

fn words(zh: bool) -> &'static Words {
    if zh { &ZH } else { &EN }
}

fn x_link(w: &Words) -> Markup {
    html! {
        a class="aien-x-link" href=(X_URL) target="_blank" rel="me noopener noreferrer" {
            (PreEscaped(X_MARK))
            span class="aien-x-label" { (w.follow) }
            " "
            span class="aien-x-handle" translate="no" { (X_HANDLE) }
        }
    }
}

/// The last group of the header nav: the contact page and the X link.
pub fn nav_reach(zh: bool) -> Markup {
    let w = words(zh);
    html! {
        ul class="aien-nav-secondary aien-nav-reach" aria-label=(w.nav_label) {
            li { a class="aien-nav-minor" href=(PATH) { (w.contact) } }
            li { (x_link(w)) }
        }
    }
}

/// The contact line in the footer: the contact page, then every route.
pub fn footer_reach(zh: bool) -> Markup {
    let w = words(zh);
    html! {
        div class="aien-reach-foot" {
            p class="aien-reach-title" { a href=(PATH) { (w.reach) } }
            p class="aien-reach-routes" {
                (x_link(w))
                " · "
                (w.email) ": " a href=(format!("mailto:{EMAIL}")) { (EMAIL) }
                " · "
                a href=(ALLEN) { (w.allen) }
                " · "
                a href=(GITHUB) target="_blank" rel="noopener noreferrer" { "GitHub: aien-dev" }
            }
        }
    }
}

/// Adds the header and footer pieces to a page that does not carry them
/// yet (hand-written and translated pages). Idempotent. Panics on a page
/// without the shared header nav: every page must show the X link.
pub fn enrich(mut page: String, zh: bool) -> String {
    if !page.contains("aien-nav-reach") {
        let nav = page.find("class=\"aien-nav\"").expect("page carries the shared header nav");
        let end = nav + page[nav..].find("</nav>").expect("shared header nav closes");
        page.insert_str(end, &nav_reach(zh).into_string());
    }
    if !page.contains("aien-reach-foot") {
        let strip = html! { div class="aien-reach-strip" { (footer_reach(zh)) } }.into_string();
        let end = if page.contains("class=\"aien-footer\"") { page.rfind("</footer>") } else { None }
            .or_else(|| page.rfind("</body>"))
            .expect("page has a body");
        page.insert_str(end, &strip);
    }
    page
}

/// Fails the build unless every HTML page in dist shows the X link in its
/// header and the contact line in its footer.
pub fn verify(dist: &Path) {
    fn walk(dir: &Path, n: &mut usize) {
        for entry in fs::read_dir(dir).expect("read dist directory") {
            let path = entry.expect("dist entry").path();
            if path.is_dir() {
                walk(&path, n);
            } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
                let html = fs::read_to_string(&path).expect("read page");
                check_page(&html).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                *n += 1;
            }
        }
    }
    let mut n = 0;
    walk(dist, &mut n);
    println!("  Follow me on X and the contact line: {n} pages");
}

fn check_page(html: &str) -> Result<(), String> {
    let x_href = format!("class=\"aien-x-link\" href=\"{X_URL}\"");
    let header = html.find("class=\"aien-header\"").ok_or("no shared header")?;
    let header_end = header + html[header..].find("</header>").ok_or("shared header does not close")?;
    let reach = html.find("class=\"aien-nav-secondary aien-nav-reach\"").ok_or("header lacks the reach group")?;
    if reach < header || reach > header_end {
        return Err("the reach group must sit in the header".into());
    }
    if !html[reach..header_end].contains(&x_href) || !html[reach..header_end].contains(&format!("href=\"{PATH}\"")) {
        return Err("header lacks the X link or the contact page".into());
    }
    let foot = html.rfind("class=\"aien-reach-foot\"").ok_or("footer lacks the contact line")?;
    if foot < header_end || !html[foot..].contains(&x_href) || !html[foot..].contains(&format!("href=\"mailto:{EMAIL}\"")) {
        return Err("footer contact line lacks the X link or the email".into());
    }
    Ok(())
}

fn route(name: &str, link: Markup, text: Markup) -> Markup {
    html! {
        div class="contact-route" {
            dt { (name) }
            dd class="contact-route-link" { (link) }
            dd class="contact-route-text" { (text) }
        }
    }
}

pub fn render_contact() -> Markup {
    html! {
        section id="contact" class="contact-page" {
            div class="container" style="max-width: 820px;" {
                div class="badge badge-blue" style="margin-bottom: 12px;" { "CONTACT" }
                h1 { "Where I am and how to reach me" }
                p class="contact-lede" {
                    "I'm Drake Stapleton. I build AienOS alone, with my agents, and I do it in public. Here is every way to find me. Pick the one that suits you."
                }
                dl class="contact-routes" {
                    (route("X", html! {
                        a href=(X_URL) target="_blank" rel="me noopener noreferrer" { (PreEscaped(X_MARK)) " " (X_HANDLE) }
                    }, html! {
                        "Where I post the work as it happens. Follow me, reply or mention me."
                    }))
                    (route("Email", html! {
                        a href=(format!("mailto:{EMAIL}")) { (EMAIL) }
                    }, html! {
                        "For anything longer than a post, or anything private."
                    }))
                    (route("Talk to Aien", html! {
                        a href=(ALLEN) { "aienos.com/aien/" }
                    }, html! {
                        "Aien is my AI. Aien can pass a message to me, or put you through to me when I'm online."
                    }))
                    (route("GitHub", html! {
                        a href=(GITHUB) target="_blank" rel="noopener noreferrer" { "github.com/aien-dev" }
                    }, html! {
                        "My code is on GitHub. Open an issue or a pull request on any public repository."
                    }))
                    (route("Sovereign Intelligence", html! {
                        a href=(SI) target="_blank" rel="noopener noreferrer" { "github.com/aien-dev/sovereign-intelligence" }
                    }, html! {
                        "The front door to everything I build: the install command, the releases and a link to every part. Sovereign Intelligence, built on AienOS."
                    }))
                }
                p class="contact-note" {
                    "Want to see what I'm doing right now? "
                    a href="/hive/" { "Watch me and my agents work in Hive" }
                    ", or read "
                    a href="/now/" { "the live ledger" }
                    "."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAND_WRITTEN: &str = "<html><head></head><body><header class=\"aien-header\"><div class=\"aien-header-inner\"><nav aria-label=\"x\" class=\"aien-nav\"><ul class=\"aien-nav-primary\"></ul></nav></div></header><main>text</main><footer>own footer</footer></body></html>";

    #[test]
    fn enrich_adds_the_header_link_and_footer_line_once() {
        let once = enrich(HAND_WRITTEN.to_string(), false);
        check_page(&once).expect("enriched page passes the check");
        assert_eq!(enrich(once.clone(), false), once, "enrich must be idempotent");
        assert!(once.find("aien-nav-reach").unwrap() < once.find("</nav>").unwrap(), "reach group sits inside the nav");
        assert!(once.find("aien-reach-strip").unwrap() > once.find("own footer").unwrap(), "footer line follows the page");
        assert!(once.contains("Follow me on X"));
    }

    #[test]
    fn translated_pages_get_chinese_words_and_the_same_links() {
        let zh = enrich(HAND_WRITTEN.replace("<footer>own footer</footer>", "<footer class=\"aien-footer\"><div>x</div></footer>"), true);
        check_page(&zh).expect("zh page passes the check");
        assert!(zh.contains("在 X 上关注我"));
        assert!(zh.find("aien-reach-strip").unwrap() < zh.rfind("</footer>").unwrap(), "inside the translated footer");
    }

    #[test]
    fn a_page_without_the_link_fails_the_check() {
        assert!(check_page(HAND_WRITTEN).is_err());
        let footer_only = enrich(HAND_WRITTEN.to_string(), false).replace("aien-nav-reach", "gone");
        assert!(check_page(&footer_only).is_err());
    }

    #[test]
    fn the_page_lists_every_route() {
        let html = render_contact().into_string();
        for needle in [
            "<h1>Where I am and how to reach me</h1>",
            "href=\"https://x.com/mdrakestapleton\"",
            "@mdrakestapleton",
            "href=\"mailto:drake@aienos.com\"",
            "href=\"/aien/\"",
            "href=\"https://github.com/aien-dev\"",
            "href=\"https://github.com/aien-dev/sovereign-intelligence\"",
        ] {
            assert!(html.contains(needle), "contact page lacks {needle}");
        }
        assert!(!html.contains('\u{2014}') && !html.contains('\u{2013}'));
        assert!(!html.contains("<script"), "the X link is a plain link");
    }
}
