//! Guards that run in `verify` mode on any generated tree, whatever built it.
//!
//! Before the Omega site generator, several gates ran only inside `build`:
//! the dist/registry comparison, the head checks and the validation of each
//! published data file. A tree built by another generator never met them.
//! This module holds those gates as pure checks over the files in dist, so
//! `site-builder verify DIR` is a complete, independent oracle.
//!
//! Independence: every expected value here comes from Rust constants and
//! Rust parsers ([`seo::PAGES`], [`seo::DATA_FILES`], the component
//! validators). Nothing is read from the generator's own registry
//! (`om/site/pages.json` is not an input), so a page the generator adds or
//! drops on its own is caught rather than trusted.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::seo::{self, attr, tag_name, DATA_FILES, HOST, PAGES};

/// The first opening tag at or after `from` whose class attribute holds
/// `token` as a whole word: (tag start, index just past its `>`, tag name).
pub fn find_class(html: &str, from: usize, token: &str) -> Option<(usize, usize, String)> {
    let mut i = from;
    while let Some(lt) = html.get(i..)?.find('<') {
        let at = i + lt;
        let gt = at + html[at..].find('>')?;
        let tag = &html[at..=gt];
        if !tag.starts_with("</") && !tag.starts_with("<!") {
            if let Some(class) = attr(tag, "class") {
                if class.split_ascii_whitespace().any(|c| c == token) {
                    return Some((at, gt + 1, tag_name(tag)));
                }
            }
        }
        i = gt + 1;
    }
    None
}

/// Index just past the close tag of the element whose opening tag ends at
/// `body`, counting nested elements of the same name.
pub fn element_end(html: &str, body: usize, name: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let (start, close) = (format!("<{name}"), format!("</{name}>"));
    let mut depth = 1usize;
    let mut i = body;
    while let Some(lt) = lower[i..].find('<') {
        let at = i + lt;
        let tail = &lower[at..];
        if tail.starts_with(&close) {
            depth -= 1;
            if depth == 0 {
                return Some(at + close.len());
            }
        } else if tail.starts_with(&start) && tail[start.len()..].starts_with(['>', ' ', '\t', '\n', '\r', '/']) {
            depth += 1;
        }
        i = at + 1;
    }
    None
}

/// Elements that have no close tag.
const VOID: [&str; 13] = ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];

/// The attribute names of one opening tag, lowercased, skipping quoted
/// values, so `title="hidden"` or `class="visually-hidden"` is no attribute
/// named `hidden`.
pub fn attr_names(tag: &str) -> Vec<String> {
    let b = tag.as_bytes();
    let ws = |c: u8| c.is_ascii_whitespace();
    let mut i = 1;
    while i < b.len() && !ws(b[i]) && b[i] != b'>' && b[i] != b'/' {
        i += 1;
    }
    let mut out = Vec::new();
    loop {
        while i < b.len() && (ws(b[i]) || b[i] == b'/') {
            i += 1;
        }
        if i >= b.len() || b[i] == b'>' {
            break;
        }
        let start = i;
        while i < b.len() && !ws(b[i]) && b[i] != b'=' && b[i] != b'>' && b[i] != b'/' {
            i += 1;
        }
        if i > start {
            out.push(tag[start..i].to_ascii_lowercase());
        }
        while i < b.len() && ws(b[i]) {
            i += 1;
        }
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && ws(b[i]) {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                i += 1;
                while i < b.len() && b[i] != q {
                    i += 1;
                }
                i += 1;
            } else {
                while i < b.len() && !ws(b[i]) && b[i] != b'>' {
                    i += 1;
                }
            }
        }
    }
    out
}

/// One pass over `html` that drops comments, the listed elements (through
/// their close) and, when `hidden` is set, every element whose opening tag
/// carries the `hidden` attribute or `aria-hidden="true"`. Anything that
/// never closes is dropped to the end, so malformed markup can only remove
/// content from the checks, never add it.
fn strip(html: &str, drop: &[&str], hidden: bool) -> String {
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while let Some(lt) = html[i..].find('<') {
        let at = i + lt;
        out.push_str(&html[i..at]);
        let rest = &html[at..];
        if rest.starts_with("<!--") {
            i = rest.find("-->").map_or(html.len(), |e| at + e + 3);
            out.push(' ');
            continue;
        }
        let name = tag_name(rest);
        let Some(gt) = rest.find('>').map(|g| at + g) else {
            i = html.len();
            break;
        };
        let tag = &html[at..=gt];
        let opening = !rest.starts_with("</") && !rest.starts_with("<!") && !name.is_empty();
        let hide = opening
            && (drop.contains(&name.as_str())
                || (hidden && (attr_names(tag).iter().any(|a| a == "hidden") || attr(tag, "aria-hidden").as_deref() == Some("true"))));
        if hide {
            i = if VOID.contains(&name.as_str()) || tag.ends_with("/>") {
                gt + 1
            } else {
                element_end(html, gt + 1, &name).unwrap_or(html.len())
            };
            out.push(' ');
            continue;
        }
        out.push_str(tag);
        i = gt + 1;
    }
    out.push_str(&html[i.min(html.len())..]);
    out
}

/// The real markup of a page: comments, scripts, styles and templates
/// removed, so an id or link written inside them is not an element.
pub fn markup(html: &str) -> String {
    strip(html, &["script", "style", "template"], false)
}

/// The part of a page a reader reaches with the page as served: the real
/// markup, minus `<noscript>` fallbacks and elements marked `hidden` or
/// `aria-hidden="true"`.
pub fn reachable(html: &str) -> String {
    strip(html, &["script", "style", "template", "noscript"], true)
}

/// The contents of every `<noscript>` element in the real markup outside
/// hidden elements: what a reader without scripts gets instead.
pub fn noscript(html: &str) -> Vec<String> {
    let m = strip(html, &["script", "style", "template"], true);
    let lower = m.to_ascii_lowercase();
    let mut out = Vec::new();
    for (at, _) in lower.match_indices("<noscript") {
        if !lower[at + 9..].starts_with(['>', ' ', '\t', '\n', '\r']) {
            continue;
        }
        let Some(body) = m[at..].find('>').map(|g| at + g + 1) else { continue };
        let end = element_end(&m, body, "noscript").map_or(m.len(), |e| e - "</noscript>".len());
        out.push(m[body..end].to_string());
    }
    out
}

/// Whether the real markup has an element with exactly this id.
pub fn has_id(html: &str, id: &str) -> bool {
    let m = markup(html);
    let mut i = 0;
    while let Some(lt) = m[i..].find('<') {
        let at = i + lt;
        let Some(gt) = m[at..].find('>').map(|g| at + g) else { break };
        if !m[at..].starts_with("</") && attr(&m[at..=gt], "id").as_deref() == Some(id) {
            return true;
        }
        i = gt + 1;
    }
    false
}

/// Text a reader sees: hidden elements and every tag removed, the common
/// entities decoded, whitespace squashed.
pub fn reader_text(fragment: &str) -> String {
    let s = strip(fragment, &["script", "style", "template"], true);
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out.replace("&nbsp;", " ").replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every `<a>` in `region`, as (href, reader text). An anchor without an
/// href has an empty one.
pub fn anchors(region: &str) -> Vec<(String, String)> {
    let lower = region.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(lt) = lower[i..].find("<a") {
        let at = i + lt;
        i = at + 2;
        if !lower[i..].starts_with(['>', ' ', '\t', '\n', '\r']) {
            continue;
        }
        let Some(gt) = region[at..].find('>').map(|g| at + g) else { break };
        let close = lower[gt..].find("</a>").map(|c| gt + c).unwrap_or(region.len());
        out.push((attr(&region[at..=gt], "href").unwrap_or_default(), reader_text(&region[gt + 1..close])));
        i = close;
    }
    out
}

/// Every `index.html` under dist, as site paths (`/`, `/machine/`,
/// `/zh-CN/turing/`), sorted. An unreadable directory is an error, never
/// a silently shorter list.
pub fn html_pages(dist: &Path) -> Result<Vec<String>, String> {
    fn walk(dir: &Path, dist: &Path, out: &mut Vec<String>) -> Result<(), String> {
        let rd = fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
        for entry in rd {
            let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
            let p = entry.path();
            let kind = entry.file_type().map_err(|e| format!("stat {}: {e}", p.display()))?;
            if kind.is_dir() {
                walk(&p, dist, out)?;
            } else if p.file_name().is_some_and(|n| n == "index.html") {
                let rel = p.strip_prefix(dist).map_err(|e| e.to_string())?.parent().unwrap_or(Path::new(""));
                let rel = rel.to_string_lossy().replace('\\', "/");
                out.push(if rel.is_empty() { "/".to_string() } else { format!("/{rel}/") });
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dist, dist, &mut out)?;
    out.sort();
    Ok(out)
}

fn is_zh(path: &str) -> bool {
    path.starts_with("/zh-CN/")
}

/// Translations whose English page is not at the same path. The /zh-CN/
/// front page translates the former front page, now /classic/; the root
/// (Allen's native door) has no translation of its own.
pub const TRANSLATED_ELSEWHERE: [(&str, &str); 1] = [("/zh-CN/", "/classic/")];

/// The English page a translated page translates (or `path` itself).
fn en_of(path: &str) -> String {
    if let Some((_, en)) = TRANSLATED_ELSEWHERE.iter().find(|(zh, _)| *zh == path) {
        return en.to_string();
    }
    if is_zh(path) { format!("/{}", &path["/zh-CN/".len()..]) } else { path.to_string() }
}

/// Where the translation of English page `path` lives, if it may have one.
fn zh_of(path: &str) -> Option<String> {
    if let Some((zh, _)) = TRANSLATED_ELSEWHERE.iter().find(|(_, en)| *en == path) {
        return Some(zh.to_string());
    }
    if path == "/" {
        return None;
    }
    Some(format!("/zh-CN{path}"))
}

/// The written translation partner of `path`, if any.
fn twin_of(path: &str, written: &BTreeSet<&str>) -> Option<String> {
    let other = if is_zh(path) { Some(en_of(path)) } else { zh_of(path) };
    other.filter(|o| written.contains(o.as_str()))
}

/// The engineer record (the former drakestapleton.com, now /engineer/),
/// compiled in from its committed source so a generated tree cannot add
/// pages to it.
pub const ENGINEER_RAW: &str = include_str!("../../../public/data/engineer.json");

/// The paths of the engineer record after validating every entry: a
/// lowercase path under /engineer/ made of `a-z 0-9 -` segments, listed
/// once; a name, title and description; a known kind, exactly one `home` at
/// /engineer/ itself; old paths that are root-relative.
pub fn engineer_paths(raw: &str) -> Result<Vec<String>, String> {
    const KINDS: [&str; 5] = ["home", "page", "collection", "work", "infographic"];
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("engineer.json does not parse: {e}"))?;
    let pages = v["pages"].as_array().filter(|a| !a.is_empty()).ok_or("engineer.json has no pages")?;
    let mut out: Vec<String> = Vec::new();
    let mut homes = 0;
    for p in pages {
        let path = p["path"].as_str().ok_or("engineer.json entry without a path")?;
        let inner = path.strip_prefix("/engineer/").ok_or(format!("engineer.json: {path} is not under /engineer/"))?;
        if !path.ends_with('/')
            || !inner.trim_end_matches('/').split('/').all(|s| inner.is_empty() || (!s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')))
        {
            return Err(format!("engineer.json: {path} must be lowercase a-z, 0-9 and - segments ending in /"));
        }
        if out.iter().any(|o| o == path) {
            return Err(format!("engineer.json: {path} is listed twice"));
        }
        for field in ["name", "title", "description"] {
            if p[field].as_str().is_none_or(|s| s.trim().is_empty()) {
                return Err(format!("engineer.json: {path} has no {field}"));
            }
        }
        let kind = p["kind"].as_str().unwrap_or_default();
        if !KINDS.contains(&kind) {
            return Err(format!("engineer.json: {path} has unknown kind {kind:?}"));
        }
        if kind == "home" {
            homes += 1;
            if path != "/engineer/" {
                return Err(format!("engineer.json: the home must be /engineer/, not {path}"));
            }
        }
        let old = p["old"].as_array().ok_or(format!("engineer.json: {path} has no old paths"))?;
        if !old.iter().all(|o| o.as_str().is_some_and(|s| s.starts_with('/') && !s.starts_with("//"))) {
            return Err(format!("engineer.json: {path} old paths must be root-relative"));
        }
        out.push(path.to_string());
    }
    if homes != 1 {
        return Err(format!("engineer.json needs exactly one home, found {homes}"));
    }
    Ok(out)
}

/// The engineer record and the About-the-engineer section of [`PAGES`] name
/// exactly the same pages: the Rust builder registers every engineer page in
/// [`PAGES`], and the record is the authored source the generator renders.
pub fn check_engineer_registered(paths: &[String]) -> Result<(), String> {
    for path in paths {
        if !seo::info_for(path).is_some_and(|p| p.section == seo::Section::Engineer) {
            return Err(format!("engineer.json: {path} is not an About-the-engineer page in seo::PAGES"));
        }
    }
    for page in PAGES.iter().filter(|p| p.section == seo::Section::Engineer) {
        if !paths.iter().any(|p| p == page.path) {
            return Err(format!("seo::PAGES: {} is missing from engineer.json", page.path));
        }
    }
    Ok(())
}

/// Every English page the site must publish: [`PAGES`], which carries the
/// engineer record's pages once the record is shown to agree with it. Both
/// are Rust-side sources, never the generator's own output.
pub fn registry() -> BTreeSet<String> {
    let engineer = engineer_paths(ENGINEER_RAW).unwrap_or_else(|e| panic!("{e}"));
    check_engineer_registered(&engineer).unwrap_or_else(|e| panic!("{e}"));
    PAGES.iter().map(|p| p.path.to_string()).collect()
}

/// The written pages and the registry agree in both directions, and every
/// translated page mirrors a registered English page that was written.
pub fn check_registry(paths: &[String]) -> Result<(), String> {
    check_registry_against(paths, &registry())
}

fn check_registry_against(paths: &[String], registry: &BTreeSet<String>) -> Result<(), String> {
    let written: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    for p in paths {
        let en = en_of(p);
        if !registry.contains(&en) {
            return Err(if is_zh(p) {
                format!("{p} has no English page in the registry")
            } else {
                format!("{p} was written but is not in seo::PAGES or the engineer record")
            });
        }
        if is_zh(p) && !written.contains(en.as_str()) {
            return Err(format!("{p} mirrors {en}, which was not written"));
        }
    }
    for path in registry {
        if !written.contains(path.as_str()) {
            return Err(format!("the registry lists {path} but no such page was written"));
        }
    }
    Ok(())
}

/// The opening tags of a head, with the text of its `<title>`.
struct Head {
    title: Option<String>,
    tags: Vec<String>,
}

fn read_head(html: &str) -> Result<Head, String> {
    let lower = html.to_ascii_lowercase();
    let open = lower
        .match_indices("<head")
        .map(|(h, _)| h)
        .find(|&h| lower[h + 5..].starts_with(['>', ' ', '\t', '\n', '\r']))
        .ok_or("no <head>")?;
    let start = open + lower[open..].find('>').ok_or("unterminated <head>")? + 1;
    let end = start + lower[start..].find("</head>").ok_or("no </head>")?;
    let head = &html[start..end];
    let lower_head = &lower[start..end];
    let mut tags = Vec::new();
    let mut title = None;
    let mut i = 0;
    while let Some(lt) = head[i..].find('<') {
        let at = i + lt;
        let rest = &head[at..];
        if rest.starts_with("<!--") {
            i = at + rest.find("-->").ok_or("unterminated comment in <head>")? + 3;
            continue;
        }
        let gt = rest.find('>').ok_or("unterminated tag in <head>")?;
        let tag = &rest[..=gt];
        let name = tag_name(tag);
        if !rest.starts_with("</") && (name == "title" || name == "script" || name == "style") {
            let close = format!("</{name}");
            let body_start = at + gt + 1;
            let close_at = lower_head[body_start..].find(&close).map(|c| body_start + c).ok_or(format!("<{name}> does not close in <head>"))?;
            if name == "title" {
                if title.is_some() {
                    return Err("more than one <title>".into());
                }
                title = Some(head[body_start..close_at].trim().to_string());
            }
            tags.push(tag.to_string());
            i = close_at + close.len();
            continue;
        }
        if !rest.starts_with("</") && !rest.starts_with("<!") {
            tags.push(tag.to_string());
        }
        i = at + gt + 1;
    }
    Ok(Head { title, tags })
}

/// The head of one page: a title, a description, the social title and
/// description, one canonical URL on the www host that is this page, the
/// og:url equal to it, the language of the page, and hreflang alternates
/// that pair an English page with its translation in both directions.
/// `twin` is the written page in the other language, if any (see
/// [`TRANSLATED_ELSEWHERE`]).
pub fn check_head(path: &str, html: &str, twin: Option<&str>) -> Result<(), String> {
    let head = read_head(html)?;
    if head.title.as_deref().is_none_or(str::is_empty) {
        return Err("no <title>".into());
    }
    let meta = |key: &str| {
        head.tags
            .iter()
            .filter(|t| tag_name(t) == "meta")
            .filter(|t| attr(t, "name").or_else(|| attr(t, "property")).as_deref() == Some(key))
            .filter_map(|t| attr(t, "content"))
            .find(|c| !c.trim().is_empty())
    };
    if meta("description").is_none() && meta("og:description").is_none() {
        return Err("no meta description".into());
    }
    if meta("og:title").is_none() {
        return Err("no og:title".into());
    }
    if meta("og:description").is_none() {
        return Err("no og:description".into());
    }
    let links = |rel: &str| -> Vec<&String> {
        head.tags.iter().filter(|t| tag_name(t) == "link" && attr(t, "rel").as_deref() == Some(rel)).collect()
    };
    let url = format!("{HOST}{path}");
    let canonical: Vec<String> = links("canonical").iter().filter_map(|t| attr(t, "href")).collect();
    if canonical != [url.clone()] {
        return Err(format!("needs exactly one canonical {url}, found {canonical:?}"));
    }
    if meta("og:url").as_deref() != Some(url.as_str()) {
        return Err(format!("og:url must equal the canonical {url}"));
    }
    let lang = html
        .find("<html")
        .and_then(|h| html[h..].find('>').map(|g| attr(&html[h..=h + g], "lang")))
        .flatten()
        .unwrap_or_default();
    let lang_ok = if is_zh(path) { lang == "zh-CN" } else { lang == "en" || lang.starts_with("en-") };
    if !lang_ok {
        return Err(format!("<html lang> is {lang:?}, which is not the language of {path}"));
    }
    let hreflang = |code: &str| -> Option<String> {
        links("alternate").iter().find(|t| attr(t, "hreflang").as_deref() == Some(code)).and_then(|t| attr(t, "href"))
    };
    if let Some(twin) = twin {
        let (en, zh) = if is_zh(path) { (twin, path) } else { (path, twin) };
        let (en, zh) = (format!("{HOST}{en}"), format!("{HOST}{zh}"));
        if hreflang("en").as_deref() != Some(en.as_str()) || hreflang("zh-CN").as_deref() != Some(zh.as_str()) {
            return Err(format!("a translated pair must name both: hreflang en {en} and zh-CN {zh}"));
        }
    } else if hreflang("zh-CN").is_some_and(|h| h != url) || hreflang("en").is_some_and(|h| h != url) {
        return Err("hreflang names a translation that was not written".into());
    }
    Ok(())
}

/// Every page in dist: registered, with a complete head, and linking the
/// six destinations (there is no nav bar, so each page carries them itself).
pub fn verify_pages(dist: &Path) -> Result<usize, String> {
    let paths = html_pages(dist)?;
    check_registry(&paths)?;
    let written: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    for p in &paths {
        let file = dist.join(p.trim_start_matches('/')).join("index.html");
        let html = fs::read_to_string(&file).map_err(|e| format!("read {}: {e}", file.display()))?;
        let twin = twin_of(p, &written);
        check_head(p, &html, twin.as_deref()).map_err(|e| format!("{p}: {e}"))?;
        crate::components::navbar::check_destinations(&html).map_err(|e| format!("{p}: {e}"))?;
    }
    Ok(paths.len())
}

fn no_dash(name: &str, raw: &str) -> Result<(), String> {
    if raw.contains('\u{2014}') || raw.contains('\u{2013}') {
        return Err(format!("{name} has an em or en dash"));
    }
    Ok(())
}

pub fn check_machines(raw: &str) -> Result<(), String> {
    no_dash("machines.json", raw)?;
    crate::components::machines::parse(raw).map(|_| ())
}

pub fn check_omega(raw: &str) -> Result<(), String> {
    no_dash("omega.json", raw)?;
    crate::components::omega::validate(&crate::components::omega::parse(raw)?)
}

pub fn check_cudaplusplus(raw: &str) -> Result<(), String> {
    no_dash("cudaplusplus.json", raw)?;
    crate::components::cudaplusplus::validate(&crate::components::cudaplusplus::parse(raw)?)
}

pub fn check_research_status(raw: &str) -> Result<(), String> {
    crate::components::research_status::parse(raw).map(|_| ())
}

pub fn check_progress(raw: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("project-progress.json does not parse: {e}"))?;
    crate::collector::validate_snapshot(&v)
}

/// The layer states: the five fixed status words, and every layer named,
/// in one of them, with a note and either no receipt or an https one.
pub fn check_status(raw: &str) -> Result<(), String> {
    const STATUSES: [&str; 5] = ["IMPLEMENTED", "QUALIFIED", "EXPERIMENTAL", "BLOCKED", "PLANNED"];
    no_dash("status.json", raw)?;
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("status.json does not parse: {e}"))?;
    let statuses: Vec<&str> = v["statuses"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    if statuses != STATUSES {
        return Err(format!("status.json statuses differ from the fixed set: {statuses:?}"));
    }
    let layers = v["layers"].as_array().filter(|a| !a.is_empty()).ok_or("status.json has no layers")?;
    for l in layers {
        let name = l["name"].as_str().filter(|s| !s.trim().is_empty()).ok_or("status.json layer without a name")?;
        let status = l["status"].as_str().unwrap_or_default();
        if !STATUSES.contains(&status) {
            return Err(format!("status.json layer {name}: unknown status {status:?}"));
        }
        if l["note"].as_str().is_none_or(|s| s.trim().is_empty()) {
            return Err(format!("status.json layer {name}: no note"));
        }
        match &l["receipt"] {
            Value::Null => {}
            Value::String(r) if r.starts_with("https://") => {}
            other => return Err(format!("status.json layer {name}: receipt must be null or an https URL, not {other}")),
        }
    }
    Ok(())
}

/// data/pages.json, the published page list, names exactly the pages of
/// [`PAGES`], each once, with a title and a description, as its own
/// English page. The oracle never reads this file to decide what the site
/// is; it checks the file against the Rust registry.
pub fn check_pages_json(raw: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("data/pages.json does not parse: {e}"))?;
    let rows = v.as_array().ok_or("data/pages.json is not a list")?;
    let mut listed = BTreeSet::new();
    for r in rows {
        let path = r["path"].as_str().ok_or("data/pages.json entry without a path")?;
        for field in ["title", "description"] {
            if r[field].as_str().is_none_or(|s| s.trim().is_empty()) {
                return Err(format!("data/pages.json: {path} has no {field}"));
            }
        }
        if r["english"].as_str() != Some(path) {
            return Err(format!("data/pages.json: {path} must name itself as its English page"));
        }
        if !listed.insert(path) {
            return Err(format!("data/pages.json lists {path} twice"));
        }
    }
    let registry = registry();
    let expected: BTreeSet<&str> = registry.iter().map(String::as_str).collect();
    if listed != expected {
        let missing: Vec<_> = expected.difference(&listed).collect();
        let extra: Vec<_> = listed.difference(&expected).collect();
        return Err(format!("data/pages.json differs from seo::PAGES plus the engineer record: missing {missing:?}, unexpected {extra:?}"));
    }
    Ok(())
}

/// data/index.json lists exactly [`DATA_FILES`], each at its www URL.
pub fn check_data_index(raw: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("data/index.json does not parse: {e}"))?;
    let files = v["files"].as_array().ok_or("data/index.json has no files")?;
    let mut listed = BTreeSet::new();
    for f in files {
        let path = f["path"].as_str().ok_or("data/index.json entry without a path")?;
        if f["url"].as_str() != Some(format!("{HOST}{path}").as_str()) {
            return Err(format!("data/index.json: {path} must carry url {HOST}{path}"));
        }
        if !listed.insert(path) {
            return Err(format!("data/index.json lists {path} twice"));
        }
    }
    let expected: BTreeSet<&str> = DATA_FILES.iter().map(|d| d.path).collect();
    if listed != expected {
        let missing: Vec<_> = expected.difference(&listed).collect();
        let extra: Vec<_> = listed.difference(&expected).collect();
        return Err(format!("data/index.json differs from seo::DATA_FILES: missing {missing:?}, unexpected {extra:?}"));
    }
    Ok(())
}

/// Every published data file parses and passes the same validation the
/// renderer applied to its source, read from dist rather than compiled in.
pub fn verify_data(dist: &Path) -> Result<(), String> {
    let read = |rel: &str| fs::read_to_string(dist.join(rel)).map_err(|e| format!("read {rel}: {e}"));
    let checks: [(&str, fn(&str) -> Result<(), String>); 9] = [
        ("machines.json", check_machines),
        ("omega.json", check_omega),
        ("cudaplusplus.json", check_cudaplusplus),
        ("research-status.json", check_research_status),
        ("status.json", check_status),
        ("data/project-progress.json", check_progress),
        ("data/index.json", check_data_index),
        ("data/pages.json", check_pages_json),
        ("now.json", |raw| serde_json::from_str::<Value>(raw).map(|_| ()).map_err(|e| format!("now.json does not parse: {e}"))),
    ];
    for (rel, check) in checks {
        check(&read(rel)?).map_err(|e| format!("{rel}: {e}"))?;
    }
    crate::facts::Facts::load(&dist.join("facts.json"))?;
    crate::components::agents::Repos::load(&dist.join("data/agent-repos.json"))?;
    // The published engineer record is the committed one, byte for byte.
    if read("data/engineer.json")? != ENGINEER_RAW {
        return Err("data/engineer.json differs from the committed public/data/engineer.json".into());
    }
    Ok(())
}

/// The root is Allen's native front door. Real elements (not text in a
/// comment or a script) carry `id="allen"` and `id="site-door"`; a reader
/// with scripts reaches a link to the classic site, and so does a reader
/// without them, through a `<noscript>` link of its own.
pub fn check_root(html: &str) -> Result<(), String> {
    for id in ["allen", "site-door"] {
        if !has_id(html, id) {
            return Err(format!("the root has no element with id=\"{id}\""));
        }
    }
    let classic = |links: Vec<(String, String)>| links.iter().any(|(h, t)| h == "/classic/" && !t.is_empty());
    if !classic(anchors(&reachable(html))) {
        return Err("the root must link the classic site at /classic/ where a reader can reach it".into());
    }
    if !noscript(html).iter().any(|n| classic(anchors(&strip(n, &[], true)))) {
        return Err("the root must link /classic/ inside <noscript> for readers without scripts".into());
    }
    Ok(())
}

/// Every root-relative script source and stylesheet that `html` loads, in
/// real markup (comments skipped; script and style bodies are not markup).
pub fn page_assets(html: &str) -> Vec<String> {
    let m = strip(html, &[], false);
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(lt) = m[i..].find('<') {
        let at = i + lt;
        let Some(gt) = m[at..].find('>').map(|g| at + g) else { break };
        let tag = &m[at..=gt];
        let name = tag_name(tag);
        i = gt + 1;
        if tag.starts_with("</") {
            continue;
        }
        let url = match name.as_str() {
            "script" => attr(tag, "src"),
            "link" if attr(tag, "rel").is_some_and(|r| r.split_ascii_whitespace().any(|t| t.eq_ignore_ascii_case("stylesheet"))) => attr(tag, "href"),
            _ => None,
        };
        if let Some(u) = url.filter(|u| u.starts_with('/') && !u.starts_with("//")) {
            out.push(u);
        }
        if name == "script" || name == "style" {
            i = element_end(&m, gt + 1, &name).unwrap_or(m.len());
        }
    }
    out
}

/// The regular file under dist that a root-relative URL names, or why not.
pub fn resolve_asset(dist: &Path, url: &str) -> Result<(), String> {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let rel = path.trim_start_matches('/');
    if rel.is_empty() || path.ends_with('/') || rel.split('/').any(|s| s.is_empty() || s == "." || s == ".." || s.contains('\\')) {
        return Err(format!("{url} does not name a file"));
    }
    let file = dist.join(rel);
    let meta = fs::symlink_metadata(&file).map_err(|_| format!("{url} is not in dist"))?;
    if !meta.file_type().is_file() {
        return Err(format!("{url} is not a regular file in dist"));
    }
    Ok(())
}

/// Every `.html` in dist loads only scripts and stylesheets that dist holds.
pub fn verify_assets(dist: &Path) -> Result<usize, String> {
    fn walk(dir: &Path, dist: &Path, n: &mut usize) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
            let p = entry.path();
            if entry.file_type().map_err(|e| format!("stat {}: {e}", p.display()))?.is_dir() {
                walk(&p, dist, n)?;
            } else if p.extension().is_some_and(|e| e == "html") {
                let html = fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
                for url in page_assets(&html) {
                    resolve_asset(dist, &url).map_err(|e| format!("{}: {e}", p.display()))?;
                    *n += 1;
                }
            }
        }
        Ok(())
    }
    let mut n = 0;
    walk(dist, dist, &mut n)?;
    Ok(n)
}

/// The sitemap lists every registered page, engineer record included, at
/// its www URL. (seo::verify covers [`PAGES`] with its other surfaces.)
pub fn check_sitemap(sitemap: &str) -> Result<(), String> {
    for path in registry() {
        if !sitemap.contains(&format!("<loc>{HOST}{path}</loc>")) {
            return Err(format!("sitemap.xml must list {path}"));
        }
    }
    Ok(())
}

/// The verify-only gates, in one call from `verify_site`.
pub fn verify(dist: &Path) {
    let n = verify_pages(dist).unwrap_or_else(|e| panic!("{e}"));
    let sitemap = fs::read_to_string(dist.join("sitemap.xml")).unwrap_or_else(|e| panic!("read sitemap.xml: {e}"));
    check_sitemap(&sitemap).unwrap_or_else(|e| panic!("{e}"));
    let root = fs::read_to_string(dist.join("index.html")).unwrap_or_else(|e| panic!("read index.html: {e}"));
    check_root(&root).unwrap_or_else(|e| panic!("index.html: {e}"));
    let assets = verify_assets(dist).unwrap_or_else(|e| panic!("{e}"));
    verify_data(dist).unwrap_or_else(|e| panic!("{e}"));
    println!("  Registry, heads and translation pairs: {n} pages; {assets} script and stylesheet loads resolve; published data files validated");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &str, lang: &str, extra: &str) -> String {
        format!(
            "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><title>T</title><meta name=\"description\" content=\"D\"><meta property=\"og:title\" content=\"T\"><meta property=\"og:description\" content=\"D\"><link rel=\"canonical\" href=\"{HOST}{path}\"><meta property=\"og:url\" content=\"{HOST}{path}\">{extra}</head><body></body></html>"
        )
    }

    fn pair(en: &str, zh: &str) -> String {
        format!("<link rel=\"alternate\" hreflang=\"en\" href=\"{HOST}{en}\"><link rel=\"alternate\" hreflang=\"zh-CN\" href=\"{HOST}{zh}\">")
    }

    fn all_pages() -> Vec<String> {
        registry().into_iter().collect()
    }

    #[test]
    fn sitemap_must_list_every_registered_page() {
        let full: String = all_pages().iter().map(|p| format!("<url><loc>{HOST}{p}</loc></url>")).collect();
        check_sitemap(&full).unwrap();
        assert!(check_sitemap(&full.replace(&format!("<loc>{HOST}/engineer/</loc>"), "")).is_err(), "an engineer page left out");
        assert!(check_sitemap(&full.replace(&format!("<loc>{HOST}/classic/</loc>"), "<loc>https://aienos.com/classic/</loc>")).is_err(), "the apex host");
    }

    #[test]
    fn the_engineer_record_is_registered() {
        let paths = engineer_paths(ENGINEER_RAW).expect("the committed record validates");
        assert!(paths.iter().any(|p| p == "/engineer/"));
        check_engineer_registered(&paths).expect("the record and seo::PAGES agree");
        let all = registry();
        assert_eq!(all.len(), PAGES.len(), "every engineer page is one seo::PAGES entry");
        assert!(paths.iter().all(|p| all.contains(p)));
        let dropped: Vec<String> = paths.iter().filter(|p| p.as_str() != "/engineer/").cloned().collect();
        assert!(check_engineer_registered(&dropped).unwrap_err().contains("/engineer/ is missing"), "a page only in seo::PAGES is caught");
        let mut extra = paths.clone();
        extra.push("/engineer/not-registered/".to_string());
        assert!(check_engineer_registered(&extra).unwrap_err().contains("/engineer/not-registered/"), "a page only in the record is caught");
        let mut stray = paths.clone();
        stray.push("/machine/".to_string());
        assert!(check_engineer_registered(&stray).is_err(), "a page from another section is caught");
        let missing: Vec<String> = all_pages().into_iter().filter(|p| p != "/engineer/").collect();
        assert!(check_registry(&missing).unwrap_err().contains("/engineer/"), "a dropped engineer page is caught");
    }

    #[test]
    fn engineer_record_negative_controls() {
        let entry = |path: &str, kind: &str| format!("{{\"path\":\"{path}\",\"old\":[\"/x\"],\"name\":\"N\",\"title\":\"T\",\"description\":\"D\",\"kind\":\"{kind}\",\"body_html\":\"\"}}");
        let rec = |rows: &[String]| format!("{{\"source\":\"s\",\"pages\":[{}]}}", rows.join(","));
        let home = entry("/engineer/", "home");
        engineer_paths(&rec(&[home.clone(), entry("/engineer/works/a-1/", "work")])).unwrap();
        let cases = [
            ("no pages", rec(&[])),
            ("no home", rec(&[entry("/engineer/a/", "page")])),
            ("two homes", rec(&[home.clone(), entry("/engineer/b/", "home")])),
            ("outside /engineer/", rec(&[home.clone(), entry("/machine/x/", "page")])),
            ("uppercase path", rec(&[home.clone(), entry("/engineer/Atlas/", "page")])),
            ("prefix in another case", rec(&[home.clone(), entry("/Engineer/a/", "page")])),
            ("no trailing slash", rec(&[home.clone(), entry("/engineer/a", "page")])),
            ("empty segment", rec(&[home.clone(), entry("/engineer//a/", "page")])),
            ("dot segment", rec(&[home.clone(), entry("/engineer/../a/", "page")])),
            ("duplicate", rec(&[home.clone(), entry("/engineer/a/", "page"), entry("/engineer/a/", "page")])),
            ("unknown kind", rec(&[home.clone(), entry("/engineer/a/", "secret")])),
            ("empty title", rec(&[home.clone(), entry("/engineer/a/", "page").replace("\"title\":\"T\"", "\"title\":\" \"")])),
            ("external old path", rec(&[home.clone(), entry("/engineer/a/", "page").replace("[\"/x\"]", "[\"//evil.example/\"]")])),
            ("not JSON", "{".to_string()),
        ];
        for (what, raw) in cases {
            assert!(engineer_paths(&raw).is_err(), "must reject: {what}");
        }
    }

    #[test]
    fn registry_accepts_exactly_the_listed_pages() {
        let mut paths = all_pages();
        check_registry(&paths).expect("the registry itself passes");
        paths.push("/zh-CN/machine/".into());
        check_registry(&paths).expect("a mirror of a written page passes");
    }

    #[test]
    fn registry_rejects_an_extra_page_a_missing_page_and_an_orphan_mirror() {
        let mut extra = all_pages();
        extra.push("/foo/".into());
        assert!(check_registry(&extra).unwrap_err().contains("/foo/ was written but is not in seo::PAGES"));
        let mut stray = all_pages();
        stray.push("/engineer/unlisted/".into());
        assert!(check_registry(&stray).is_err(), "an engineer page not in the record");
        let missing: Vec<String> = all_pages().into_iter().filter(|p| p != "/discoveries/").collect();
        assert!(check_registry(&missing).unwrap_err().contains("lists /discoveries/"));
        let mut orphan = all_pages();
        orphan.push("/zh-CN/foo/".into());
        assert!(check_registry(&orphan).unwrap_err().contains("no English page"));
    }

    #[test]
    fn registry_carries_the_six_dashboards_and_the_peer_pages() {
        for p in ["/", "/classic/", "/omega/", "/machine/", "/discoveries/", "/learn/", "/allen/", "/alpha/"] {
            assert!(seo::info_for(p).is_some(), "{p} must be registered");
        }
    }

    const ROOT: &str = "<body><div id=\"allen\" class=\"allen-scene\"><button id=\"site-door\">Enter</button></div><a href=\"/classic/\">Classic site</a><noscript><a href=\"/classic/\">Read the classic site</a></noscript><main>intro</main></body>";

    #[test]
    fn the_root_door_passes() {
        check_root(ROOT).expect("real Allen and door, classic linked with and without scripts");
    }

    #[test]
    fn root_negative_controls() {
        let cases = [
            ("allen id only in a comment", ROOT.replace("<div id=\"allen\" class=\"allen-scene\">", "<!-- id=\"allen\" --><div class=\"allen-scene\">")),
            ("allen id only in a script", ROOT.replace("<div id=\"allen\" class=\"allen-scene\">", "<script>var s='<div id=\"allen\">'</script><div class=\"allen-scene\">")),
            ("door id only as text", ROOT.replace("<button id=\"site-door\">Enter</button>", "<p>id=\"site-door\"</p>")),
            ("door id only in a template", ROOT.replace("<button id=\"site-door\">Enter</button>", "<template><button id=\"site-door\">Enter</button></template>")),
            ("classic link hidden", ROOT.replace("<a href=\"/classic/\">Classic site</a>", "<a href=\"/classic/\" hidden>Classic site</a>")),
            ("classic link only in noscript", ROOT.replace("<a href=\"/classic/\">Classic site</a>", "")),
            ("classic link only in a comment", ROOT.replace("<a href=\"/classic/\">Classic site</a>", "<!-- <a href=\"/classic/\">Classic site</a> -->")),
            ("no noscript route", ROOT.replace("<noscript><a href=\"/classic/\">Read the classic site</a></noscript>", "")),
            ("noscript route hidden", ROOT.replace("<noscript><a href=\"/classic/\">", "<noscript><a href=\"/classic/\" aria-hidden=\"true\">")),
            ("noscript inside a hidden element", ROOT.replace("<noscript>", "<div hidden><noscript>").replace("</noscript>", "</noscript></div>")),
            ("noscript route without text", ROOT.replace("Read the classic site", "")),
        ];
        for (what, html) in cases {
            assert!(check_root(&html).is_err(), "must reject: {what}");
        }
    }

    #[test]
    fn reachability_drops_hidden_noscript_and_non_markup() {
        let html = "<p>a</p><!-- <p>b</p> --><noscript><p>c</p></noscript><div hidden><p>d</p></div><div aria-hidden=\"true\"><p>e</p></div><script>'<p>f</p>'</script><template><p>g</p></template><p title=\"hidden\" class=\"visually-hidden\">h</p><img hidden src=\"/x.png\"><p>i</p>";
        let t = reader_text(&reachable(html));
        assert_eq!(t, "a h i");
        assert_eq!(attr_names("<p title=\"hidden\" data-x='a b' hidden>"), ["title", "data-x", "hidden"]);
        assert_eq!(reader_text(&reachable("<p>a</p><div hidden><p>never closes")), "a", "unclosed hidden content hides, never adds");
    }

    #[test]
    fn assets_must_resolve_to_regular_files() {
        let dir = std::env::temp_dir().join(format!("oracle-assets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("js/dir.js")).unwrap();
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(dir.join("js/site.js"), "x").unwrap();
        fs::write(dir.join("assets/site.css"), "x").unwrap();
        let page = "<head><link rel=\"stylesheet\" href=\"/assets/site.css?v=1\"><script defer src=\"/js/site.js\"></script><!-- <script src=\"/js/gone.js\"></script> --><script>var s='<script src=\"/js/inline.js\">'</script><link rel=\"icon\" href=\"/missing.ico\"><script src=\"https://example.com/a.js\"></script></head>";
        assert_eq!(page_assets(page), ["/assets/site.css?v=1", "/js/site.js"]);
        for url in page_assets(page) {
            resolve_asset(&dir, &url).unwrap();
        }
        for bad in ["/js/missing.js", "/js/dir.js", "/js/", "/", "/js/../js/site.js", "/js//site.js"] {
            assert!(resolve_asset(&dir, bad).is_err(), "must reject {bad}");
        }
        fs::write(dir.join("index.html"), page).unwrap();
        assert_eq!(verify_assets(&dir).unwrap(), 2);
        fs::write(dir.join("index.html"), page.replace("/js/site.js", "/js/missing.js")).unwrap();
        assert!(verify_assets(&dir).is_err(), "a page loading a missing script fails");
        fs::write(dir.join("index.html"), page.replace("rel=\"stylesheet\" href=\"/assets/site.css?v=1\"", "rel=\"preload stylesheet\" href=\"/assets/gone.css\"")).unwrap();
        assert!(verify_assets(&dir).is_err(), "a missing stylesheet fails");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn walker_finds_every_index_page() {
        let dir = std::env::temp_dir().join(format!("oracle-walk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for rel in ["index.html", "machine/index.html", "zh-CN/machine/index.html", "machine/other.html"] {
            let f = dir.join(rel);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, "x").unwrap();
        }
        assert_eq!(html_pages(&dir).unwrap(), ["/", "/machine/", "/zh-CN/machine/"]);
        fs::remove_dir_all(&dir).unwrap();
        assert!(html_pages(&dir).is_err(), "a missing tree is an error, not an empty site");
    }

    #[test]
    fn a_complete_head_passes() {
        let (m, zm) = ("/machine/", "/zh-CN/machine/");
        check_head(m, &page(m, "en", ""), None).unwrap();
        check_head(m, &page(m, "en", &pair(m, zm)), Some(zm)).unwrap();
        check_head(zm, &page(zm, "zh-CN", &pair(m, zm)), Some(m)).unwrap();
        check_head("/classic/", &page("/classic/", "en", &pair("/classic/", "/zh-CN/")), Some("/zh-CN/")).unwrap();
        check_head("/zh-CN/", &page("/zh-CN/", "zh-CN", &pair("/classic/", "/zh-CN/")), Some("/classic/")).unwrap();
        check_head("/", &page("/", "en", ""), None).expect("the Allen root claims no translation");
        let commented = page(m, "en", "<!-- <link rel=\"canonical\" href=\"https://aienos.com/old/\"> -->");
        check_head(m, &commented, None).expect("a commented-out tag is not a tag");
    }

    #[test]
    fn translation_pairs_follow_the_explicit_map() {
        let written: BTreeSet<&str> = ["/", "/classic/", "/zh-CN/", "/machine/", "/zh-CN/machine/", "/omega/"].into_iter().collect();
        assert_eq!(twin_of("/zh-CN/", &written).as_deref(), Some("/classic/"));
        assert_eq!(twin_of("/classic/", &written).as_deref(), Some("/zh-CN/"));
        assert_eq!(twin_of("/", &written), None, "the root is not translated");
        assert_eq!(twin_of("/machine/", &written).as_deref(), Some("/zh-CN/machine/"));
        assert_eq!(twin_of("/omega/", &written), None);
        let mut paths = all_pages();
        paths.push("/zh-CN/".into());
        check_registry(&paths).expect("/zh-CN/ mirrors /classic/");
    }

    #[test]
    fn head_negative_controls() {
        let good = page("/machine/", "en", "");
        let m = "/machine/";
        let zm = "/zh-CN/machine/";
        let cases: [(&str, &str, String, Option<&str>); 16] = [
            ("no <head>", m, good.replace("<head>", "<div>"), None),
            ("no </head>", m, good.replace("</head>", ""), None),
            ("empty title", m, good.replace("<title>T</title>", "<title> </title>"), None),
            ("no description", m, good.replace("<meta name=\"description\" content=\"D\">", "").replace("<meta property=\"og:description\" content=\"D\">", ""), None),
            ("no og:title", m, good.replace("<meta property=\"og:title\" content=\"T\">", ""), None),
            ("canonical elsewhere", m, good.replace(&format!("href=\"{HOST}/machine/\""), "href=\"https://aienos.com/machine/\""), None),
            ("two canonicals", m, good.replace("</head>", &format!("<link rel=\"canonical\" href=\"{HOST}/machine/\"></head>")), None),
            ("og:url differs", m, good.replace(&format!("content=\"{HOST}/machine/\""), &format!("content=\"{HOST}/\"")), None),
            ("wrong language", m, good.replace("lang=\"en\"", "lang=\"zh-CN\""), None),
            ("title only outside the head", m, good.replace("<title>T</title>", "").replace("<body>", "<body><title>T</title>"), None),
            ("pair without hreflang", m, good.clone(), Some(zm)),
            ("hreflang to a missing translation", m, page(m, "en", &pair(m, zm)), None),
            ("zh page paired with the wrong English page", zm, page(zm, "zh-CN", &pair("/turing/", zm)), Some(m)),
            ("the Allen root claims the Chinese front page", "/", page("/", "en", &pair("/", "/zh-CN/")), None),
            ("/zh-CN/ paired with the root, not /classic/", "/zh-CN/", page("/zh-CN/", "zh-CN", &pair("/", "/zh-CN/")), Some("/classic/")),
            ("/classic/ without its pair", "/classic/", page("/classic/", "en", ""), Some("/zh-CN/")),
        ];
        for (what, path, html, twin) in cases {
            assert!(check_head(path, &html, twin).is_err(), "must reject: {what}");
        }
    }

    #[test]
    fn compiled_data_passes_its_published_checks() {
        check_machines(crate::components::machines::RAW).unwrap();
        check_omega(crate::components::omega::RAW).unwrap();
        check_cudaplusplus(crate::components::cudaplusplus::RAW).unwrap();
        check_research_status(crate::components::research_status::RAW).unwrap();
        check_status(&crate::components::status::status_json()).unwrap();
    }

    #[test]
    fn data_negative_controls() {
        let dash = |raw: &str| raw.replacen("\": \"", "\": \"\u{2014}", 1);
        assert!(check_machines(&dash(crate::components::machines::RAW)).is_err());
        assert!(check_omega(&dash(crate::components::omega::RAW)).is_err());
        assert!(check_cudaplusplus(&dash(crate::components::cudaplusplus::RAW)).is_err());
        assert!(check_research_status(&dash(crate::components::research_status::RAW)).is_err());
        for bad in ["{", "[]", "{\"machines\": 1}"] {
            assert!(check_machines(bad).is_err(), "machines {bad}");
            assert!(check_omega(bad).is_err(), "omega {bad}");
            assert!(check_cudaplusplus(bad).is_err(), "cudaplusplus {bad}");
            assert!(check_research_status(bad).is_err(), "research status {bad}");
            assert!(check_progress(bad).is_err(), "progress {bad}");
        }
        let status = crate::components::status::status_json();
        assert!(check_status(&status.replacen("\"status\": \"", "\"status\": \"ALMOST", 1)).is_err());
        assert!(check_status(&status.replace("\"BLOCKED\", ", "")).is_err());
        assert!(check_status("{\"statuses\":[\"IMPLEMENTED\",\"QUALIFIED\",\"EXPERIMENTAL\",\"BLOCKED\",\"PLANNED\"],\"layers\":[{\"name\":\"x\",\"status\":\"PLANNED\",\"note\":\"n\",\"receipt\":\"http://x\"}]}").is_err());
    }

    #[test]
    fn published_page_list_must_match_the_rust_registry() {
        let row = |p: &str| format!("{{\"path\":\"{p}\",\"title\":\"T\",\"description\":\"D\",\"english\":\"{p}\"}}");
        let all: Vec<String> = all_pages().iter().map(|p| row(p)).collect();
        check_pages_json(&format!("[{}]", all.join(","))).unwrap();
        let short = &all[..all.len() - 1];
        assert!(check_pages_json(&format!("[{}]", short.join(","))).is_err(), "a dropped page");
        let mut extra = all.clone();
        extra.push(row("/unlisted/"));
        assert!(check_pages_json(&format!("[{}]", extra.join(","))).is_err(), "an unlisted page");
        let mut twice = short.to_vec();
        twice.push(all[0].clone());
        assert!(check_pages_json(&format!("[{}]", twice.join(","))).is_err(), "a page twice");
        assert!(check_pages_json(&format!("[{}]", all.join(",").replacen("\"title\":\"T\"", "\"title\":\" \"", 1))).is_err(), "an empty title");
        assert!(check_pages_json(&format!("[{}]", all.join(",").replacen("\"english\":\"/\"", "\"english\":\"/x/\"", 1))).is_err(), "a wrong English page");
        assert!(check_pages_json("{}").is_err());
    }

    #[test]
    fn data_index_must_list_exactly_the_data_files() {
        let entry = |p: &str| format!("{{\"path\":\"{p}\",\"url\":\"{HOST}{p}\"}}");
        let all: Vec<String> = DATA_FILES.iter().map(|d| entry(d.path)).collect();
        check_data_index(&format!("{{\"files\":[{}]}}", all.join(","))).unwrap();
        let short = &all[..all.len() - 1];
        assert!(check_data_index(&format!("{{\"files\":[{}]}}", short.join(","))).is_err());
        let mut swapped = short.to_vec();
        swapped.push(entry("/data/secret.tsv"));
        assert!(check_data_index(&format!("{{\"files\":[{}]}}", swapped.join(","))).is_err(), "same length, different set");
        let wrong_url = all.join(",").replacen(HOST, "https://aienos.com", 1);
        assert!(check_data_index(&format!("{{\"files\":[{wrong_url}]}}")).is_err());
    }
}
