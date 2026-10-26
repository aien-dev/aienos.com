//! Page-level Content-Security-Policy. GitHub Pages cannot set response
//! headers, so every emitted HTML page carries the policy as the first
//! element of its head: a `<meta http-equiv>` governs only what follows it.
//!
//! Scripts run from first-party files only. The single outside script host is
//! the consent-gated X pixel loaded by `/js/allen-intro.js`; its requests go
//! to the analytics hosts in `img-src` and `connect-src`. `style-src` keeps
//! `'unsafe-inline'` because every page carries `style` attributes (the
//! shared layout sets them on `#root` and `main`) and hand-written pages
//! carry `<style>` blocks; inline styles cannot run code. Two additions are
//! page-scoped by what the page actually loads: `/js/hive.js` compiles the
//! seeded freshness WASM (`'wasm-unsafe-eval'` allows WebAssembly compilation
//! only, never JavaScript `eval`), and pages that embed YouTube videos may
//! frame `www.youtube.com`. The edge-rendered `/hive/board/` is not built here
//! and keeps its own stricter header policy.

use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const X_HOSTS: &str = "https://analytics.twitter.com https://analytics.x.com https://t.co";
const WASM_LOADER: &str = "src=\"/js/hive.js\"";
const YOUTUBE_EMBED: &str = "src=\"https://www.youtube.com/embed/";
const META_OPEN: &str = "<meta http-equiv=\"Content-Security-Policy\" content=\"";

/// The policy for one page, derived from what the page loads.
pub fn policy_for(html: &str) -> String {
    let wasm = if html.contains(WASM_LOADER) { " 'wasm-unsafe-eval'" } else { "" };
    let mut p = format!(
        "default-src 'self'; script-src 'self' https://static.ads-twitter.com{wasm}; style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: {X_HOSTS}; connect-src 'self' {X_HOSTS}; font-src 'self'; object-src 'none'; \
         base-uri 'self'; form-action 'self'"
    );
    if html.contains(YOUTUBE_EMBED) {
        p.push_str("; frame-src https://www.youtube.com");
    }
    p
}

/// End of the opening `<head ...>` tag, or after a leading `<meta charset>`
/// so the charset declaration stays first.
fn insertion_point(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let head = lower.find("<head")?;
    let mut at = head + lower[head..].find('>')? + 1;
    let rest = &lower[at..];
    let lead = rest.len() - rest.trim_start().len();
    if rest.trim_start().starts_with("<meta charset") {
        at += lead + rest[lead..].find('>')? + 1;
    }
    Some(at)
}

/// Adds the policy to a finished page. Idempotent.
pub fn enrich(mut page: String) -> String {
    if page.contains(META_OPEN) {
        return page;
    }
    let at = insertion_point(&page).expect("csp: page has a head");
    let meta = format!("{META_OPEN}{}\">", policy_for(&page));
    page.insert_str(at, &meta);
    page
}

static HANDLER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)<[a-z][a-z0-9-]*\b[^>]*?\son[a-z]+\s*="#).unwrap());
static SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<script\b([^>]*)>(.*?)</script\s*>").unwrap());
/// Comments, style blocks and script bodies: text a handler scan must skip.
static INERT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<!--.*?-->|<style\b.*?</style\s*>|<script\b[^>]*>.*?</script\s*>").unwrap());

/// One page: exactly one policy, equal to `policy_for`, ahead of anything it
/// governs; no inline script other than JSON-LD data; no inline event
/// handler; no `javascript:` URL.
pub fn check_page(html: &str) -> Result<(), String> {
    if html.matches(META_OPEN).count() != 1 {
        return Err("needs exactly one Content-Security-Policy meta".into());
    }
    let start = html.find(META_OPEN).unwrap();
    let body = &html[start + META_OPEN.len()..];
    let got = &body[..body.find('"').ok_or("unterminated policy")?];
    let want = policy_for(html);
    if got != want {
        return Err(format!("policy {got:?} differs from {want:?}"));
    }
    let lower = html.to_ascii_lowercase();
    let head = lower.find("<head").ok_or("no head")?;
    if start < head || lower[..start].contains("<script") || lower[..start].contains("<link") || lower[..start].contains("<style") {
        return Err("the policy must precede every script, link and style".into());
    }
    for c in SCRIPT.captures_iter(html) {
        let attrs = c[1].to_ascii_lowercase();
        let ld = attrs.contains("type=\"application/ld+json\"");
        if !attrs.contains("src=") && !ld {
            return Err(format!("inline script: {}", c[2].trim().chars().take(60).collect::<String>()));
        }
        if attrs.contains("src=") && !c[2].trim().is_empty() {
            return Err("a script with src must have an empty body".into());
        }
    }
    let markup = INERT.replace_all(html, "");
    if let Some(m) = HANDLER.find(&markup) {
        return Err(format!("inline event handler: {}", m.as_str().chars().take(80).collect::<String>()));
    }
    if markup.to_ascii_lowercase().contains("javascript:") {
        return Err("javascript: URL".into());
    }
    Ok(())
}

/// Every HTML file in the build carries the policy and needs nothing it forbids.
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
    assert!(n > 0, "csp: no HTML pages found under {}", dist.display());
    println!("  Content-Security-Policy meta: {n} pages");
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>t</title><link rel=\"stylesheet\" href=\"/a.css\"><script defer src=\"/js/allen-intro.js\"></script><script type=\"application/ld+json\">{\"a\":1}</script></head><body><p style=\"color:red\">x</p></body></html>";

    #[test]
    fn enriched_page_passes_and_keeps_charset_first() {
        let page = enrich(PAGE.to_string());
        check_page(&page).unwrap();
        assert!(page.contains("<head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\""));
        assert_eq!(enrich(page.clone()), page, "enrich is idempotent");
        let p = policy_for(&page);
        for d in [
            "default-src 'self'",
            "script-src 'self' https://static.ads-twitter.com;",
            "object-src 'none'",
            "base-uri 'self'",
            "form-action 'self'",
            "img-src 'self' data: https://analytics.twitter.com https://analytics.x.com https://t.co",
            "connect-src 'self' https://analytics.twitter.com https://analytics.x.com https://t.co",
        ] {
            assert!(p.contains(d), "policy lacks {d}");
        }
        assert!(!p.contains("frame-src") && !p.contains("wasm"), "no page-scoped additions on a plain page");
        let script_src = p.split(';').find(|d| d.trim_start().starts_with("script-src")).unwrap();
        assert!(!script_src.contains("unsafe"), "scripts never get unsafe-inline or unsafe-eval");
    }

    #[test]
    fn additions_follow_what_the_page_loads() {
        let hive = enrich(PAGE.replace("</head>", "<script defer src=\"/js/hive.js\"></script></head>"));
        assert!(policy_for(&hive).contains("script-src 'self' https://static.ads-twitter.com 'wasm-unsafe-eval';"));
        let tube = enrich(PAGE.replace("</body>", "<iframe src=\"https://www.youtube.com/embed/x\"></iframe></body>"));
        assert!(policy_for(&tube).ends_with("; frame-src https://www.youtube.com"));
        check_page(&hive).unwrap();
        check_page(&tube).unwrap();
    }

    #[test]
    fn negative_controls_are_refused() {
        let good = enrich(PAGE.to_string());
        let bad = [
            ("no meta", PAGE.to_string()),
            ("two metas", good.replacen("<title>", &format!("{META_OPEN}x\"><title>"), 1)),
            ("wrong policy", good.replace("object-src 'none'", "object-src *")),
            ("inline script", good.replace("</body>", "<script>alert(1)</script></body>")),
            ("inline module", good.replace("</body>", "<script type=\"module\">x()</script></body>")),
            ("src with body", good.replace("allen-intro.js\"></script>", "allen-intro.js\">x()</script>")),
            ("inline handler", good.replace("<p style", "<p onclick=\"x()\" style")),
            ("upper handler", good.replace("<p style", "<P ONLOAD='x()' style")),
            ("javascript url", good.replace("</body>", "<a href=\"javascript:x()\">a</a></body>")),
            ("meta after script", PAGE.replace("<title>", "<script src=\"/x.js\"></script><title>").replace("<title>", &format!("{META_OPEN}{}\"><title>", policy_for(PAGE)))),
        ];
        for (what, page) in bad {
            assert!(check_page(&page).is_err(), "must refuse: {what}");
        }
        // JSON-LD and text that merely mentions handlers stay allowed.
        let fine = good.replace("</body>", "<p>use onclick= in text</p><script type=\"application/ld+json\">{\"onload\":\"x\"}</script></body>");
        check_page(&fine).unwrap();
    }
}
