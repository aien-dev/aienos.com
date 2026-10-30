//! The machines and the speeds: what the project runs on today and how
//! fast its own engine goes, with a receipt beside every number.
//!
//! One data file, `data/machines.json`, compiled in with `include_str!`
//! and validated at build time and in the tests. Every machine and every
//! speed row must carry a public link; a row without one fails the build.
//! Numbers stand beside the physical ceiling of the machine they ran on
//! and beside my own previous run. A comparison with another project is
//! allowed when the same sentence has a real ISO date and a link to an
//! aien-dev receipt or to that project's own published numbers. Overclaims
//! and unsourced comparisons are refused. The one allowlisted sentence is
//! "Slower than I want."
//! The homepage shows both sections; /machine/ shows the machines first,
//! before the theory, so the page describes real hardware.

use std::sync::OnceLock;

use maud::{html, Markup};
use serde::Deserialize;

use super::boxlink::{foot, Target};

/// The raw data file, compiled into the binary.
pub const RAW: &str = include_str!("../../data/machines.json");

/// Phrases matched case-insensitively, with a word boundary on each side.
pub const COMPARISON_PHRASES: &[&str] = &[
    "better than",
    "worse than",
    "faster than",
    "slower than",
    "on par with",
    "on a par with",
    "level with",
    "ahead of",
    "same speed as",
    "as fast as",
    "factor of",
    "comparable to",
    "competitive with",
    "twice as",
    "twice the",
    "double the",
    "double as",
    "half as",
    "half the speed",
    "half the time",
    "in half",
];

/// Whole words, case-insensitively. Not stems.
/// Bare "behind" and "match" are ordinary English, so they are not claims.
/// Superlatives ("fastest", "slowest", "quickest", "best", "worst") count only
/// beside a project name, in `has_guarded_word`.
pub const COMPARISON_WORDS: &[&str] = &["beat", "beats", "beating", "beaten"];

/// A word that starts with one of these, case-insensitively: outperforms,
/// outpaced, surpasses, exceeded, rivals, rivalry.
pub const COMPARISON_STEMS: &[&str] = &["outperform", "outpace", "surpass", "exceed", "rival"];

/// Overclaims. These stay banned even when a date and a link sit beside them.
/// Phrases are lowercase; callers compare a lowercased string.
pub const OVERCLAIMS: [&str; 5] = [
    "replacement for cuda",
    "everything cuda gives you",
    "omega's replacement and its status",
    "beating cuda",
    "matching and then",
];

/// The only sentence that may say "slower than" with no date and no link.
const OWNED_SLOWER: &str = "Slower than I want.";

fn is_owned_slower(s: &str) -> bool {
    let t = s.trim();
    t == OWNED_SLOWER || t == "Slower than I want"
}

fn is_real_date(year: i32, month: u32, day: u32) -> bool {
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
    let dim = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    day <= dim
}

/// A real calendar date written YYYY-MM-DD. `2026-02-31` and `2026-13-01`
/// do not count. A digit glued on either side does not count.
fn has_real_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 10 {
        return false;
    }
    for i in 0..=b.len() - 10 {
        if i > 0 && b[i - 1].is_ascii_digit() {
            continue;
        }
        if i + 10 < b.len() && b[i + 10].is_ascii_digit() {
            continue;
        }
        let w = &b[i..i + 10];
        if w[4] != b'-' || w[7] != b'-' {
            continue;
        }
        if !w[..4].iter().all(u8::is_ascii_digit)
            || !w[5..7].iter().all(u8::is_ascii_digit)
            || !w[8..10].iter().all(u8::is_ascii_digit)
        {
            continue;
        }
        let year: i32 = std::str::from_utf8(&w[..4]).unwrap().parse().unwrap();
        let month: u32 = std::str::from_utf8(&w[5..7]).unwrap().parse().unwrap();
        let day: u32 = std::str::from_utf8(&w[8..10]).unwrap().parse().unwrap();
        if is_real_date(year, month, day) {
            return true;
        }
    }
    false
}

/// GitHub orgs whose repositories are the other project's own published record.
pub const PROJECT_GITHUB_ORGS: &[&str] = &[
    "ggml-org",
    "ggerganov",
    "vllm-project",
    "NVIDIA",
    "triton-lang",
];

/// Domains that publish the other project's own numbers.
pub const PROJECT_SOURCE_DOMAINS: &[&str] = &[
    "docs.vllm.ai",
    "vllm.ai",
    "developer.nvidia.com",
    "pytorch.org",
    "huggingface.co",
];

/// Project names that turn a nearby superlative, "twice", "double", "half", or "vs" into a comparison.
const PROJECT_TOKENS: &[&str] = &[
    "vllm", "llama", "tensorrt", "pytorch", "chatgpt", "openai", "gemini", "claude", "triton", "cudnn",
    "tgi", "sglang",
];

fn url_is_allowed(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let cut = rest.find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>')).unwrap_or(rest.len());
    let rest = &rest[..cut];
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host = host.split('@').next_back().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    if (host == "github.com" || host == "raw.githubusercontent.com")
        && (path.starts_with("/aien-dev/") || path == "/aien-dev")
    {
        return true;
    }
    if host == "github.com" {
        for org in PROJECT_GITHUB_ORGS {
            let prefix = format!("/{}/", org.to_ascii_lowercase());
            if path.starts_with(&prefix) || path == format!("/{org}").to_ascii_lowercase() {
                return true;
            }
        }
    }
    PROJECT_SOURCE_DOMAINS.iter().any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

fn each_url(s: &str, mut f: impl FnMut(&str)) {
    let lower = s.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &lower[i..];
        let http = rest.find("http://");
        let https = rest.find("https://");
        let at = match (http, https) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let Some(at) = at else { break };
        let start = i + at;
        let url = &s[start..];
        let end = url.find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>')).unwrap_or(url.len());
        f(&url[..end]);
        i = start + end.max(1);
    }
}

/// A receipt on an aien-dev host, or the other project's own domain, in this same sentence.
/// The bare words "receipt" and "published" are not a link. An unrelated https URL is not a source.
fn has_allowed_source(s: &str) -> bool {
    let mut ok = false;
    each_url(s, |url| {
        if url_is_allowed(url) {
            ok = true;
        }
    });
    ok
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn boundary_phrase(lower: &str, phrase: &str) -> bool {
    let mut from = 0;
    while let Some(at) = lower[from..].find(phrase) {
        let i = from + at;
        let before_ok = i == 0 || !is_word_char(lower.as_bytes()[i - 1] as char);
        let after = i + phrase.len();
        let after_ok = after >= lower.len() || !is_word_char(lower.as_bytes()[after] as char);
        if before_ok && after_ok {
            return true;
        }
        from = i + phrase.len();
    }
    false
}

fn has_word_form(lower: &str) -> bool {
    for word in lower.split(|c: char| !is_word_char(c)) {
        if word.is_empty() {
            continue;
        }
        if COMPARISON_WORDS.contains(&word) || COMPARISON_STEMS.iter().any(|stem| word.starts_with(stem)) {
            return true;
        }
    }
    false
}

fn project_near(lower: &str, at: usize) -> bool {
    let start = lower.floor_char_boundary(at.saturating_sub(48));
    let end = lower.ceil_char_boundary((at + 48).min(lower.len()));
    let window = &lower[start..end];
    PROJECT_TOKENS.iter().any(|token| boundary_phrase(window, token))
}

/// Superlatives and "twice" / "double" / "half" only when a project name sits nearby.
/// Bare uses ("the quickest walkthrough", "the best way", "GPU half") are not claims.
fn has_guarded_word(lower: &str) -> bool {
    const GUARDED: &[&str] = &[
        "best", "worst", "fastest", "slowest", "quickest", "twice", "double", "half",
    ];
    let mut from = 0;
    while from < lower.len() {
        let rest = &lower[from..];
        let mut found: Option<(usize, usize)> = None;
        for word in GUARDED {
            if let Some(at) = rest.find(word) {
                let i = from + at;
                let before_ok = i == 0 || !is_word_char(lower.as_bytes()[i - 1] as char);
                let after = i + word.len();
                let after_ok = after >= lower.len() || !is_word_char(lower.as_bytes()[after] as char);
                if before_ok && after_ok && found.map(|(p, _)| at < p).unwrap_or(true) {
                    found = Some((at, word.len()));
                }
            }
        }
        let Some((at, len)) = found else { break };
        let i = from + at;
        if project_near(lower, i) {
            return true;
        }
        from = i + len;
    }
    false
}

fn has_vs(lower: &str) -> bool {
    let b = lower.as_bytes();
    let mut i = 0;
    while i + 2 <= b.len() {
        if &b[i..i + 2] == b"vs" {
            let before = i == 0 || b[i - 1].is_ascii_whitespace();
            let mut end = i + 2;
            if end < b.len() && b[end] == b'.' {
                end += 1;
            }
            let after_ok = end >= b.len()
                || b[end].is_ascii_whitespace()
                || matches!(b[end], b')' | b']' | b',' | b';' | b':' | b'!' | b'?');
            if before && after_ok && project_near(lower, i) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn has_within_percent(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let mut from = 0;
    while let Some(at) = lower[from..].find("within") {
        let i = from + at;
        let before_ok = i == 0 || !is_word_char(bytes[i - 1] as char);
        let mut j = i + "within".len();
        if !before_ok || j >= bytes.len() || !bytes[j].is_ascii_whitespace() {
            from = i + "within".len();
            continue;
        }
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || !bytes[j].is_ascii_digit() {
            from = i + "within".len();
            continue;
        }
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j + 1 < bytes.len() && bytes[j] == b'.' && bytes[j + 1].is_ascii_digit() {
            j += 2;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
        }
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'%' {
            j += 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if lower[j..].starts_with("of") {
                let after = j + 2;
                if after >= lower.len() || !is_word_char(lower.as_bytes()[after] as char) {
                    return true;
                }
            }
        }
        from = i + "within".len();
    }
    false
}

fn is_x_tail(c: char) -> bool {
    c.is_whitespace()
        || matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '"' | '\'' | '\u{201d}' | '\u{2019}')
}

/// `2x vLLM`, `2× faster`, `1.8x.`, `(2x)`, `20.3-fold`, `12% faster`, or `3 times`.
/// A letter after `x` is not a multiplier, so `1920x1080` and hex `0xff` stay whole.
/// URLs are removed by the caller.
fn has_multiplier(lower: &str) -> bool {
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let start_ok = chars[i].is_ascii_digit() && (i == 0 || !is_word_char(chars[i - 1]));
        if !start_ok {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        if j + 1 < chars.len() && chars[j] == '.' && chars[j + 1].is_ascii_digit() {
            j += 2;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
        }
        let mut k = j;
        while k < chars.len() && chars[k] == ' ' {
            k += 1;
        }
        if chars.get(j) == Some(&'-') {
            let rest: String = chars[j + 1..].iter().collect();
            if rest.starts_with("fold") {
                let after = "fold".len();
                if after >= rest.len() || !is_word_char(rest[after..].chars().next().unwrap_or(' ')) {
                    return true;
                }
            }
        }
        if chars.get(k) == Some(&'%') {
            let mut m = k + 1;
            while m < chars.len() && chars[m] == ' ' {
                m += 1;
            }
            let rest: String = chars[m..].iter().collect();
            if rest.starts_with("faster") {
                let after = "faster".len();
                if after >= rest.len() || !is_word_char(rest[after..].chars().next().unwrap_or(' ')) {
                    return true;
                }
            }
        }
        if k < chars.len() && (chars[k] == 'x' || chars[k] == '\u{00d7}') {
            let zero = chars[i..j].iter().all(|c| *c == '0' || *c == '.');
            let hex = zero && chars[k] == 'x';
            if !hex {
                let after = k + 1;
                if after >= chars.len() || is_x_tail(chars[after]) {
                    return true;
                }
            }
        }
        if k > j {
            let rest: String = chars[k..].iter().collect();
            if rest.starts_with("times") {
                let after = "times".len();
                if after >= rest.len() || !is_word_char(rest[after..].chars().next().unwrap_or(' ')) {
                    return true;
                }
            }
        }
        i = j;
    }
    false
}

fn without_urls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let http = rest.find("http://");
        let https = rest.find("https://");
        let i = match (http, https) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let Some(i) = i else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = &rest[end..];
        out.push(' ');
    }
    out
}

fn negated_before(lower: &str, at: usize) -> bool {
    let start = lower.floor_char_boundary(at.saturating_sub(48));
    let mut window = &lower[start..at];
    if let Some(i) = window.rfind(['.', '!', '?']) {
        window = &window[i + 1..];
    }
    for word in ["not", "never", "no"] {
        if boundary_phrase(window, word) {
            return true;
        }
    }
    false
}

fn phrase_claim(lower: &str) -> bool {
    for phrase in COMPARISON_PHRASES {
        let mut from = 0;
        while let Some(at) = lower[from..].find(phrase) {
            let i = from + at;
            let before_ok = i == 0 || !is_word_char(lower.as_bytes()[i - 1] as char);
            let after = i + phrase.len();
            let after_ok = after >= lower.len() || !is_word_char(lower.as_bytes()[after] as char);
            if before_ok && after_ok {
                let denial = (*phrase == "comparable to" || *phrase == "competitive with") && negated_before(lower, i);
                if !denial {
                    return true;
                }
            }
            from = i + phrase.len();
        }
    }
    false
}

fn is_comparison_claim(s: &str) -> bool {
    let scanned = without_urls(&s.to_ascii_lowercase());
    phrase_claim(&scanned)
        || has_word_form(&scanned)
        || has_guarded_word(&scanned)
        || has_vs(&scanned)
        || has_within_percent(&scanned)
        || has_multiplier(&scanned)
}

/// `None` when the text is not a comparison, or is one honest sentence
/// with a real ISO date and a source link. Overclaims always return
/// "overclaim". A comparison with no date or no link in this same text
/// returns "unsourced comparison".
pub fn comparison_problem(s: &str) -> Option<&'static str> {
    if is_owned_slower(s) {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    for w in OVERCLAIMS {
        if lower.contains(w) {
            return Some("overclaim");
        }
    }
    if is_comparison_claim(s) && !(has_real_iso_date(s) && has_allowed_source(s)) {
        return Some("unsourced comparison");
    }
    None
}

fn href_in_tag(tag: &str) -> Option<&str> {
    let tag_l = tag.to_ascii_lowercase();
    for key in ["href=\"", "href='"] {
        if let Some(i) = tag_l.find(key) {
            let value = &tag[i + key.len()..];
            let quote = if key.ends_with('"') { '"' } else { '\'' };
            let end = value.find(quote)?;
            return Some(&value[..end]);
        }
    }
    None
}

fn closes_block(tag_l: &str) -> bool {
    tag_l.starts_with("</p")
        || tag_l.starts_with("</li")
        || tag_l.starts_with("</h")
        || tag_l.starts_with("</div")
        || tag_l.starts_with("</tr")
        || tag_l.starts_with("</dd")
        || tag_l.starts_with("</dt")
        || tag_l.starts_with("</blockquote")
        || tag_l.starts_with("</section")
        || tag_l.starts_with("</article")
        || tag_l.starts_with("<br")
}

fn open_name(tag_l: &str) -> Option<&str> {
    let t = tag_l.strip_prefix('<')?;
    if t.starts_with('/') || t.starts_with('!') {
        return None;
    }
    let end = t.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(t.len());
    if end == 0 {
        return None;
    }
    Some(&t[..end])
}

/// Skip a balanced element. `rest` begins just after the opening tag.
fn skip_balanced<'a>(rest: &'a str, name: &str) -> &'a str {
    let lower = rest.to_ascii_lowercase();
    let open = format!("<{name}");
    let close = format!("</{name}");
    let mut depth = 1usize;
    let mut i = 0usize;
    while depth > 0 && i < lower.len() {
        let rel_open = lower[i..].find(&open);
        let rel_close = lower[i..].find(&close);
        match (rel_open, rel_close) {
            (Some(o), Some(c)) if o < c => {
                depth += 1;
                i += o + open.len();
            }
            (_, Some(c)) => {
                depth -= 1;
                i += c + close.len();
                if depth == 0 {
                    let after = rest.get(i..).unwrap_or("");
                    let gt = after.find('>').map(|n| n + 1).unwrap_or(0);
                    return after.get(gt..).unwrap_or("");
                }
            }
            _ => return "",
        }
    }
    ""
}

/// Ledger and roster classes. `data-verbatim` alone does not opt out.
const VERBATIM_CLASSES: &[&str] = &[
    "now-item",
    "now-note",
    "now-done",
    "now-queue-item",
    "now-tl-item",
    "now-tl-evidence",
    "now-model-role",
    "agents-verbatim",
];

fn has_verbatim_class(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    let Some(i) = lower.find("class=") else { return false };
    let rest = &tag[i + 6..];
    let Some(quote) = rest.chars().next() else { return false };
    if quote != '"' && quote != '\'' {
        return false;
    }
    let rest = &rest[1..];
    let end = rest.find(quote).unwrap_or(rest.len());
    rest[..end].split_whitespace().any(|c| VERBATIM_CLASSES.iter().any(|v| c.eq_ignore_ascii_case(v)))
}

/// Program text in `pre` (and therefore `pre > code`) is not prose.
/// A bare inline `<code>` is prose. `data-verbatim` opts out only for the fixed class list.
fn skip_whole_element(tag: &str, tag_l: &str) -> bool {
    matches!(
        open_name(tag_l),
        Some("script" | "style" | "pre" | "textarea" | "kbd" | "samp")
    ) || has_verbatim_class(tag)
}

/// Keep link targets, drop tags, and break on block ends so a date in one
/// element does not license a claim in the next. Script, style, comments,
/// program text (`pre`, so `pre > code`, plus `textarea`, `kbd`, `samp`), and a
/// fixed verbatim class are not page prose. Inline `code` is prose.
fn readable_text(text: &str) -> String {
    if !text.contains('<') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map(|end| &rest[end + 3..]).unwrap_or("");
            continue;
        }
        let Some(end) = rest.find('>') else {
            out.push_str(rest);
            return out;
        };
        let tag = &rest[..=end];
        let tag_l = tag.to_ascii_lowercase();
        if !tag_l.ends_with("/>") && skip_whole_element(tag, &tag_l) {
            if let Some(name) = open_name(&tag_l) {
                rest = skip_balanced(&rest[end + 1..], name);
                continue;
            }
        }
        if let Some(url) = href_in_tag(tag) {
            out.push(' ');
            out.push_str(url);
            out.push(' ');
        }
        if closes_block(&tag_l) {
            out.push('\n');
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

fn is_closer(c: char) -> bool {
    matches!(c, ')' | ']' | '"' | '\'' | '\u{201d}' | '\u{2019}')
}

/// A sentence ends at `.`, `!`, or `?` followed by whitespace, or at the
/// end of the text. A closing `)` or quote may sit between the mark and
/// the whitespace. The next sentence may start with a digit or a lowercase
/// letter. A mark with no whitespace after it stays inside the sentence,
/// so llama.cpp, URLs, and 21.95 stay whole.
fn split_sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, '.' | '!' | '?') {
            let mut j = i + 1;
            while j < chars.len() && is_closer(chars[j]) {
                j += 1;
            }
            if j >= chars.len() || chars[j].is_whitespace() {
                for ch in chars.iter().take(j).skip(i) {
                    cur.push(*ch);
                }
                let sentence = cur.trim().to_string();
                if !sentence.is_empty() {
                    out.push(sentence);
                }
                cur.clear();
                i = j;
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                continue;
            }
        }
        cur.push(c);
        i += 1;
    }
    let sentence = cur.trim().to_string();
    if !sentence.is_empty() {
        out.push(sentence);
    }
    out
}

/// Sentence-sized check. A date or a link in another sentence does not
/// license a comparison in this one.
pub fn comparison_errors(text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    for sentence in split_sentences(&readable_text(text)) {
        if let Some(kind) = comparison_problem(&sentence) {
            errors.push(format!("{kind} in {sentence:?}"));
        }
    }
    errors
}

/// Published papers and receipts, by exact source path and by the page that renders them.
/// A path hits this list only when it equals an entry or ends with `/` + that entry.
pub const PROSE_EXEMPT_PATHS: &[&str] = &[
    "public/data/engineer.json",
    "public/engineer/papers/the-continuation-test.md",
    "crates/site-builder/data/omega.json",
    "crates/site-builder/data/models.json",
    "public/research/aien-research-plan.md",
    "public/research/computing-machinery-and-understanding-emergence/index.html",
    "public/research/the-stapleton-doctrine/index.html",
    "public/call-to-humanity/index.html",
    "data/engineer.json",
    "engineer/papers/the-continuation-test.md",
    "research/aien-research-plan.md",
    "research/computing-machinery-and-understanding-emergence/index.html",
    "research/the-stapleton-doctrine/index.html",
    "call-to-humanity/index.html",
    "omega/index.html",
    "omega.json",
    "/engineer/",
    "engineer/index.html",
    "/engineer/path/",
    "engineer/path/index.html",
    "/engineer/what-i-learned/",
    "engineer/what-i-learned/index.html",
    "/engineer/philosophy/",
    "engineer/philosophy/index.html",
    "/engineer/software/",
    "engineer/software/index.html",
    "/engineer/evidence/",
    "engineer/evidence/index.html",
    "/engineer/atlas/",
    "engineer/atlas/index.html",
    "/engineer/scholar/",
    "engineer/scholar/index.html",
    "/engineer/interest/",
    "engineer/interest/index.html",
    "/engineer/donate/",
    "engineer/donate/index.html",
    "/engineer/glossary/",
    "engineer/glossary/index.html",
    "/engineer/symphony/",
    "engineer/symphony/index.html",
    "/engineer/symphony/first/",
    "engineer/symphony/first/index.html",
    "/engineer/symphony/workflow/",
    "engineer/symphony/workflow/index.html",
    "/engineer/symphony/map/",
    "engineer/symphony/map/index.html",
    "/engineer/works/",
    "engineer/works/index.html",
    "/engineer/works/aegis/",
    "engineer/works/aegis/index.html",
    "/engineer/works/aien/",
    "engineer/works/aien/index.html",
    "/engineer/works/atlas-harness/",
    "engineer/works/atlas-harness/index.html",
    "/engineer/works/atlas-symphony/",
    "engineer/works/atlas-symphony/index.html",
    "/engineer/works/beltbrain/",
    "engineer/works/beltbrain/index.html",
    "/engineer/works/chatgpt-vs-aien/",
    "engineer/works/chatgpt-vs-aien/index.html",
    "/engineer/works/cudaplusplus/",
    "engineer/works/cudaplusplus/index.html",
    "/engineer/works/defiantmob/",
    "engineer/works/defiantmob/index.html",
    "/engineer/works/dhg-hive/",
    "engineer/works/dhg-hive/index.html",
    "/engineer/works/endless-symphony/",
    "engineer/works/endless-symphony/index.html",
    "/engineer/works/foundations-2025/",
    "engineer/works/foundations-2025/index.html",
    "/engineer/works/mobdev-panel-arena/",
    "engineer/works/mobdev-panel-arena/index.html",
    "/engineer/works/research/",
    "engineer/works/research/index.html",
    "/engineer/works/the-continuation-test/",
    "engineer/works/the-continuation-test/index.html",
    "/engineer/works/chatgpt-vs-aien/infographic/",
    "engineer/works/chatgpt-vs-aien/infographic/index.html",
];

pub fn is_prose_exempt(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    PROSE_EXEMPT_PATHS.iter().any(|entry| {
        if entry.starts_with('/') {
            norm == *entry || norm.ends_with(entry)
        } else {
            norm == *entry || norm.ends_with(&format!("/{entry}"))
        }
    })
}

/// Same check as `comparison_errors`, except a path on `PROSE_EXEMPT_PATHS` is skipped.
pub fn comparison_errors_at(path: &str, text: &str) -> Vec<String> {
    if is_prose_exempt(path) {
        Vec::new()
    } else {
        comparison_errors(text)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub as_of: String,
    pub source: String,
    pub judging: String,
    pub ceilings: Vec<Ceiling>,
    pub machines: Vec<Machine>,
    pub speeds: Vec<Speed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ceiling {
    pub id: String,
    pub model: String,
    pub machine: String,
    pub tok_s: f64,
    pub how: String,
    pub link: String,
    pub receipt: String,
    pub branch: String,
    pub on_main: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub link: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub role: String,
    pub today: String,
    pub evidence_date: String,
    pub evidence: String,
    pub link: String,
    pub branch: String,
    pub on_main: bool,
    pub more: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speed {
    pub id: String,
    pub what: String,
    pub model: String,
    pub machine: String,
    pub engine: String,
    pub value: String,
    pub detail: String,
    pub tok_s: Option<f64>,
    pub ceiling: String,
    pub previous: String,
    pub date: String,
    pub link: String,
    pub receipt: String,
    pub branch: String,
    pub on_main: bool,
    pub verdict: String,
    pub note: String,
}

impl Data {
    pub fn ceiling(&self, id: &str) -> Option<&Ceiling> {
        if id.is_empty() {
            return None;
        }
        self.ceilings.iter().find(|c| c.id == id)
    }
}

fn has_dash(s: &str) -> bool {
    s.contains('\u{2014}') || s.contains('\u{2013}')
}

fn check_text(what: &str, s: &str, errors: &mut Vec<String>) {
    if has_dash(s) {
        errors.push(format!("{what}: em or en dash in {s:?}"));
    }
    for problem in comparison_errors(s) {
        errors.push(format!("{what}: {problem}"));
    }
}

fn check_link(what: &str, link: &str, errors: &mut Vec<String>) {
    if !link.starts_with("https://") {
        errors.push(format!("{what}: link must be a public https URL, got {link:?}"));
    }
}

/// Parse and validate. Every error is listed, not just the first.
pub fn parse(raw: &str) -> Result<Data, String> {
    let d: Data = serde_json::from_str(raw).map_err(|e| format!("machines.json: {e}"))?;
    let mut errors = Vec::new();
    check_text("as_of", &d.as_of, &mut errors);
    check_text("source", &d.source, &mut errors);
    check_text("judging", &d.judging, &mut errors);
    if d.machines.is_empty() {
        errors.push("no machines".into());
    }
    if d.speeds.is_empty() {
        errors.push("no speeds".into());
    }
    for c in &d.ceilings {
        let what = format!("ceiling {}", c.id);
        check_link(&what, &c.link, &mut errors);
        for s in [&c.model, &c.machine, &c.how, &c.receipt, &c.branch] {
            check_text(&what, s, &mut errors);
        }
        if c.tok_s.is_nan() || c.tok_s <= 0.0 {
            errors.push(format!("{what}: ceiling must be positive"));
        }
    }
    for m in &d.machines {
        let what = format!("machine {}", m.id);
        check_link(&what, &m.link, &mut errors);
        for s in [&m.name, &m.detail, &m.role, &m.today, &m.evidence_date, &m.evidence, &m.branch] {
            check_text(&what, s, &mut errors);
        }
        if m.evidence_date.len() != 10 {
            errors.push(format!("{what}: evidence_date must be YYYY-MM-DD"));
        }
        for l in &m.more {
            check_link(&what, &l.link, &mut errors);
            check_text(&what, &l.label, &mut errors);
        }
    }
    for s in &d.speeds {
        let what = format!("speed {}", s.id);
        check_link(&what, &s.link, &mut errors);
        for t in [
            &s.what, &s.model, &s.machine, &s.engine, &s.value, &s.detail, &s.previous, &s.date,
            &s.receipt, &s.branch, &s.verdict, &s.note,
        ] {
            check_text(&what, t, &mut errors);
        }
        if !s.ceiling.is_empty() && d.ceiling(&s.ceiling).is_none() {
            errors.push(format!("{what}: unknown ceiling {:?}", s.ceiling));
        }
        if !s.ceiling.is_empty() && s.tok_s.is_none() {
            errors.push(format!("{what}: a ceiling needs a numeric tok_s"));
        }
        if let (Some(t), Some(c)) = (s.tok_s, d.ceiling(&s.ceiling)) {
            if t > c.tok_s {
                errors.push(format!("{what}: {t} tok/s is above its ceiling {}", c.tok_s));
            }
        }
        if s.on_main && s.branch != "main" {
            errors.push(format!("{what}: on_main is true but branch is {:?}", s.branch));
        }
    }
    let ids: Vec<&str> = d.speeds.iter().map(|s| s.id.as_str()).collect();
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            errors.push(format!("speed {id}: duplicate id"));
        }
    }
    if errors.is_empty() {
        Ok(d)
    } else {
        Err(errors.join("\n"))
    }
}

static DATA: OnceLock<Data> = OnceLock::new();

/// The validated data. A bad file stops the build.
pub fn data() -> &'static Data {
    DATA.get_or_init(|| parse(RAW).unwrap_or_else(|e| panic!("{e}")))
}

/// "0.752" for 7.63 of 10.15.
pub fn fraction(tok_s: f64, ceiling: f64) -> String {
    format!("{:.3}", tok_s / ceiling)
}

/// "22.0 to 22.5 tok/s" as a big number and a small unit (the last word);
/// a value with no space stays whole.
fn big_value(value: &str) -> Markup {
    let (n, unit) = value.rsplit_once(' ').unwrap_or((value, ""));
    html! {
        span class="mx-hero-value mono" { (n) @if !unit.is_empty() { " " span class="mx-unit" { (unit) } } }
    }
}

fn where_badge(on_main: bool, branch: &str) -> Markup {
    html! {
        @if on_main {
            span class="mx-where mx-where-main" { "receipt on main" }
        } @else {
            span class="mx-where mx-where-branch" title=(format!("branch {branch}")) { "branch " (branch) }
        }
    }
}

// ---------------------------------------------------------------- machines

fn machine_card(m: &Machine) -> Markup {
    html! {
        article class="mx-card" id=(format!("machine-{}", m.id)) {
            header class="mx-card-head" {
                h3 class="mx-name" { (m.name) }
                p class="mx-detail" { (m.detail) }
            }
            p class="mx-role" { (m.role) }
            dl class="mx-facts" {
                dt { "Today" }
                dd { (m.today) }
                dt { "Last evidence" }
                dd {
                    span class="mx-date mono" { (m.evidence_date) }
                    " " (m.evidence)
                }
            }
            footer class="mx-card-foot" {
                (where_badge(m.on_main, &m.branch))
                @if !m.more.is_empty() {
                    span class="mx-more" {
                        @for (i, l) in m.more.iter().enumerate() {
                            @if i > 0 { " · " }
                            a href=(l.link) target="_blank" rel="noopener noreferrer" { (l.label) }
                        }
                    }
                }
            }
            (foot(&m.link, Target::Evidence))
        }
    }
}

/// The grid of machine cards, shared by the homepage and /machine/.
pub fn render_machine_cards(d: &Data) -> Markup {
    html! {
        div class="mx-grid" {
            @for m in &d.machines { (machine_card(m)) }
        }
    }
}

/// Homepage section: one card per machine.
pub fn render_machines_section() -> Markup {
    let d = data();
    html! {
        section id="machines" class="mx-section" aria-labelledby="machines-title" {
            div class="container" {
                div class="mx-head" {
                    p class="kicker" { "The machines" }
                    h2 id="machines-title" class="mx-h2" { "Five machines, one project" }
                    p class="mx-lead" {
                        "Aien is built and judged on a DGX Spark, runs bare metal on an Intel iMac and a Raspberry Pi 5, passes every kernel gate in QEMU first, and is being built for a Windows PC with an RTX 5070. Each card says what runs on that machine today, when the last evidence landed, and where the receipt is."
                    }
                }
                (render_machine_cards(d))
                p class="mx-stamp" {
                    "Inventory date " span class="mono" { (d.as_of) } " is when this list was taken. It is not the date of each card. Each card carries its own evidence date. A machine without a public receipt is not on this list."
                }
            }
        }
    }
}

/// The compact version for the top of /machine/: the same cards under a
/// shorter introduction, so the page names real hardware before the theory.
pub fn render_real_machines() -> Markup {
    let d = data();
    html! {
        section id="real-machines" class="narr-section mx-on-narr" aria-labelledby="real-machines-heading" {
            h2 id="real-machines-heading" { "The bodies of one machine" }
            p {
                "One machine, five physical bodies. Aien is one system whose work moves between them: a DGX Spark for development and judgment, an Intel iMac and a Raspberry Pi 5 that run the kernel bare metal, QEMU for every gate before hardware, and a Windows PC with an RTX 5070 as the next target. Each card says what runs on that body today, the last evidence for it, and where the receipt is. The loop below is what they are for."
            }
            (render_machine_cards(d))
            p class="mx-stamp" {
                "Inventory date " span class="mono" { (d.as_of) } " is when this list was taken. It is not the date of each card. Each card carries its own evidence date."
            }
            p class="narr-more" {
                a href="/#speed" { "The speeds measured on them, with receipts, are on the homepage." }
            }
        }
    }
}

// ---------------------------------------------------------------- speeds

fn speed_row(d: &Data, s: &Speed) -> Markup {
    let ceiling = d.ceiling(&s.ceiling);
    html! {
        tr class="mx-row" id=(format!("speed-{}", s.id)) {
            td class="mx-td-what" data-label="What" {
                span class="mx-what" { (s.what) }
                span class="mx-model" { (s.model) }
            }
            td class="mx-td-machine" data-label="Machine and engine" {
                (s.machine)
                span class="mx-sub" { (s.engine) }
            }
            td class="mx-td-value" data-label="Measured" {
                span class="mx-value mono" { (s.value) }
                @if !s.detail.is_empty() { span class="mx-sub" { (s.detail) } }
                @if !s.verdict.is_empty() { span class="mx-verdict" { (s.verdict) } }
            }
            td class="mx-td-ceiling" data-label="Ceiling" {
                @if let (Some(t), Some(c)) = (s.tok_s, ceiling) {
                    span class="mono" { (fraction(t, c.tok_s)) }
                    span class="mx-sub" {
                        "of "
                        a href=(c.link) target="_blank" rel="noopener noreferrer" { (format!("{:.2} tok/s", c.tok_s)) }
                    }
                } @else {
                    span class="mx-sub mx-none" { "none shown" }
                }
            }
            td class="mx-td-prev" data-label="My previous run" {
                @if s.previous.is_empty() {
                    span class="mx-sub mx-none" { "first of its kind" }
                } @else {
                    (s.previous)
                }
            }
            td class="mx-td-date mono" data-label="Date" { (s.date) }
            td class="mx-td-receipt" data-label="Receipt" {
                span class="mx-receipt" { (s.receipt) }
                (where_badge(s.on_main, &s.branch))
                @if !s.note.is_empty() { span class="mx-sub" { (s.note) } }
                (foot(&s.link, Target::Evidence))
            }
        }
    }
}

/// Homepage section: the speed table, the 35B end to end first.
pub fn render_speeds_section() -> Markup {
    let d = data();
    let first = d.speeds.first().expect("at least one speed row");
    let first_ceiling = d.ceiling(&first.ceiling);
    html! {
        section id="speed" class="mx-section mx-speed" aria-labelledby="speed-title" {
            div class="container" {
                div class="mx-head" {
                    p class="kicker" { "Speed, with receipts" }
                    h2 id="speed-title" class="mx-h2" { "The 35B end to end, on a test server" }
                    p class="mx-lead" {
                        "Qwen3.6-35B-A3B, a mixture-of-experts with about 3B parameters active per token, answers through my own C GPU stack, on the Spark. The number below is end-to-end wall time from a test server running a branch build, prompt reading included. Chip time for the same model is decode only. The gap between chip time and this end-to-end time comes from prompt reading. The full table holds the 27B chip-time rows, the engine alone, the smaller Rig engine and the long-context cost, each labelled end to end or chip time, and every row links to the folder its number came from."
                    }
                }
                div class="mx-hero" {
                    div class="mx-hero-n" {
                        (big_value(&first.value))
                        span class="mx-hero-l" { (first.model) " · " (first.engine) " · " (first.machine) }
                    }
                    @if let (Some(t), Some(c)) = (first.tok_s, first_ceiling) {
                        div class="mx-hero-c" {
                            span class="mx-hero-value mono mx-hero-frac" { (fraction(t, c.tok_s)) }
                            span class="mx-hero-l" {
                                "of the " a href=(c.link) target="_blank" rel="noopener noreferrer" { (format!("{:.2} tok/s", c.tok_s)) " memory ceiling" }
                            }
                        }
                    }
                    @if !first.previous.is_empty() {
                        div class="mx-hero-p mx-hero-prev" {
                            (big_value(first.previous.split(',').next().unwrap_or("")))
                            span class="mx-hero-l" { "my previous run" }
                        }
                    }
                    a class="mx-hero-r" href=(first.link) target="_blank" rel="noopener noreferrer" {
                        "Receipt: " (first.receipt) " " (where_badge(first.on_main, &first.branch))
                    }
                }
                // One number leads; every other row stays one click away in a
                // native disclosure, so the section reads as one idea.
                details class="mx-receipts" {
                    summary class="mx-receipts-summary" { "All " (d.speeds.len()) " speed receipts, ceilings and how they are judged" }
                    div class="mx-table-wrap" {
                        table class="mx-table" {
                            thead {
                                tr {
                                    th { "What" }
                                    th { "Machine and engine" }
                                    th { "Measured" }
                                    th { "Ceiling" }
                                    th { "My previous run" }
                                    th { "Date" }
                                    th { "Receipt" }
                                }
                            }
                            tbody {
                                @for s in &d.speeds { (speed_row(d, s)) }
                            }
                        }
                    }
                    p class="mx-judging" { (d.judging) }
                    @for c in &d.ceilings {
                        p class="mx-stamp" {
                            "Ceiling for " (c.model) " on the " (c.machine) ": " span class="mono" { (format!("{:.2} tok/s", c.tok_s)) } ". " (c.how) " "
                            a href=(c.link) target="_blank" rel="noopener noreferrer" { (c.receipt) }
                            " " (where_badge(c.on_main, &c.branch))
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

    /// A speed row whose receipt is chip time: a chip folder, milliseconds per
    /// token, or GPU time per token. End-to-end wall time and cold time to
    /// first token are not chip time.
    fn is_chip_time(s: &Speed) -> bool {
        let blob = format!("{} {} {}", s.receipt, s.detail, s.value).to_ascii_lowercase();
        blob.contains("chip") || blob.contains("ms per token") || blob.contains("gpu time")
    }

    #[test]
    fn real_data_parses_and_validates() {
        let d = parse(RAW).expect("machines.json must validate");
        assert_eq!(d.machines.len(), 5, "five machines: Spark, iMac, Pi 5, QEMU, RTX 5070 PC");
        for want in ["Spark", "iMac", "Raspberry Pi 5", "QEMU", "RTX 5070"] {
            assert!(d.machines.iter().any(|m| m.name.contains(want)), "machine {want} missing");
        }
        assert!(d.speeds.first().unwrap().model.starts_with("Qwen3.6-35B-A3B"), "the 35B end to end comes first");
        let spark = d.machines.iter().find(|m| m.id == "spark").unwrap();
        assert_eq!(spark.evidence_date, "2026-10-21");
        assert!(spark.today.contains("test server") && spark.today.contains("branch build"), "{}", spark.today);
        assert!(spark.today.contains("bin-16b0e6f8") && spark.today.contains("21.95") && spark.today.contains("22.46"), "{}", spark.today);
        assert!(!spark.today.contains("aien-serve-http"), "the Spark card must not say the receipt is the live server: {}", spark.today);
        assert!(!spark.today.contains("qwen3.6-35b-a3b-fp8"), "the q36 receipt does not name that id: {}", spark.today);
        assert!(!spark.evidence.to_ascii_lowercase().contains("qwen3.6-35b-a3b-fp8"));
        assert!(!spark.today.contains("Qwen3.8-27B"), "the Spark today line must not say it serves the 27B");
        assert!(d.speeds.first().unwrap().what.starts_with("End to end"));
        assert!(d.speeds.first().unwrap().what.contains("test server") && d.speeds.first().unwrap().what.contains("branch build"));
        assert!(d.speeds.first().unwrap().engine.contains("Omega"));
        assert!(d.judging.contains("prompt reading"));
        assert!(!d.judging.to_ascii_lowercase().contains("without the server"));
        for s in d.speeds.iter().filter(|s| is_chip_time(s)) {
            assert!(s.what.to_ascii_lowercase().contains("chip time"), "{}: chip-time row must say so ({})", s.id, s.what);
        }
    }

    #[test]
    fn every_row_has_a_public_link() {
        let d = data();
        for m in &d.machines {
            assert!(m.link.starts_with("https://github.com/aien-dev/"), "{}: {}", m.id, m.link);
            for l in &m.more {
                assert!(l.link.starts_with("https://github.com/aien-dev/"), "{}: {}", m.id, l.link);
            }
        }
        for s in &d.speeds {
            assert!(s.link.starts_with("https://github.com/aien-dev/"), "{}: {}", s.id, s.link);
        }
        for c in &d.ceilings {
            assert!(c.link.starts_with("https://github.com/aien-dev/"), "{}: {}", c.id, c.link);
        }
    }

    #[test]
    fn fraction_matches_the_inventory() {
        assert_eq!(fraction(7.63, 10.15), "0.752");
        assert_eq!(fraction(7.657, 10.15), "0.754");
    }

    #[test]
    fn rejects_missing_link_dashes_and_comparisons() {
        let no_link = RAW.replacen(
            "\"link\": \"https://github.com/aien-dev/aien-sovereign-core-mv/tree/qwen38/serve-fast/evidence/qwen38-serve-fast-20261019\"",
            "\"link\": \"\"",
            1,
        );
        assert!(parse(&no_link).unwrap_err().contains("public https URL"));
        let dashed = RAW.replacen("\"what\": \"Live server", "\"what\": \"Live \u{2014} server", 1);
        assert!(parse(&dashed).unwrap_err().contains("em or en dash"));
        let compared = RAW.replacen("\"note\": \"Nothing has run on the 5070 yet.\"", "\"note\": \"2x faster than llama.cpp\"", 1);
        let err = parse(&compared).unwrap_err();
        assert!(err.contains("unsourced comparison"), "{err}");
        let honest = RAW.replacen(
            "\"note\": \"Nothing has run on the 5070 yet.\"",
            "\"note\": \"On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (https://github.com/aien-dev/sovereign-intelligence).\"",
            1,
        );
        assert!(parse(&honest).is_ok(), "an honest dated sourced comparison is allowed");
        let over = RAW.replacen(
            "\"note\": \"Nothing has run on the 5070 yet.\"",
            "\"note\": \"A replacement for CUDA, dated 2026-10-21, receipt https://example.com/r.\"",
            1,
        );
        let err = parse(&over).unwrap_err();
        assert!(err.contains("overclaim"), "{err}");
        let neighbor = RAW.replacen(
            "\"note\": \"Nothing has run on the 5070 yet.\"",
            "\"note\": \"Measured on 2026-10-21 (https://example.com/bench). CUDA is slower than Omega.\"",
            1,
        );
        let err = parse(&neighbor).unwrap_err();
        assert!(err.contains("unsourced comparison"), "{err}");
        let bare = RAW.replacen(
            "\"note\": \"Nothing has run on the 5070 yet.\"",
            "\"note\": \"On 2026-10-21 this beats vLLM (receipt).\"",
            1,
        );
        let err = parse(&bare).unwrap_err();
        assert!(err.contains("unsourced comparison"), "{err}");
        let above = RAW.replacen("\"tok_s\": 7.63,", "\"tok_s\": 70.63,", 1);
        assert!(parse(&above).unwrap_err().contains("above its ceiling"));
        let extra = RAW.replacen("\"as_of\"", "\"surprise\": 1, \"as_of\"", 1);
        assert!(parse(&extra).is_err(), "unknown fields must be refused");
    }

    #[test]
    fn sections_render_clean() {
        for page in [
            render_machines_section().into_string(),
            render_speeds_section().into_string(),
            render_real_machines().into_string(),
        ] {
            assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'), "dash on the page");
            let problems = comparison_errors(&page);
            assert!(problems.is_empty(), "comparison problem on the page: {problems:?}");
        }
        let speeds = render_speeds_section().into_string();
        assert!(speeds.contains("7.63 tok/s"));
        assert!(speeds.contains("0.752"));
        assert!(speeds.contains("10.15 tok/s"));
        assert!(speeds.contains("id=\"speed-27b-live\""));
        assert!(speeds.contains("id=\"speed-35b-e2e\""));
        assert!(speeds.contains("22.0 to 22.5"));
        assert!(speeds.contains("test server running a branch build"));
        assert!(speeds.contains("mx-where-main\">receipt on main"), "35B row badge");
        assert!(speeds.contains("comes from prompt reading"));
        assert!(!speeds.to_ascii_lowercase().contains("without the server"));
        assert!(!speeds.to_ascii_lowercase().contains("leaves out the server"));
        assert!(speeds.contains(&data().judging));
        let machines = render_machines_section().into_string();
        assert!(machines.contains("mx-where-main\">receipt on main"), "Spark badge");
        for id in ["machine-spark", "machine-imac", "machine-pi5", "machine-qemu", "machine-rig"] {
            assert!(machines.contains(&format!("id=\"{id}\"")), "{id} missing");
        }
        assert_eq!(machines.matches("class=\"box-link\"").count(), 5, "one evidence link per machine");
        assert_eq!(crate::components::boxlink::check_boxes(&machines), Ok(5), "every machine card links to GitHub");
        let rows = data().speeds.len();
        assert_eq!(crate::components::boxlink::check_boxes(&speeds), Ok(rows), "every speed row links to GitHub");
    }

    #[test]
    fn honest_example_passes_the_page_check() {
        let page = "Lead 21.95 tok/s from https://example.com/bench.v2/time.jsonl stayed put. On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (https://github.com/aien-dev/sovereign-intelligence).";
        let problems = comparison_errors(page);
        assert!(problems.is_empty(), "{problems:?}");
        let html = "<p>On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (<a href=\"https://github.com/aien-dev/sovereign-intelligence\">bench</a>).</p>";
        let problems = comparison_errors(html);
        assert!(problems.is_empty(), "{problems:?}");
        let unrelated = comparison_errors("On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (https://example.com/bench).");
        assert!(
            unrelated.iter().any(|e| e.contains("unsourced comparison")),
            "an unrelated https link is not a source: {unrelated:?}"
        );
    }

    #[test]
    fn unsourced_comparisons_fail_on_the_page() {
        let slower = comparison_errors("CUDA is slower than Omega");
        assert!(slower.iter().any(|e| e.contains("unsourced comparison")), "{slower:?}");
        let neighbor = comparison_errors(
            "Measured on 2026-10-21 (https://example.com/bench). CUDA is slower than Omega.",
        );
        assert!(
            neighbor.iter().any(|e| e.contains("unsourced comparison") && e.to_ascii_lowercase().contains("slower than")),
            "{neighbor:?}"
        );
        assert!(
            !neighbor.iter().any(|e| e.contains("Measured on")),
            "the dated sentence is not the failure: {neighbor:?}"
        );
        let bare = comparison_errors("On 2026-10-21 this beats vLLM (receipt).");
        assert!(bare.iter().any(|e| e.contains("unsourced comparison")), "{bare:?}");
        let published = comparison_errors("On 2026-10-21 this was faster than the published figure.");
        assert!(published.iter().any(|e| e.contains("unsourced comparison")), "{published:?}");
        assert!(comparison_problem("Slower than I want.").is_none());
        assert!(comparison_problem("On 2026-02-31 this was faster than CUDA (https://example.com/r).").is_some());
        assert!(comparison_problem(
            "On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (https://example.com/bench)."
        ).is_some());
        assert!(comparison_problem(
            "On 2026-10-21 this run was faster than llama.cpp's published 40 tok/s (https://github.com/ggml-org/llama.cpp)."
        ).is_none());
        assert!(comparison_problem(
            "On 2026-10-21 this was faster than vLLM (https://docs.vllm.ai/en/latest/)."
        ).is_none());
    }

    #[test]
    fn a_sentence_ends_at_a_mark_then_whitespace_whatever_follows() {
        for mark in [".", "!", "?"] {
            let page = format!(
                "Measured on 2026-10-21 (https://example.com/bench){mark} vLLM is slower than Omega."
            );
            let problems = comparison_errors(&page);
            assert!(
                problems.iter().any(|e| e.contains("unsourced comparison") && e.contains("slower than")),
                "{mark}: {problems:?}"
            );
            assert!(
                !problems.iter().any(|e| e.contains("Measured on")),
                "the dated sentence is not the failure ({mark}): {problems:?}"
            );
        }
        let closer = "Measured on 2026-10-21 (https://example.com/bench.) vLLM is slower than Omega.";
        let problems = comparison_errors(closer);
        assert!(
            problems.iter().any(|e| e.contains("slower than") && !e.contains("Measured on")),
            "a closing paren between the mark and the space still ends the sentence: {problems:?}"
        );
        let quoted = "Measured on 2026-10-21 (https://example.com/bench).\" vLLM is slower than Omega.";
        let problems = comparison_errors(quoted);
        assert!(
            problems.iter().any(|e| e.contains("slower than")),
            "a closing quote between the mark and the space still ends the sentence: {problems:?}"
        );
        let digit = "Measured on 2026-10-21 (https://example.com/bench). 2x vLLM was the next sentence.";
        let problems = comparison_errors(digit);
        assert!(
            problems.iter().any(|e| e.contains("unsourced comparison") && e.contains("2x")),
            "a next sentence may start with a digit: {problems:?}"
        );
        assert!(!problems.iter().any(|e| e.contains("Measured on")), "{problems:?}");
        let whole = comparison_errors("Lead 21.95 tok/s from llama.cpp and https://example.com/bench.v2/time.jsonl stayed put.");
        assert!(whole.is_empty(), "no whitespace after the mark, so the token stays whole: {whole:?}");
    }

    #[test]
    fn every_comparison_term_fails_without_a_date_and_link_and_passes_with_both() {
        let terms = [
            "beat vLLM",
            "beats vLLM",
            "beating vLLM",
            "beaten by vLLM",
            "outperforms vLLM",
            "outpaces vLLM",
            "outpaced vLLM",
            "surpasses vLLM",
            "exceeds vLLM",
            "exceeded vLLM",
            "rivals vLLM",
            "better than vLLM",
            "worse than vLLM",
            "faster than vLLM",
            "slower than vLLM",
            "on par with vLLM",
            "on a par with vLLM",
            "level with vLLM",
            "ahead of vLLM",
            "same speed as vLLM",
            "as fast as vLLM",
            "2x vLLM",
            "2× faster",
            "3 times vLLM",
            "20.3-fold vLLM",
            "a factor of 1.80",
            "twice vLLM",
            "double vLLM",
            "half vLLM",
            "12% faster",
            "1.8x.",
            "(2x)",
            "the fastest vLLM",
            "best vLLM",
            "comparable to vLLM",
            "competitive with vLLM",
            "within 5% of vLLM",
            "vLLM vs Omega",
        ];
        for term in terms {
            let bare = format!("This run will {term}.");
            let problems = comparison_errors(&bare);
            assert!(
                problems.iter().any(|e| e.contains("unsourced comparison")),
                "{term} must fail: {problems:?}"
            );
            let honest = if term.contains(['.', '!', '?']) {
                format!("On 2026-10-21 (https://github.com/aien-dev/sovereign-intelligence) this was {term}")
            } else {
                format!("On 2026-10-21 this was {term} (https://github.com/aien-dev/sovereign-intelligence).")
            };
            let problems = comparison_errors(&honest);
            assert!(problems.is_empty(), "{term} with a real date and a link must pass: {problems:?}");
        }
        assert!(comparison_problem("The quickest walkthrough of what exists today.").is_none());
        assert!(comparison_problem("The drive behind the grades.").is_none());
        assert!(comparison_problem("answers matching the shipped reference.").is_none());
        assert!(comparison_problem("This is not directly comparable to TOP500 scores.").is_none());
        assert!(comparison_problem("lowercase 0x and lowercase digits.").is_none());
        assert!(comparison_problem("P vs NP is open.").is_none());
        assert!(comparison_problem("The frame is 1920x1080 and the hex is 0xff.").is_none());
        let shouted = comparison_errors("BEATS vLLM. Faster Than the published figure.");
        assert_eq!(shouted.len(), 2, "word forms are case-insensitive, one sentence each: {shouted:?}");
        assert!(comparison_problem("served_match is an identifier, not a claim.").is_none());
        assert!(comparison_errors("<style>/* beat the memorizer */</style><p>A note.</p>").is_empty());
        assert!(comparison_errors("<pre><code>files match SHA256SUMS. 3x physical.</code></pre>").is_empty());
        assert!(
            !comparison_errors("<code>2x vLLM</code>").is_empty(),
            "inline code is prose"
        );
        assert!(
            !comparison_errors("<p data-verbatim>The gb10 is 2x vLLM.</p>").is_empty(),
            "data-verbatim alone is not an opt-out"
        );
        assert!(comparison_errors("<p class=\"now-note\" data-verbatim>The gb10 is 2x vLLM.</p><p>A note.</p>").is_empty());
        assert!(comparison_errors("<p class=\"agents-verbatim\" data-verbatim>aien-serve-http vs vLLM vs llama.cpp</p>").is_empty());
    }

    #[test]
    fn published_papers_are_exempt_by_exact_path() {
        let papers = [
            "public/data/engineer.json",
            "public/engineer/papers/the-continuation-test.md",
            "crates/site-builder/data/omega.json",
            "crates/site-builder/data/models.json",
            "public/research/aien-research-plan.md",
            "public/research/computing-machinery-and-understanding-emergence/index.html",
            "public/research/the-stapleton-doctrine/index.html",
            "public/call-to-humanity/index.html",
        ];
        let claim = "This beats vLLM with no date.";
        for path in papers {
            assert!(is_prose_exempt(path), "{path} missing from the fixed list");
            assert!(comparison_errors_at(path, claim).is_empty(), "{path} was scanned");
        }
        assert!(!is_prose_exempt("public/research/index.html"));
        assert!(!comparison_errors_at("public/research/index.html", claim).is_empty());
        assert!(PROSE_EXEMPT_PATHS.contains(&"public/call-to-humanity/index.html"));
        assert!(!PROSE_EXEMPT_PATHS.contains(&"public/research/index.html"));
    }

    #[test]
    fn omega_and_every_other_page_use_the_shared_check() {
        let omega = crate::components::omega::render_omega_page().into_string();
        let problems = comparison_errors_at("omega/index.html", &omega);
        assert!(problems.is_empty(), "/omega/ is the omega.json page and is exempt: {problems:?}");
        let planted = "Measured on 2026-10-21 (https://example.com/bench). vLLM is slower than Omega.";
        let problems = comparison_errors(planted);
        assert!(
            problems.iter().any(|e| e.contains("unsourced comparison") && e.contains("slower than")),
            "the same function still refuses a planted sentence: {problems:?}"
        );

        for page in crate::components::engineer::pages() {
            assert!(is_prose_exempt(&page.path), "engineer page {} is on the exempt list", page.path);
            assert!(
                comparison_errors_at(&page.path, "This beats vLLM.").is_empty(),
                "{} must be skipped",
                page.path
            );
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        collect_html(&root.join("public"), &mut files);
        collect_html(&root.join("crates/site-builder/templates"), &mut files);
        collect_named(&root.join("om"), "main.html", &mut files);
        assert!(files.len() > 20, "expected the shipped pages, found {}", files.len());
        let mut bad = Vec::new();
        for path in &files {
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let problems = comparison_errors_at(&path.display().to_string(), &text);
            if !problems.is_empty() {
                bad.push(format!(
                    "{}: {}",
                    path.display(),
                    problems.iter().take(4).map(|e| clip(e, 200)).collect::<Vec<_>>().join(" | ")
                ));
            }
        }
        assert!(bad.is_empty(), "pages failed the shared check:\n{}", bad.join("\n"));
        assert!(
            files.iter().any(|p| p.ends_with("om/alpha-page/main.html")),
            "the generated alpha page is scanned, not skipped"
        );
    }

    fn clip(s: &str, n: usize) -> String {
        s.chars().take(n).collect()
    }

    fn collect_html(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_html(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
                out.push(path);
            }
        }
    }

    fn collect_named(dir: &std::path::Path, name: &str, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("vendor") {
                    continue;
                }
                collect_named(&path, name, out);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                out.push(path);
            }
        }
    }
}
