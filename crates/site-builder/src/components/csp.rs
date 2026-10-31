//! Page-level Content-Security-Policy. GitHub Pages cannot set response
//! headers, so every emitted HTML page carries the policy as the first
//! element of its head: a `<meta http-equiv>` governs only what follows it.
//!
//! Scripts run from first-party files only. The single outside script host is
//! the consent-gated X pixel loaded by `/js/allen-intro.js`; its requests go
//! to the analytics hosts in `img-src` and `connect-src`. `style-src` keeps
//! `'unsafe-inline'` because every page carries `style` attributes (the
//! shared layout sets them on `#root` and `main`) and hand-written pages
//! carry `<style>` blocks; inline styles cannot run code. Additions are
//! page-scoped by what the page actually loads:
//! - `WASM_SCRIPTS` compile WebAssembly in the page, so their pages get
//!   `'wasm-unsafe-eval'` (WebAssembly compilation only, never JavaScript
//!   `eval`).
//! - `BLOB_WORKER_SCRIPTS` start a worker from a `blob:` URL, so their pages
//!   get `worker-src blob:`; a blob worker inherits the page policy.
//! - Pages that embed YouTube videos may frame `www.youtube.com`.
//!
//! `verify` checks both lists against the scripts themselves, in both
//! directions, so a grant cannot go missing or outlive its need.
//!
//! A frame showing `srcdoc` inherits its parent's policy, so programs Aien
//! writes, which carry inline scripts, cannot run in one. They run in
//! `RUNNER_PATH`, the one document whose policy allows inline scripts: its
//! only script, `RUNNER_SCRIPT`, writes what the parent posts, and only while
//! the frame is sandboxed to an opaque origin. The edge-rendered
//! `/hive/board/` is not built here and keeps its own stricter header policy.

use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const X_HOSTS: &str = "https://analytics.twitter.com https://analytics.x.com https://t.co";
const WASM_SCRIPTS: &[&str] = &["/js/hive.js", "/js/allen.js", "/js/allen-scene.js"];
const BLOB_WORKER_SCRIPTS: &[&str] = &["/js/allen.js"];
const YOUTUBE_EMBED: &str = "src=\"https://www.youtube.com/embed/";
const META_OPEN: &str = "<meta http-equiv=\"Content-Security-Policy\" content=\"";

/// The program runner, relative to dist, and its script.
pub const RUNNER_PATH: &str = "aien/run.html";
pub const RUNNER_SCRIPT: &str = "/js/aien-run.js";
/// The runner's policy. The program's own policy, which `sandboxed()` in the
/// Aien scripts writes into its head, narrows this further.
const RUNNER_POLICY: &str = "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; \
                             img-src 'self' data:; form-action 'none'; base-uri 'none'";

fn loads(html: &str, script: &str) -> bool {
    html.contains(&format!("src=\"{script}\""))
}

/// The policy for one page, derived from what the page loads.
pub fn policy_for(html: &str) -> String {
    if loads(html, RUNNER_SCRIPT) {
        return RUNNER_POLICY.into();
    }
    let wasm = if WASM_SCRIPTS.iter().any(|s| loads(html, s)) { " 'wasm-unsafe-eval'" } else { "" };
    let mut p = format!(
        "default-src 'self'; script-src 'self' https://static.ads-twitter.com{wasm}; style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: {X_HOSTS}; connect-src 'self' {X_HOSTS}; font-src 'self'; object-src 'none'; \
         base-uri 'self'; form-action 'self'"
    );
    if BLOB_WORKER_SCRIPTS.iter().any(|s| loads(html, s)) {
        p.push_str("; worker-src blob:");
    }
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
/// A `javascript:` URL where a URL starts: after `=` (attributes, and `url=`
/// in a refresh) or `(` (CSS `url(`), past optional quotes and the spaces
/// and controls a browser trims. Prose such as "needs JavaScript: it" is
/// not a URL.
static JS_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[=(]["'\x00-\x20]*javascript:"#).unwrap());
static PAGE_SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"src="(/js/[A-Za-z0-9._-]+\.js)""#).unwrap());
static COMPILES_WASM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"WebAssembly\.(compile|compileStreaming|instantiate|instantiateStreaming|validate|Module)\b").unwrap());

/// Lowercased markup with character references decoded (`&colon;`, `&tab;`,
/// `&newline;` and numeric ones, with or without `;`) and the tabs and line
/// breaks a URL parser drops removed, so `java&#x09;script&colon;` reads as
/// `javascript:`.
fn url_text(markup: &str) -> String {
    let lower = markup.to_ascii_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut rest = lower.as_str();
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let (decoded, used) = reference(rest);
        match decoded {
            Some(c) => out.push(c),
            None => out.push('&'),
        }
        rest = &rest[used..];
    }
    out.push_str(rest);
    out.retain(|c| !matches!(c, '\t' | '\n' | '\r'));
    out
}

/// One character reference at the start of `s` (which begins with `&`):
/// the character and the bytes consumed, or `None` and 1.
fn reference(s: &str) -> (Option<char>, usize) {
    for (name, c) in [("&colon;", ':'), ("&tab;", '\t'), ("&newline;", '\n'), ("&lpar;", '(')] {
        if s.starts_with(name) {
            return (Some(c), name.len());
        }
    }
    let Some(num) = s.strip_prefix("&#") else { return (None, 1) };
    let (hex, digits) = match num.strip_prefix('x') {
        Some(h) => (true, h),
        None => (false, num),
    };
    let len = digits.bytes().take_while(|b| if hex { b.is_ascii_hexdigit() } else { b.is_ascii_digit() }).count();
    if len == 0 {
        return (None, 1);
    }
    let value = u32::from_str_radix(&digits[..len], if hex { 16 } else { 10 }).ok().and_then(char::from_u32);
    let mut used = 2 + usize::from(hex) + len;
    if s[used..].starts_with(';') {
        used += 1;
    }
    (Some(value.unwrap_or('\u{fffd}')), used)
}

/// One page: exactly one policy, equal to `policy_for`, ahead of anything it
/// governs; no inline script other than JSON-LD data; no inline event
/// handler; no `javascript:` URL. The runner loads its script and nothing
/// else.
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
    if want == RUNNER_POLICY && (lower.matches("<script").count() != 1 || lower.contains("<iframe") || lower.contains("<object") || lower.contains("<embed")) {
        return Err("the runner loads its own script and nothing else that runs".into());
    }
    let markup = INERT.replace_all(html, "");
    if let Some(m) = HANDLER.find(&markup) {
        return Err(format!("inline event handler: {}", m.as_str().chars().take(80).collect::<String>()));
    }
    if JS_URL.is_match(&url_text(&markup)) {
        return Err("javascript: URL".into());
    }
    Ok(())
}

/// What one first-party script needs beyond the base policy, read from the
/// script itself: (compiles WebAssembly, starts a blob: worker).
fn script_needs(js: &str) -> (bool, bool) {
    (COMPILES_WASM.is_match(js), js.contains("new Worker(") && js.contains("URL.createObjectURL("))
}

/// Every HTML file in the build carries the policy and needs nothing it
/// forbids; only `RUNNER_PATH` loads the runner; the page-scoped grants match
/// what the scripts the pages load actually do. All failures are reported
/// together.
pub fn verify(dist: &Path) {
    fn walk(dist: &Path, dir: &Path, n: &mut usize, bad: &mut Vec<String>, scripts: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).expect("read dist directory") {
            let path = entry.expect("dist entry").path();
            if path.is_dir() {
                walk(dist, &path, n, bad, scripts);
            } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
                let html = fs::read_to_string(&path).expect("read page");
                if let Err(e) = check_page(&html) {
                    bad.push(format!("{}: {e}", path.display()));
                }
                let rel = path.strip_prefix(dist).expect("under dist").to_string_lossy().replace('\\', "/");
                if loads(&html, RUNNER_SCRIPT) != (rel == RUNNER_PATH) {
                    bad.push(format!("{}: only {RUNNER_PATH} loads {RUNNER_SCRIPT}, and it must", path.display()));
                }
                scripts.extend(PAGE_SCRIPT.captures_iter(&html).map(|c| c[1].to_string()));
                *n += 1;
            }
        }
    }
    let (mut n, mut bad, mut scripts) = (0, Vec::new(), BTreeSet::new());
    walk(dist, dist, &mut n, &mut bad, &mut scripts);
    assert!(n > 0, "csp: no HTML pages found under {}", dist.display());
    if !dist.join(RUNNER_PATH).is_file() {
        bad.push(format!("{}: missing; Aien's programs run there", dist.join(RUNNER_PATH).display()));
    }
    for script in &scripts {
        let file = dist.join(&script[1..]);
        let Ok(js) = fs::read_to_string(&file) else {
            bad.push(format!("{}: a page loads it, but it is not in the build", file.display()));
            continue;
        };
        let (wasm, blob_worker) = script_needs(&js);
        if wasm != WASM_SCRIPTS.contains(&script.as_str()) {
            bad.push(format!("{script}: compiles WebAssembly is {wasm}, but WASM_SCRIPTS says otherwise"));
        }
        if blob_worker != BLOB_WORKER_SCRIPTS.contains(&script.as_str()) {
            bad.push(format!("{script}: starts a blob: worker is {blob_worker}, but BLOB_WORKER_SCRIPTS says otherwise"));
        }
    }
    for listed in WASM_SCRIPTS.iter().chain(BLOB_WORKER_SCRIPTS) {
        if !scripts.contains(*listed) {
            bad.push(format!("{listed}: granted a policy addition, but no page loads it"));
        }
    }
    bad.sort();
    bad.dedup();
    assert!(bad.is_empty(), "csp: {} problems in {n} pages:\n{}", bad.len(), bad.join("\n"));
    println!("  Content-Security-Policy meta: {n} pages, {} first-party scripts checked", scripts.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>t</title><link rel=\"stylesheet\" href=\"/a.css\"><script defer src=\"/js/allen-intro.js\"></script><script type=\"application/ld+json\">{\"a\":1}</script></head><body><p style=\"color:red\">x</p></body></html>";
    const RUNNER: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>r</title><link rel=\"stylesheet\" href=\"/a.css\"><script src=\"/js/aien-run.js\"></script></head><body><p>x</p></body></html>";

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
        assert!(!p.contains("frame-src") && !p.contains("wasm") && !p.contains("worker-src"), "no page-scoped additions on a plain page");
        let script_src = p.split(';').find(|d| d.trim_start().starts_with("script-src")).unwrap();
        assert!(!script_src.contains("unsafe"), "scripts never get unsafe-inline or unsafe-eval");
    }

    #[test]
    fn additions_follow_what_the_page_loads() {
        let hive = enrich(PAGE.replace("</head>", "<script defer src=\"/js/hive.js\"></script></head>"));
        assert!(policy_for(&hive).contains("script-src 'self' https://static.ads-twitter.com 'wasm-unsafe-eval';"));
        assert!(!policy_for(&hive).contains("worker-src"));
        let aien = enrich(PAGE.replace("</head>", "<script defer src=\"/js/allen.js\"></script></head>"));
        assert!(policy_for(&aien).contains("'wasm-unsafe-eval';"));
        assert!(policy_for(&aien).ends_with("; worker-src blob:"));
        let tube = enrich(PAGE.replace("</body>", "<iframe src=\"https://www.youtube.com/embed/x\"></iframe></body>"));
        assert!(policy_for(&tube).ends_with("; frame-src https://www.youtube.com"));
        for page in [&hive, &aien, &tube] {
            check_page(page).unwrap();
            let script_src = policy_for(page);
            let script_src = script_src.split(';').find(|d| d.trim_start().starts_with("script-src")).unwrap();
            assert!(script_src.split_whitespace().all(|t| t != "'unsafe-inline'" && t != "'unsafe-eval'"), "no page script gets inline or eval");
        }
    }

    #[test]
    fn the_runner_alone_allows_inline_scripts() {
        let runner = enrich(RUNNER.to_string());
        check_page(&runner).unwrap();
        assert_eq!(policy_for(&runner), RUNNER_POLICY);
        for d in ["default-src 'none'", "form-action 'none'", "base-uri 'none'"] {
            assert!(RUNNER_POLICY.contains(d), "runner policy lacks {d}");
        }
        for bad in [
            runner.replace("</head>", "<script src=\"/js/allen-intro.js\"></script></head>"),
            runner.replace("</body>", "<iframe src=\"/x/\"></iframe></body>"),
            runner.replace("</body>", "<script>x()</script></body>"),
        ] {
            assert!(check_page(&bad).is_err(), "the runner must load nothing else: {bad}");
        }
    }

    #[test]
    fn script_needs_read_the_script() {
        assert_eq!(script_needs("WebAssembly.instantiate(bytes)"), (true, false));
        assert_eq!(script_needs("const w = new Worker(URL.createObjectURL(blob));"), (false, true));
        assert_eq!(script_needs("new Worker('/js/lab-worker.js')"), (false, false));
        assert_eq!(script_needs("WebAssembly.RuntimeError"), (false, false));
    }

    #[test]
    fn negative_controls_are_refused() {
        let good = enrich(PAGE.to_string());
        let bad = [
            ("no meta", PAGE.to_string()),
            ("two metas", good.replacen("<title>", &format!("{META_OPEN}x\"><title>"), 1)),
            ("wrong policy", good.replace("object-src 'none'", "object-src *")),
            ("runner policy on a page", good.replace(&policy_for(&good), RUNNER_POLICY)),
            ("inline script", good.replace("</body>", "<script>alert(1)</script></body>")),
            ("inline module", good.replace("</body>", "<script type=\"module\">x()</script></body>")),
            ("src with body", good.replace("allen-intro.js\"></script>", "allen-intro.js\">x()</script>")),
            ("inline handler", good.replace("<p style", "<p onclick=\"x()\" style")),
            ("upper handler", good.replace("<p style", "<P ONLOAD='x()' style")),
            ("javascript url", good.replace("</body>", "<a href=\"javascript:x()\">a</a></body>")),
            ("unquoted javascript url", good.replace("</body>", "<a href=JavaScript:x()>a</a></body>")),
            ("spaced javascript url", good.replace("</body>", "<a href=\" \tjavascript:x()\">a</a></body>")),
            ("tab in javascript url", good.replace("</body>", "<a href=\"java&#x09;script:x()\">a</a></body>")),
            ("named colon", good.replace("</body>", "<a href=\"javascript&colon;x()\">a</a></body>")),
            ("decimal colon", good.replace("</body>", "<a href=\"javascript&#58x()\">a</a></body>")),
            ("refresh to javascript", good.replace("</head>", "<meta http-equiv=\"refresh\" content=\"0; url=javascript:x()\"></head>")),
            ("css javascript url", good.replace("<p style=\"color:red\"", "<p style=\"background:url(javascript:x())\"")),
            ("meta after script", PAGE.replace("<title>", "<script src=\"/x.js\"></script><title>").replace("<title>", &format!("{META_OPEN}{}\"><title>", policy_for(PAGE)))),
        ];
        for (what, page) in bad {
            assert!(check_page(&page).is_err(), "must refuse: {what}");
        }
        // JSON-LD and text that merely mentions handlers or JavaScript stay allowed.
        let fine = good.replace(
            "</body>",
            "<p>use onclick= in text</p><noscript><p>Aien needs JavaScript: he lives in the page.</p></noscript><script type=\"application/ld+json\">{\"onload\":\"x\"}</script></body>",
        );
        check_page(&fine).unwrap();
    }
}
