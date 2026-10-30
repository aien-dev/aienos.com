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
/// These are claims on their own. "faster than", "half", "twice", "parity",
/// and "beat" are context-bound: they count only in `context_is_claim`.
pub const COMPARISON_PHRASES: &[&str] = &[
    "better than",
    "worse than",
    "slower than",
    "level with",
    "ahead of",
    "same speed as",
    "as fast as",
    "factor of",
    "comparable to",
    "competitive with",
    "edges out",
];

/// Whole words, case-insensitively. Not stems. A claim on their own.
/// Bare "behind" is ordinary English, so it is not a claim.
/// "match", "ties", "tied with", "equal to" and "on par with" are context-bound,
/// in `TIE_WORDS` and `TIE_PHRASES`: they count beside an outside name, a rival
/// noun, a multiplier, or the "best measured CUDA number" bar.
/// Superlatives ("fastest", "slowest", "quickest", "best", "worst") count only
/// beside a project name, in `has_guarded_word`.
/// "beat", "parity", "trails" and "lags" are context-bound, in `CONTEXT_WORDS`.
pub const COMPARISON_WORDS: &[&str] = &["equals", "outdoes", "tops", "rivals"];

/// Count only when the same sentence also names an outside system, a rival
/// noun, or a number glued to the comparative ("2x faster", "half as fast").
const CONTEXT_PHRASES: &[&str] = &[
    "faster than",
    "quicker than",
    "cheaper than",
    "more efficient than",
];

const CONTEXT_WORDS: &[&str] = &[
    "half", "twice", "double", "parity", "trails", "lags",
    "beat", "beats", "beating", "beaten",
];

/// Tie words. Same targets as the other context words, plus a multiplier
/// or the bar phrase "best measured CUDA number". A different word in a
/// CUDA bar sentence does not stay on the exact-sentence list.
const TIE_WORDS: &[&str] = &["match", "matches", "matching", "ties"];

const TIE_PHRASES: &[&str] = &["tied with", "equal to", "on par with", "on a par with"];

/// Connectors. They count when the same sentence names a rival.
const CONNECTORS: &[&str] = &[
    "versus",
    "compared with",
    "compared to",
    "relative to",
    "speedup over",
];

/// A generic rival. Not a project name. Context-bound words count beside one of these.
const RIVAL_NOUNS: &[&str] = &[
    "competitors",
    "the competition",
    "alternatives",
    "other systems",
    "other frameworks",
    "other stacks",
    "other compilers",
    "other runtimes",
    "the industry",
    "state of the art",
    "sota",
    "incumbent",
    "leader",
    "leading",
    "market leader",
    "gpu stack",
    "gpu stacks",
    "the field",
    "everyone else",
    "existing tools",
    "rival",
];

/// A word that starts with one of these, case-insensitively: outperforms,
/// outpaced, surpasses, exceeded. "rival" is a noun in `RIVAL_NOUNS`, not a
/// stem: "holds back every rival" is not a comparison. The verb "rivals" is
/// a comparison word on its own.
pub const COMPARISON_STEMS: &[&str] = &["outperform", "outpace", "surpass", "exceed"];

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

/// Outside systems. "cuda" does not match when "++" follows it: CUDA++ is this project.
/// "nvidia" is an outside system only when the sentence compares against it
/// (`nvidia_is_target`). A descriptive mention ("NVIDIA GPUs", "no NVIDIA software")
/// does not, by itself, turn "half" or "parity" into a claim. Superlatives and
/// "vs" still see every token here, including a descriptive NVIDIA.
const PROJECT_TOKENS: &[&str] = &[
    "cuda", "nvidia", "vllm", "llama.cpp", "llama", "tensorrt", "pytorch", "triton", "cudnn",
    "chatgpt", "openai", "gemini", "claude", "tgi", "sglang", "cutlass",
    "ollama", "mlx", "rocm", "groq",
];

/// Multi-word rival names. "onnx runtime" is not a single token.
const PROJECT_PHRASES: &[&str] = &["onnx runtime"];

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

/// A project token on a word boundary. "cuda" followed by "++" is CUDA++, this project, and does not count.
fn token_present(lower: &str, token: &str) -> bool {
    let mut from = 0;
    while let Some(rel) = lower[from..].find(token) {
        let i = from + rel;
        let before_ok = i == 0 || !is_word_char(lower[..i].chars().next_back().unwrap());
        let after = i + token.len();
        let after_ok = after >= lower.len() || !is_word_char(lower[after..].chars().next().unwrap());
        let cuda_plus = token == "cuda" && lower[after..].starts_with("++");
        if before_ok && after_ok && !cuda_plus {
            return true;
        }
        from = i + token.len().max(1);
    }
    false
}

fn word_ending_at(lower: &str, at: usize) -> Option<(usize, &str)> {
    let trimmed = lower[..at].trim_end_matches(|c: char| !is_word_char(c));
    if trimmed.is_empty() {
        return None;
    }
    let start = trimmed
        .rfind(|c: char| !is_word_char(c))
        .map(|i| i + trimmed[i..].chars().next().unwrap().len_utf8())
        .unwrap_or(0);
    Some((start, &trimmed[start..]))
}

/// NVIDIA is the thing being compared, not a description of hardware or a missing stack.
/// "faster than NVIDIA", "parity with NVIDIA", "twice NVIDIA", and "half of NVIDIA" count.
/// "NVIDIA GPUs", "no NVIDIA software", and "with no NVIDIA user-space" do not.
fn nvidia_is_target(lower: &str) -> bool {
    const MARKERS: &[&str] = &[
        "than", "with", "versus", "against", "vs", "of",
        "twice", "double", "half",
        "beat", "beats", "beating", "beaten",
        "parity", "trails", "lags",
        "faster", "quicker", "cheaper",
    ];
    let mut from = 0;
    while let Some(rel) = lower[from..].find("nvidia") {
        let i = from + rel;
        let before_ok = i == 0 || !is_word_char(lower[..i].chars().next_back().unwrap());
        let after = i + "nvidia".len();
        let after_ok = after >= lower.len() || !is_word_char(lower[after..].chars().next().unwrap());
        if before_ok && after_ok {
            let mut word = word_ending_at(lower, i);
            if matches!(word, Some((_, "the" | "a" | "an"))) {
                if let Some((start, _)) = word {
                    word = word_ending_at(lower, start);
                }
            }
            if let Some((_, w)) = word {
                if MARKERS.contains(&w) {
                    return true;
                }
            }
        }
        from = i + "nvidia".len();
    }
    false
}

/// A comparison word other than "half", "twice" or "double". Those three stay
/// descriptive in "GPU half" and do not, by themselves, make a nearby NVIDIA a rival.
fn comparison_word_activates_nvidia(lower: &str) -> bool {
    const WORDS: &[&str] = &[
        "parity", "trails", "lags",
        "beat", "beats", "beating", "beaten",
        "match", "matches", "matching", "ties",
        "slower", "faster", "quicker", "cheaper",
    ];
    if lower.split(|c: char| !is_word_char(c)).any(|word| {
        WORDS.contains(&word)
            || COMPARISON_WORDS.contains(&word)
            || COMPARISON_STEMS.iter().any(|stem| word.starts_with(stem))
    }) {
        return true;
    }
    if CONTEXT_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
        || TIE_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
        || COMPARISON_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
        || CONNECTORS.iter().any(|phrase| boundary_phrase(lower, phrase))
    {
        return true;
    }
    has_attached_comparative(lower) || has_vs(lower)
}

fn has_outside_system(lower: &str) -> bool {
    if nvidia_is_target(lower) || (token_present(lower, "nvidia") && comparison_word_activates_nvidia(lower)) {
        return true;
    }
    PROJECT_TOKENS.iter().any(|token| *token != "nvidia" && token_present(lower, token))
        || PROJECT_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
}

/// Every rival name, in any position. NVIDIA counts here too. Used when the
/// sentence already has a connector or a `<rival>: Nx` multiplier, so a
/// descriptive NVIDIA sentence with no comparison word never reaches this.
fn names_rival(lower: &str) -> bool {
    PROJECT_TOKENS.iter().any(|token| token_present(lower, token))
        || PROJECT_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
        || has_rival(lower)
}

fn has_rival(lower: &str) -> bool {
    RIVAL_NOUNS.iter().any(|noun| boundary_phrase(lower, noun))
}

fn project_near(lower: &str, at: usize) -> bool {
    let start = lower.floor_char_boundary(at.saturating_sub(48));
    let end = lower.ceil_char_boundary((at + 48).min(lower.len()));
    let window = &lower[start..end];
    PROJECT_TOKENS.iter().any(|token| token_present(window, token))
}

/// Superlatives only when a project name sits nearby.
/// Bare uses ("the quickest walkthrough", "the best way") are not claims.
/// "twice", "double", and "half" are context-bound, not superlatives.
fn has_guarded_word(lower: &str) -> bool {
    const GUARDED: &[&str] = &["best", "worst", "fastest", "slowest", "quickest"];
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

fn is_timezone(word: &str) -> bool {
    matches!(word, "cdt" | "cst" | "edt" | "est" | "mdt" | "mst" | "pdt" | "pst" | "utc" | "gmt")
}

/// `19:3x`, `~19:30` and a trailing timezone are clock tokens. They are not
/// multipliers. A later `3x faster` in the same sentence still counts.
fn is_clock_multiplier(chars: &[char], number_start: usize, after_mark: usize) -> bool {
    if number_start > 0 && chars[number_start - 1] == ':' {
        let mut k = number_start - 1;
        if k > 0 {
            k -= 1;
            if chars[k].is_ascii_digit() {
                let mut digits = 1usize;
                while k > 0 && chars[k - 1].is_ascii_digit() && digits < 2 {
                    k -= 1;
                    digits += 1;
                }
                let before = if k == 0 { None } else { Some(chars[k - 1]) };
                if matches!(before, None | Some('~')) || before.is_some_and(|c| c.is_whitespace()) {
                    return true;
                }
            }
        }
    }
    let mut m = after_mark;
    while m < chars.len() && chars[m] == ' ' {
        m += 1;
    }
    if m >= chars.len() {
        return false;
    }
    let rest: String = chars[m..].iter().collect();
    let word = rest.split(|c: char| !is_word_char(c)).next().unwrap_or("");
    is_timezone(word)
}

fn is_x_tail(c: char) -> bool {
    c.is_whitespace()
        || matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '"' | '\'' | '\u{201d}' | '\u{2019}')
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
                // "ahead of time" is an idiom, not a comparison. "ahead of NVIDIA" still is.
                if *phrase == "ahead of" {
                    let rest = lower[after..].trim_start();
                    if rest.starts_with("time") {
                        let n = "time".len();
                        let boundary = n >= rest.len() || !is_word_char(rest[n..].chars().next().unwrap());
                        if boundary {
                            from = i + phrase.len();
                            continue;
                        }
                    }
                }
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

fn has_context_word(lower: &str) -> bool {
    lower.split(|c: char| !is_word_char(c)).any(|word| CONTEXT_WORDS.contains(&word))
}

fn has_context_phrase(lower: &str) -> bool {
    CONTEXT_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
}

fn adjective_at(rest: &str, words: &[&str]) -> bool {
    words.iter().any(|adj| {
        rest.starts_with(adj)
            && (adj.len() >= rest.len() || !is_word_char(rest[adj.len()..].chars().next().unwrap()))
    })
}

/// "half as fast", "2x faster", "40% cheaper", "N times faster".
/// A bare "2x" or "137 times" is not attached.
fn has_attached_comparative(lower: &str) -> bool {
    const LOOSE: &[&str] = &[
        "faster", "slower", "cheaper", "quicker", "more efficient",
        "fast", "slow", "cheap", "quick", "efficient",
    ];
    for lead in ["half as", "twice as", "double as"] {
        let mut from = 0;
        while let Some(rel) = lower[from..].find(lead) {
            let i = from + rel;
            let before_ok = i == 0 || !is_word_char(lower[..i].chars().next_back().unwrap());
            let after = i + lead.len();
            let after_ok = after >= lower.len() || !is_word_char(lower[after..].chars().next().unwrap());
            if before_ok && after_ok {
                let rest = lower[after..].trim_start_matches(' ');
                if adjective_at(rest, LOOSE) {
                    return true;
                }
            }
            from = i + lead.len().max(1);
        }
    }
    const TIGHT: &[&str] = &["faster", "slower", "cheaper", "quicker", "more efficient"];
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
        let rest_at = |m: usize| chars[m..].iter().collect::<String>();
        if chars.get(k) == Some(&'%') {
            let mut m = k + 1;
            while m < chars.len() && chars[m] == ' ' {
                m += 1;
            }
            if adjective_at(&rest_at(m), TIGHT) {
                return true;
            }
        }
        if k < chars.len() && (chars[k] == 'x' || chars[k] == '\u{00d7}') {
            let mut m = k + 1;
            while m < chars.len() && chars[m] == ' ' {
                m += 1;
            }
            if adjective_at(&rest_at(m), TIGHT) {
                return true;
            }
        }
        if k > j {
            let rest = rest_at(k);
            if rest.starts_with("times") {
                let after_times = "times".len();
                let boundary = after_times >= rest.len()
                    || !is_word_char(rest[after_times..].chars().next().unwrap());
                if boundary {
                    let mut m = k + "times".chars().count();
                    while m < chars.len() && chars[m] == ' ' {
                        m += 1;
                    }
                    if adjective_at(&rest_at(m), TIGHT) {
                        return true;
                    }
                }
            }
        }
        i = j;
    }
    false
}

fn has_nx_own(lower: &str) -> bool {
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
        if k < chars.len() && (chars[k] == 'x' || chars[k] == '\u{00d7}') {
            let mut m = k + 1;
            while m < chars.len() && chars[m] == ' ' {
                m += 1;
            }
            let rest: String = chars[m..].iter().collect();
            for phrase in ["its own", "our own"] {
                if rest.starts_with(phrase) {
                    let n = phrase.len();
                    let after_ok = n >= rest.len() || !is_word_char(rest[n..].chars().next().unwrap());
                    if after_ok {
                        return true;
                    }
                }
            }
        }
        i = j;
    }
    false
}

/// "faster than its own frozen starting point" and "20.3x its own baseline".
/// A word such as "frozen" may sit between "own" and the baseline noun.
fn is_self_baseline(lower: &str) -> bool {
    for phrase in ["than its own", "than our own"] {
        let mut from = 0;
        while let Some(rel) = lower[from..].find(phrase) {
            let i = from + rel;
            let before_ok = i == 0 || !is_word_char(lower[..i].chars().next_back().unwrap());
            let after = i + phrase.len();
            let after_ok = after >= lower.len() || !is_word_char(lower[after..].chars().next().unwrap());
            if before_ok && after_ok {
                let end = (after + 80).min(lower.len());
                let window = &lower[after..end];
                if ["starting point", "baseline", "previous"].iter().any(|noun| boundary_phrase(window, noun)) {
                    return true;
                }
            }
            from = i + phrase.len();
        }
    }
    has_nx_own(lower)
}

/// A bare `20x`, `3 times`, or `20.3-fold` names a rival only when that rival
/// follows the number. "2x vLLM" and "20.3-fold vLLM" count. "Claude Max 20x plan"
/// does not: Claude is the product, and 20x is the plan name.
fn bare_multiplier_targets_rival(lower: &str) -> bool {
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
        let mut tail_at: Option<usize> = None;
        if chars.get(j) == Some(&'-') {
            let rest: String = chars[j + 1..].iter().collect();
            if rest.starts_with("fold") {
                let after = "fold".len();
                if after >= rest.len() || !is_word_char(rest[after..].chars().next().unwrap_or(' ')) {
                    tail_at = Some(j + 1 + "fold".chars().count());
                }
            }
        }
        if k < chars.len() && (chars[k] == 'x' || chars[k] == '\u{00d7}') {
            let zero = chars[i..j].iter().all(|c| *c == '0' || *c == '.');
            let hex = zero && chars[k] == 'x';
            if !hex {
                let after = k + 1;
                if (after >= chars.len() || is_x_tail(chars[after])) && !is_clock_multiplier(&chars, i, after) {
                    tail_at = Some(after);
                }
            }
        }
        if k > j {
            let rest: String = chars[k..].iter().collect();
            if rest.starts_with("times") {
                let after = "times".len();
                if after >= rest.len() || !is_word_char(rest[after..].chars().next().unwrap_or(' ')) {
                    tail_at = Some(k + "times".chars().count());
                }
            }
        }
        if let Some(at) = tail_at {
            let tail: String = chars[at..].iter().collect();
            if names_rival(&tail) {
                return true;
            }
        }
        i = j;
    }
    false
}

fn has_tie(lower: &str) -> bool {
    lower.split(|c: char| !is_word_char(c)).any(|word| TIE_WORDS.contains(&word))
        || TIE_PHRASES.iter().any(|phrase| boundary_phrase(lower, phrase))
}

fn has_multiplier(lower: &str) -> bool {
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !(chars[i].is_ascii_digit() && (i == 0 || !is_word_char(chars[i - 1]))) {
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
        if k < chars.len() && (chars[k] == 'x' || chars[k] == '\u{00d7}') {
            let after = k + 1;
            if after >= chars.len() || is_x_tail(chars[after]) {
                let zero = chars[i..j].iter().all(|c| *c == '0' || *c == '.');
                if !(zero && chars[k] == 'x') && !is_clock_multiplier(&chars, i, after) {
                    return true;
                }
            }
        }
        if chars.get(j) == Some(&'-') {
            let rest: String = chars[j + 1..].iter().collect();
            if rest.starts_with("fold") {
                let n = "fold".len();
                if n >= rest.len() || !is_word_char(rest[n..].chars().next().unwrap_or(' ')) {
                    return true;
                }
            }
        }
        if k > j {
            let rest: String = chars[k..].iter().collect();
            if rest.starts_with("times") {
                let n = "times".len();
                if n >= rest.len() || !is_word_char(rest[n..].chars().next().unwrap_or(' ')) {
                    return true;
                }
            }
        }
        i = j;
    }
    false
}

/// `<rival>: Nx`, including `llama.cpp: 1.8x` and `speedup over vLLM: 2x`.
/// The rival sits before the colon. A colon that is not followed by a multiplier
/// is not this form, so "Game 2: Beat the memorizer" is not a multiplier claim.
fn colon_nx_targets_rival(lower: &str) -> bool {
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != ':' {
            i += 1;
            continue;
        }
        let mut k = i + 1;
        while k < chars.len() && chars[k] == ' ' {
            k += 1;
        }
        if k >= chars.len() || !chars[k].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut j = k + 1;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        if j + 1 < chars.len() && chars[j] == '.' && chars[j + 1].is_ascii_digit() {
            j += 2;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
        }
        let mut m = j;
        while m < chars.len() && chars[m] == ' ' {
            m += 1;
        }
        let times = m < chars.len() && (chars[m] == 'x' || chars[m] == '\u{00d7}');
        if !times || is_clock_multiplier(&chars, k, m + 1) {
            i += 1;
            continue;
        }
        let after = m + 1;
        if after < chars.len() && !is_x_tail(chars[after]) {
            i += 1;
            continue;
        }
        let before: String = chars[..i].iter().collect();
        if names_rival(&before) {
            return true;
        }
        i += 1;
    }
    false
}

fn has_connector(lower: &str) -> bool {
    CONNECTORS.iter().any(|phrase| boundary_phrase(lower, phrase)) && names_rival(lower)
}

fn context_is_claim(lower: &str) -> bool {
    let tie = has_tie(lower);
    let word_or_phrase = has_context_word(lower) || has_context_phrase(lower) || tie;
    let attached = has_attached_comparative(lower);
    let aimed = bare_multiplier_targets_rival(lower) || colon_nx_targets_rival(lower);
    if !word_or_phrase && !attached && !aimed {
        return false;
    }
    if is_self_baseline(lower) && !has_outside_system(lower) && !has_rival(lower) {
        return false;
    }
    if attached {
        return true;
    }
    let bar = lower.contains("best measured cuda number");
    if word_or_phrase && (has_outside_system(lower) || has_rival(lower) || (tie && (bar || has_multiplier(lower)))) {
        return true;
    }
    aimed
}

fn is_comparison_claim(s: &str) -> bool {
    let scanned = without_urls(&s.to_ascii_lowercase());
    if phrase_claim(&scanned)
        || has_word_form(&scanned)
        || has_guarded_word(&scanned)
        || has_vs(&scanned)
        || has_connector(&scanned)
        || has_within_percent(&scanned)
    {
        return true;
    }
    context_is_claim(&scanned)
}

/// One exact sentence the checker allows, with the source and the reason.
/// A different wording is not on the list, so it is checked like any other sentence.
struct FixedSentence {
    sentence: &'static str,
    source: &'static str,
    reason: &'static str,
    /// The omega page prints these bars as `Done when: {sentence}`.
    done_when_prefix: bool,
}

/// CUDA performance bars from omega.json, then two published chemistry results.
/// A tie does not pass a CUDA bar: each bar sentence must require a beat, and a
/// different sentence with the same words does not. The chemistry sentences match
/// only as written. They are the scholar body in public/data/engineer.json.
/// Line 89 of that file is the description. Both sentences are in body_html
/// on line 91, and the link for each one is in the next paragraph.
const FIXED_SENTENCES: &[FixedSentence] = &[
    FixedSentence {
        sentence: "Omega's best format beats the best measured CUDA number at one conversation, with golden tokens identical to my host reference.",
        source: "https://github.com/aien-dev/aienos.com/blob/main/crates/site-builder/data/omega.json",
        reason: "Omega performance bar from omega.json. A different wording does not pass, so a tie does not pass.",
        done_when_prefix: true,
    },
    FixedSentence {
        sentence: "Omega prefill beats the best measured CUDA number at 512 and 8k.",
        source: "https://github.com/aien-dev/aienos.com/blob/main/crates/site-builder/data/omega.json",
        reason: "Omega performance bar from omega.json. A different wording does not pass, so a tie does not pass.",
        done_when_prefix: true,
    },
    FixedSentence {
        sentence: "Omega total words per second beats the best measured CUDA number at 8 and 32 conversations.",
        source: "https://github.com/aien-dev/aienos.com/blob/main/crates/site-builder/data/omega.json",
        reason: "Omega performance bar from omega.json. A different wording does not pass, so a tie does not pass.",
        done_when_prefix: true,
    },
    FixedSentence {
        sentence: "Platinum deposited this way outperformed organic-solvent materials in photocatalytic dye degradation.",
        source: "https://doi.org/10.3762/bjnano.10.71",
        reason: "published chemistry result, sourced in the next paragraph",
        done_when_prefix: false,
    },
    FixedSentence {
        sentence: "Ionic-liquid synthesis outperformed organic-solvent routes in photocatalytic methylene blue degradation.",
        source: "https://digitalcommons.wku.edu/theses/2101/",
        reason: "published chemistry result, sourced in the next paragraph",
        done_when_prefix: false,
    },
];

/// Game title. It names a memorizer, not another project's system.
const TURING_GAME_SENTENCE: &str = "Game 2: Beat the memorizer";

fn plain_sentence(s: &str) -> String {
    s.replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_fixed_sentence(s: &str) -> bool {
    let n = plain_sentence(s);
    if n == TURING_GAME_SENTENCE {
        return true;
    }
    FIXED_SENTENCES.iter().any(|row| {
        n == row.sentence || (row.done_when_prefix && n == format!("Done when: {}", row.sentence))
    })
}

/// `None` when the text is not a comparison, or is one honest sentence
/// with a real ISO date and a source link. Overclaims always return
/// "overclaim". A comparison with no date or no link in this same text
/// returns "unsourced comparison".
fn decode_nbsp(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let ch = s[i..].chars().next().unwrap();
        if ch == '\u{00a0}' {
            out.push(' ');
            i += ch.len_utf8();
            continue;
        }
        let rest = &lower[i..];
        let entity = ["&nbsp;", "&#160;", "&#xa0;", "&#x00a0;"]
            .into_iter()
            .find(|entity| rest.starts_with(entity));
        if let Some(entity) = entity {
            out.push(' ');
            i += entity.len();
            continue;
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_tags_to_space(s: &str) -> String {
    if !s.contains('<') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find('>') {
            out.push(' ');
            rest = &rest[start + end + 1..];
        } else {
            out.push_str(&rest[start..]);
            return out;
        }
    }
    out.push_str(rest);
    out
}

/// NBSP and `&nbsp;` become spaces, inline tags become spaces, and whitespace collapses.
fn prepare_sentence(s: &str) -> String {
    decode_nbsp(&strip_tags_to_space(s))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// "beating cuda" is an overclaim. "beating cuda++" is this project's GPU half, not that claim.
fn has_overclaim(lower: &str) -> bool {
    for w in OVERCLAIMS {
        if w == "beating cuda" {
            let mut from = 0;
            while let Some(at) = lower[from..].find(w) {
                let i = from + at;
                let before_ok = i == 0 || !is_word_char(lower[..i].chars().next_back().unwrap());
                let after = i + w.len();
                let plus = lower[after..].starts_with("++");
                let after_ok = after >= lower.len() || !is_word_char(lower[after..].chars().next().unwrap());
                if before_ok && after_ok && !plus {
                    return true;
                }
                from = i + w.len();
            }
            continue;
        }
        if lower.contains(w) {
            return true;
        }
    }
    false
}

pub fn comparison_problem(s: &str) -> Option<&'static str> {
    let prepared = prepare_sentence(s);
    let lower = prepared.to_ascii_lowercase();
    if has_overclaim(&lower) {
        return Some("overclaim");
    }
    if is_owned_slower(&prepared) || is_fixed_sentence(&prepared) {
        return None;
    }
    if is_comparison_claim(&prepared) && !(has_real_iso_date(&prepared) && has_allowed_source(&prepared)) {
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
        || tag_l.starts_with("</nav")
}

fn is_inline_join(tag_l: &str) -> bool {
    matches!(
        open_name(tag_l),
        Some(
            "a" | "abbr" | "b" | "bdi" | "bdo" | "br" | "cite" | "code" | "data" | "dfn" | "em" | "i"
                | "mark" | "q" | "s" | "small" | "span" | "strong" | "sub" | "sup" | "time" | "u"
                | "var" | "wbr"
        )
    )
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
        if is_inline_join(&tag_l) {
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
    // Block ends already become newlines in `readable_text`. A newline ends a
    // sentence so a claim in the next block does not inherit the previous line.
    for line in readable_text(text).split('\n') {
        for sentence in split_sentences(line) {
            if let Some(kind) = comparison_problem(&sentence) {
                errors.push(format!("{kind} in {sentence:?}"));
            }
        }
    }
    errors
}

/// Published papers and receipts, by exact repo path. Live pages are not on
/// this list: `/omega/`, `omega.json`, and the `/engineer/` tree are scanned.
/// A path matches only when the whole string equals an entry.
pub const PROSE_EXEMPT_PATHS: &[&str] = &[
    "public/engineer/papers/the-continuation-test.md",
    "crates/site-builder/data/models.json",
    "public/research/aien-research-plan.md",
    "public/research/computing-machinery-and-understanding-emergence/index.html",
    "public/research/the-stapleton-doctrine/index.html",
    "public/call-to-humanity/index.html",
];

pub fn is_prose_exempt(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    PROSE_EXEMPT_PATHS.contains(&norm.as_str())
}

/// Same check as `comparison_errors`, except a path on `PROSE_EXEMPT_PATHS` is skipped.
pub fn comparison_errors_at(path: &str, text: &str) -> Vec<String> {
    if is_prose_exempt(path) {
        Vec::new()
    } else {
        comparison_errors(text)
    }
}

fn section_is_exempt(rel: &str) -> bool {
    is_prose_exempt(&format!("public/{rel}")) || is_prose_exempt(rel)
}

/// `https://www.aienos.com/now/` becomes `now/index.html`. Anything else is unmapped.
fn map_aienos_page(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches(|c: char| c == '.' || c == ')');
    let rest = url
        .strip_prefix("https://www.aienos.com")
        .or_else(|| url.strip_prefix("http://www.aienos.com"))
        .or_else(|| url.strip_prefix("https://aienos.com"))
        .or_else(|| url.strip_prefix("http://aienos.com"))?;
    let path = rest.split(['?', '#', ' ']).next().unwrap_or("");
    if !path.is_empty() && !path.starts_with('/') {
        return None;
    }
    let rel = path.trim_matches('/');
    if rel.is_empty() {
        Some("index.html".to_string())
    } else if rel.ends_with(".html") || rel.ends_with(".md") || rel.ends_with(".json") || rel.ends_with(".tsv") {
        Some(rel.to_string())
    } else {
        Some(format!("{rel}/index.html"))
    }
}

fn norm_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

/// Fields from the public ledger long enough to identify a row. A sentence that
/// contains one of these is that row, not a new claim on the page.
fn ledger_fields(ledger: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in ledger.lines() {
        for field in line.split('\t') {
            let n = norm_ws(field);
            if n.chars().count() >= 40 {
                out.push(n);
            }
        }
    }
    out
}

fn is_ledger_excerpt(sentence: &str, fields: &[String]) -> bool {
    let s = norm_ws(sentence);
    let n = s.chars().count();
    if n < 40 {
        return false;
    }
    fields.iter().any(|field| s.contains(field.as_str()) || field.contains(s.as_str()))
}

/// `llms-full.txt` is one file with one section per page. Each section is
/// checked under its own source path. An exempt paper, receipt or ledger
/// section is skipped. A sentence copied from the public ledger stays with
/// the ledger. A section with no mappable source fails closed. The file as a
/// whole is not exempt.
fn push_line_problems(problems: &mut Vec<String>, n: usize, label: &str, line: &str, fields: &[String]) {
    if is_ledger_excerpt(line, fields) {
        return;
    }
    for sentence in split_sentences(line) {
        if is_ledger_excerpt(&sentence, fields) {
            continue;
        }
        if let Some(kind) = comparison_problem(&sentence) {
            problems.push(format!("llms-full.txt:{n} ({label}): {kind} in {sentence:?}"));
        }
    }
}

pub fn llms_full_problems(text: &str, ledger: &str) -> Vec<String> {
    let fields = ledger_fields(ledger);
    let mut problems = Vec::new();
    // None: file banner, before the first section. Some(None): section not yet
    // mapped. Some(Some(path)): mapped page.
    let mut section: Option<Option<String>> = None;
    let mut exempt = false;
    let mut reported_unmapped = false;
    let mut section_line = 1usize;
    let mut in_fence = false;
    let mut prelude: Vec<(usize, String)> = Vec::new();
    let flush = |section: &mut Option<Option<String>>,
                 reported_unmapped: bool,
                 section_line: usize,
                 prelude: &mut Vec<(usize, String)>,
                 problems: &mut Vec<String>,
                 fields: &[String]| {
        if let Some(None) = section {
            if !reported_unmapped && !prelude.is_empty() {
                problems.push(format!("llms-full.txt:{section_line}: unmapped section"));
                for (n, line) in prelude.iter() {
                    push_line_problems(problems, *n, "unmapped", line, fields);
                }
            }
            *section = None;
        }
        prelude.clear();
    };
    for (idx, line) in text.lines().enumerate() {
        let n = idx + 1;
        // html_to_text emits pre and textarea as fenced blocks. The page check
        // skips that program text, so the section check skips the same blocks.
        // A "---" inside a fence is program text, not the next page.
        if in_fence {
            if line.trim() == "```" {
                in_fence = false;
            }
            continue;
        }
        if line.trim() == "```" {
            in_fence = true;
            continue;
        }
        if line == "---" {
            flush(&mut section, reported_unmapped, section_line, &mut prelude, &mut problems, &fields);
            section = Some(None);
            exempt = false;
            reported_unmapped = false;
            section_line = n;
            continue;
        }
        if let Some(rest) = line.strip_prefix("Source: ") {
            if section.is_none() {
                section_line = n;
            }
            match map_aienos_page(rest) {
                Some(path) => {
                    exempt = section_is_exempt(&path);
                    if exempt {
                        prelude.clear();
                    } else {
                        for (pn, pline) in prelude.drain(..) {
                            push_line_problems(&mut problems, pn, &path, &pline, &fields);
                        }
                    }
                    section = Some(Some(path));
                }
                None => {
                    problems.push(format!("llms-full.txt:{n}: unmapped section {rest}"));
                    exempt = false;
                    section = Some(None);
                    reported_unmapped = true;
                    for (pn, pline) in prelude.drain(..) {
                        push_line_problems(&mut problems, pn, "unmapped", &pline, &fields);
                    }
                }
            }
            continue;
        }
        if section.is_none() {
            for sentence in split_sentences(line) {
                if let Some(kind) = comparison_problem(&sentence) {
                    problems.push(format!("llms-full.txt:{n}: {kind} in {sentence:?}"));
                }
            }
            continue;
        }
        if line.is_empty() || line.starts_with("Published:") {
            continue;
        }
        if exempt {
            continue;
        }
        match &section {
            Some(Some(path)) => push_line_problems(&mut problems, n, path, line, &fields),
            Some(None) => {
                if reported_unmapped {
                    push_line_problems(&mut problems, n, "unmapped", line, &fields);
                } else {
                    prelude.push((n, line.to_string()));
                }
            }
            None => {}
        }
    }
    flush(&mut section, reported_unmapped, section_line, &mut prelude, &mut problems, &fields);
    problems
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
        assert_eq!(spark.evidence_date, "2026-10-19");
        assert_eq!(spark.branch, "qwen38/serve-fast");
        assert!(!spark.on_main);
        assert!(spark.today.contains("Serves Qwen3.8-27B"), "{}", spark.today);
        assert!(spark.today.contains("aien-serve-http"), "{}", spark.today);
        assert!(!spark.today.contains("bin-16b0e6f8"), "{}", spark.today);
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
        assert!(
            published.is_empty(),
            "faster than, with no outside system and no rival noun, is not a claim: {published:?}"
        );
        let competition = comparison_errors("On 2026-10-21 this was faster than the competition.");
        assert!(
            competition.iter().any(|e| e.contains("unsourced comparison")),
            "{competition:?}"
        );
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
            "the fastest vLLM",
            "the worst vLLM",
            "the slowest vLLM",
            "the quickest vLLM",
            "best vLLM",
            "equals vLLM",
            "ties vLLM",
            "parity with vLLM",
            "outdoes vLLM",
            "tops vLLM",
            "trails vLLM",
            "lags vLLM",
            "edges out vLLM",
            "quicker than vLLM",
            "cheaper than vLLM",
            "more efficient than vLLM",
            "Omega vs CUDA",
            "twice CUDA",
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
        assert!(comparison_problem("The ratio was 1.8x.").is_none());
        assert!(comparison_problem("The note says (2x).").is_none());
        assert!(comparison_problem("The quickest walkthrough of what exists today.").is_none());
        assert!(comparison_problem("Omega vs CUDA.").is_some());
        assert!(comparison_problem("This ties CUDA.").is_some());
        assert!(comparison_problem("Omega's best format beats the best measured CUDA number at one conversation, with golden tokens identical to my host reference.").is_none());
        assert!(comparison_problem("Done when: Omega prefill beats the best measured CUDA number at 512 and 8k.").is_none());
        assert!(comparison_problem("Omega total words per second beats the best measured CUDA number at 8 and 32 conversations.").is_none());
        assert!(comparison_problem("Omega beats the best measured CUDA number.").is_some());
        assert!(comparison_problem("Game 2: Beat the memorizer").is_none());
        assert!(comparison_problem("Game 2: Beat CUDA").is_some());
        assert!(comparison_problem("The drive behind the grades.").is_none());
        assert!(comparison_problem("answers matching the shipped reference.").is_none());
        assert!(comparison_problem("This is not directly comparable to TOP500 scores.").is_none());
        assert!(comparison_problem("lowercase 0x and lowercase digits.").is_none());
        assert!(comparison_problem("P vs NP is open.").is_none());
        assert!(comparison_problem("The frame is 1920x1080 and the hex is 0xff.").is_none());
        let shouted = comparison_errors("BEATS vLLM. Faster Than the competition.");
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
    fn round5_sentences_pass_and_real_rivals_still_fail() {
        let pass = [
            "CUDA++, the GPU half of Omega, has its own page: /cudaplusplus/ CUDA++.",
            "What Omega is, in plain words, and its GPU half, CUDA++.",
            "CUDA++, its GPU half",
            "the GPU half of Omega and every step from program to chip",
            "CUDA++ is the GPU half of Omega, plus the motor.",
            "AR2 simulated analog provider with digital oracle parity",
            "CUDA++ is the GPU half of Omega, its own MIT project, plus the motor, with no NVIDIA user-space software, and how it was built.",
            "Its GPU half is now its own open source project, CUDA++, plus the motor, with no NVIDIA software above the kernel driver.",
            "The prime-counting benchmark on the GB10 runs 20.3 times faster than its own frozen starting point.",
            "I earned it step by step, and today I love that AI helps other people climb faster than I could.",
            "20.3x",
            "Prime Drag Race native GPU sieve on GB10 at 19,701 passes per second, 20.3x its own frozen starting point of 971.",
            "CUDA++ is described as the GPU half of Omega.",
            "CUDA++ is the GPU half of Omega, now its own open source project under the MIT licence.",
            "It is the GPU half of Omega, plus the motor, on the NVIDIA GPUs people already own, with no NVIDIA user-space software.",
            "AEGIS treats that boundary like a house: watch the door, recognize unauthorized access, isolate sessions, and preserve forensic trails for incident teams.",
            "CUDA++ is the GPU half of Omega, now its own open source project under Apache-2.0 with the LLVM exception.",
            "The bird's-eye map, whose axis begins 18 June 2026, labels a 4k standing prompt \"typed ~169\u{00d7}\" and notes that MobDev and Panel already exist.",
            "The cognitive faculty acted 137 times, the synthesis faculty 122 times.",
            "A heart beating does not mean the body is finished.",
            "The heart is beating.",
            "Then I watched one subsystem do it: a goal given, and a machine that continued with no hand inside the loop, promoting itself, cutting its own cost nearly in half, checking its work at the end.",
            "The heart beats.",
            "GPU half of Omega",
            "oracle parity",
            "forensic trails",
            "acted 137 times",
            "faster than I could",
            "typed ~169x",
            "typed ~169\u{00d7}",
            "20.3x its own frozen starting point",
            "faster than its own baseline",
            "faster than our own previous run",
            "2x faster than its own baseline",
            "parity with CUDA++",
            "twice CUDA++",
            "The GPU half of Omega, with no NVIDIA software.",
            "Claude Max 20x plan.",
            "Claude Max 20x planread 2026-10-30T14:45Z",
        ];
        for sentence in pass {
            assert!(
                comparison_problem(sentence).is_none(),
                "must pass: {sentence:?} -> {:?}",
                comparison_problem(sentence)
            );
        }
        let fail = [
            "2x faster than vLLM",
            "twice as fast as PyTorch",
            "parity with CUDA",
            "beats llama.cpp",
            "faster than the competition",
            "faster than its own baseline and vLLM",
            "2x faster",
            "40% cheaper",
            "half as fast",
            "3 times faster",
            "faster than NVIDIA",
            "twice NVIDIA",
            "Omega vs NVIDIA",
            "parity with NVIDIA",
            "the fastest NVIDIA card",
            "Omega vs CUDA",
            "20.3-fold vLLM",
            "2x vLLM",
        ];
        for sentence in fail {
            let problem = comparison_problem(sentence);
            assert!(
                problem.is_some(),
                "must fail: {sentence:?}"
            );
        }
        // The same-sentence source rule is unchanged. A one-word variant is not
        // on the exact list, so it still needs a real date and an allowlisted link.
        let variant = "Platinum deposited this way outperformed organic-solvent compounds in photocatalytic dye degradation.";
        assert_eq!(comparison_problem(variant), Some("unsourced comparison"));
        let sourced = format!(
            "{variant} Recorded 2019-04-02 (https://github.com/aien-dev/sovereign-intelligence)."
        );
        assert!(comparison_problem(&sourced).is_none(), "a dated allowlisted link still covers a real comparison");
        let unrelated = format!("{variant} See https://doi.org/10.3762/bjnano.10.71.");
        assert!(comparison_problem(&unrelated).is_some(), "doi.org is not an allowlisted source host");
    }

    #[test]
    fn fixed_sentences_match_exactly_and_carry_a_source() {
        for row in FIXED_SENTENCES {
            assert!(!row.source.is_empty() && row.source.starts_with("https://"), "{}", row.sentence);
            assert!(!row.reason.is_empty(), "{}", row.sentence);
            assert!(comparison_problem(row.sentence).is_none(), "{}", row.sentence);
        }
        let chemistry: Vec<_> = FIXED_SENTENCES.iter().filter(|row| !row.done_when_prefix).collect();
        assert_eq!(chemistry.len(), 2);
        for row in &chemistry {
            assert_eq!(row.reason, "published chemistry result, sourced in the next paragraph");
            let changed = row.sentence.replacen("outperformed", "surpassed", 1);
            assert_ne!(changed, row.sentence);
            assert_eq!(comparison_problem(&changed), Some("unsourced comparison"), "{changed}");
            let prefixed = format!("Done when: {}", row.sentence);
            assert!(comparison_problem(&prefixed).is_some(), "the chemistry sentence has no Done when prefix");
        }
        assert_eq!(
            chemistry[0].source,
            "https://doi.org/10.3762/bjnano.10.71"
        );
        assert_eq!(
            chemistry[1].source,
            "https://digitalcommons.wku.edu/theses/2101/"
        );
        let scholar = crate::components::engineer::pages()
            .into_iter()
            .find(|page| page.path == "/engineer/scholar/")
            .expect("scholar page");
        for row in chemistry {
            let at = scholar.body_html.find(row.sentence).unwrap_or_else(|| panic!("missing {}", row.sentence));
            let following = &scholar.body_html[at + row.sentence.len()..];
            let window = following.chars().take(800).collect::<String>();
            assert!(
                window.contains(row.source),
                "{} is not in the paragraph after {}",
                row.source,
                row.sentence
            );
        }
        assert!(comparison_errors(&scholar.body_html).is_empty());
    }

    #[test]
    fn cuda_bar_tie_variants_fail() {
        let bars = [
            "Omega's best format beats the best measured CUDA number at one conversation, with golden tokens identical to my host reference.",
            "Omega prefill beats the best measured CUDA number at 512 and 8k.",
            "Omega total words per second beats the best measured CUDA number at 8 and 32 conversations.",
        ];
        for bar in bars {
            assert!(comparison_problem(bar).is_none(), "{bar}");
            let tied = bar.replacen("beats", "matches", 1);
            assert_ne!(tied, bar);
            assert_eq!(comparison_problem(&tied), Some("unsourced comparison"), "{tied}");
        }
        assert_eq!(
            comparison_problem("Omega prefill matches the best measured CUDA number at 512 and 8k."),
            Some("unsourced comparison")
        );
    }

    #[test]
    fn arch_review_examples_fail() {
        let fail = [
            "beats the incumbent",
            "faster than the leading GPU stack",
            "Nvidia's stack is 3x slower",
            "NVIDIA trails Omega",
            "reaches NVIDIA parity",
            "Omega versus vLLM",
            "compared with vLLM",
            "matches vLLM",
            "tied with vLLM",
            "llama.cpp: 1.8x",
            "speedup over vLLM: 2x",
            "faster&nbsp;than CUDA",
            "faster\u{00a0}than CUDA",
            "faster<br>than CUDA",
            "faster<em>than</em> CUDA",
            "faster<span>than</span> CUDA",
            "equal to vLLM",
            "on par with vLLM",
            "matching vLLM",
            "match vLLM",
            "ties vLLM",
            "compared to vLLM",
            "relative to vLLM",
            "ahead of vLLM",
            "beats the leader",
            "faster than the market leader",
            "beats the GPU stacks",
            "faster than the field",
            "beats everyone else",
            "faster than existing tools",
            "beats Ollama",
            "faster than ONNX Runtime",
            "beats MLX",
            "faster than ROCm",
            "beats Groq",
            "faster than TensorRT",
            "beats SGLang",
        ];
        for sentence in fail {
            assert!(
                comparison_problem(sentence).is_some(),
                "must fail: {sentence:?} -> {:?}",
                comparison_problem(sentence)
            );
        }
        assert!(comparison_errors("faster<br>than CUDA").iter().any(|e| e.contains("faster than CUDA")));
        assert_eq!(comparison_problem("beating CUDA"), Some("overclaim"));
        assert_eq!(comparison_problem("beating CUDA today"), Some("overclaim"));
        assert!(
            comparison_problem("convert once ahead of time and store").is_none(),
            "ahead of time is an idiom: {:?}",
            comparison_problem("convert once ahead of time and store")
        );
        assert_eq!(
            comparison_problem("Omega is ahead of NVIDIA."),
            Some("unsourced comparison"),
            "ahead of a rival is still a comparison"
        );
        assert!(
            comparison_problem("Session-limit cutoff 2026-10-19 ~19:3x CDT killed 9 Claude agents.").is_none(),
            "a clock token is not a multiplier: {:?}",
            comparison_problem("Session-limit cutoff 2026-10-19 ~19:3x CDT killed 9 Claude agents.")
        );
        assert!(
            comparison_problem("Noted at ~19:30 CDT on 2026-10-19.").is_none(),
            "a timestamp is not a multiplier: {:?}",
            comparison_problem("Noted at ~19:30 CDT on 2026-10-19.")
        );
        assert_eq!(
            comparison_problem("On 2026-10-19 at ~19:30 CDT the run was 3x faster than vLLM."),
            Some("unsourced comparison"),
            "a real multiplier beside a timestamp still fails"
        );
        assert!(
            comparison_problem("That grip holds back every rival chipmaker.").is_none(),
            "holds back plus a rival noun is not a comparison: {:?}",
            comparison_problem("That grip holds back every rival chipmaker.")
        );
        assert!(
            comparison_problem("holds back every rival chipmaker, every independent builder, and every person who wants intelligence running on hardware they own.").is_none(),
            "the call-to-humanity sentence is not a comparison: {:?}",
            comparison_problem("holds back every rival chipmaker, every independent builder, and every person who wants intelligence running on hardware they own.")
        );
        assert_eq!(
            comparison_problem("Omega is faster than every rival chipmaker."),
            Some("unsourced comparison"),
            "a rival noun with a real comparative still fails"
        );
        assert_eq!(
            comparison_problem("This rivals vLLM."),
            Some("unsourced comparison"),
            "the verb rivals is still a comparison"
        );
        assert!(
            comparison_problem("beating CUDA++").is_none(),
            "beating CUDA++ is this project's GPU half, not the beating-CUDA overclaim: {:?}",
            comparison_problem("beating CUDA++")
        );
        for sentence in [
            "The GPU half of Omega, with no NVIDIA software.",
            "parity with CUDA++",
            "twice CUDA++",
            "answers matching the shipped reference.",
            "Understanding versus search.",
            "How hard was it compared with verifying it?",
            "A paper projects average investment equal to 3.63% of U.S. GDP per year.",
            "Claude Max 20x plan.",
        ] {
            assert!(comparison_problem(sentence).is_none(), "must pass: {sentence:?} -> {:?}", comparison_problem(sentence));
        }
    }

    #[test]
    fn llms_full_is_checked_section_by_section() {
        let ledger = "id\titem\nL1\tCUDA++ developer experience includes a D-bar that beats CUDA in the written plan for the driver work\n";
        let doc = "\
# Sovereign Intelligence

The site in one file.

---

# A Call to Humanity

Source: https://www.aienos.com/call-to-humanity/

This beats vLLM with no date.

---

# Now

Source: https://www.aienos.com/now/

CUDA++ developer experience includes a D-bar that beats CUDA in the written plan for the driver work
The now page itself is faster than vLLM.

---

# Agents

Source: https://www.aienos.com/agents/

Same-Spark end-to-end head-to-head: aien-serve-http vs vLLM vs llama.cpp.

---

# Nowhere

Source: https://example.com/not-a-page

Omega beats CUDA.

---

# Reference

Source: https://www.aienos.com/omega/reference/

```
note collatz: exceeding it traps 4
---
```

The reference page itself is faster than CUDA.
";
        let problems = llms_full_problems(doc, ledger);
        let joined = problems.join("\n");
        assert!(!joined.contains("call-to-humanity"), "exempt source must be skipped: {joined}");
        assert!(!joined.contains("D-bar that beats CUDA"), "a ledger excerpt is not a new page claim: {joined}");
        assert!(joined.contains("now/index.html") && joined.contains("faster than vLLM"), "the now page's own sentence still fails: {joined}");
        assert!(joined.contains("agents/index.html") && joined.contains("vs vLLM"), "a live page section still fails: {joined}");
        assert!(joined.contains("unmapped"), "an unmapped source fails closed: {joined}");
        assert!(!joined.contains("exceeding it traps"), "program text in a fence is not page prose: {joined}");
        assert!(
            joined.contains("omega/reference/index.html") && joined.contains("faster than CUDA"),
            "prose after a fence is still checked: {joined}"
        );
        assert!(!is_prose_exempt("llms-full.txt"), "the aggregate file is not itself exempt");
        assert!(!is_prose_exempt("public/llms-full.txt"));
    }

    #[test]
    fn engineer_json_unsourced_comparison_fails_the_build() {
        let raw = include_str!("../../../../public/data/engineer.json");
        let line = raw
            .lines()
            .position(|l| l.contains("Platinum deposited this way outperformed"))
            .expect("chemistry sentence")
            + 1;
        assert_eq!(line, 91, "body_html holds the sentences; line 89 is the description");
        assert!(!raw.lines().nth(88).unwrap().contains("Platinum deposited"));
        let needle = "Platinum deposited this way outperformed organic-solvent materials in photocatalytic dye degradation.";
        let doctored = raw.replacen(needle, &format!("{needle} This new line outperforms vLLM."), 1);
        assert_ne!(doctored, raw);
        let value: serde_json::Value = serde_json::from_str(&doctored).expect("doctored engineer.json still parses");
        let body = value["pages"]
            .as_array()
            .expect("pages")
            .iter()
            .find_map(|page| {
                let body = page["body_html"].as_str()?;
                body.contains("This new line outperforms vLLM.").then(|| body.to_string())
            })
            .expect("the injected sentence is in engineer.json");
        let problems = comparison_errors(&body);
        assert!(
            problems.iter().any(|e| e.contains("unsourced comparison") && e.contains("outperforms vLLM")),
            "an unsourced comparison added to engineer.json fails the check the build runs: {problems:?}"
        );
        let clean: serde_json::Value = serde_json::from_str(raw).expect("engineer.json");
        let clean_body = clean["pages"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|page| {
                let body = page["body_html"].as_str()?;
                body.contains(needle).then_some(body)
            })
            .unwrap();
        assert!(comparison_errors(clean_body).is_empty(), "{clean_body}");
    }

    #[test]
    fn published_papers_are_exempt_by_exact_path() {
        let papers = [
            "public/engineer/papers/the-continuation-test.md",
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
            assert!(!is_prose_exempt(&format!("/tmp/{path}")), "{path} must not match by ending");
            assert!(!is_prose_exempt(&format!("dist/{path}")), "{path} must not match by ending");
        }
        for live in [
            "public/research/index.html",
            "omega/index.html",
            "dist/omega/index.html",
            "public/omega/index.html",
            "crates/site-builder/data/omega.json",
            "omega.json",
            "public/data/engineer.json",
            "/engineer/",
            "engineer/index.html",
            "public/engineer/index.html",
            "dist/engineer/index.html",
        ] {
            assert!(!is_prose_exempt(live), "{live} is a live page and must be scanned");
            assert!(!comparison_errors_at(live, claim).is_empty(), "{live} was skipped");
        }
        assert_eq!(PROSE_EXEMPT_PATHS, papers);
    }

    #[test]
    fn omega_and_every_other_page_use_the_shared_check() {
        let omega = crate::components::omega::render_omega_page().into_string();
        assert!(!is_prose_exempt("omega/index.html"));
        let problems = comparison_errors("This beats vLLM.");
        assert!(!problems.is_empty());
        let mut bad = Vec::new();
        for problem in comparison_errors_at("omega/index.html", &omega) {
            bad.push(format!("omega/index.html: {problem}"));
        }
        let planted = "Measured on 2026-10-21 (https://example.com/bench). vLLM is slower than Omega.";
        let problems = comparison_errors(planted);
        assert!(
            problems.iter().any(|e| e.contains("unsourced comparison") && e.contains("slower than")),
            "the same function still refuses a planted sentence: {problems:?}"
        );

        for page in crate::components::engineer::pages() {
            assert!(!is_prose_exempt(&page.path), "engineer page {} must be scanned", page.path);
            assert!(
                !comparison_errors_at(&page.path, "This beats vLLM.").is_empty(),
                "{} must not be skipped",
                page.path
            );
        }

        let facts: crate::facts::Facts = serde_json::from_str(include_str!("../../../../facts.json")).expect("facts");
        let engineer_pages = crate::components::engineer::pages();
        for page in &engineer_pages {
            let html = crate::components::engineer::render(page, &engineer_pages, &facts).into_string();
            for problem in comparison_errors(&html) {
                bad.push(format!("{}: {problem}", page.path));
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        collect_html(&root.join("public"), &mut files);
        collect_html(&root.join("crates/site-builder/templates"), &mut files);
        collect_named(&root.join("om"), "main.html", &mut files);
        assert!(files.len() > 20, "expected the shipped pages, found {}", files.len());
        for path in &files {
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let rel = path.strip_prefix(&root).unwrap_or(path);
            let rel = rel.to_string_lossy().replace('\\', "/");
            let problems = comparison_errors_at(&rel, &text);
            if !problems.is_empty() {
                bad.push(format!(
                    "{}: {}",
                    path.display(),
                    problems.iter().map(|e| clip(e, 400)).collect::<Vec<_>>().join(" | ")
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
        let rd = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("unreadable directory {}: {e}", dir.display()));
        for entry in rd {
            let entry = entry.unwrap_or_else(|e| panic!("unreadable directory entry in {}: {e}", dir.display()));
            let path = entry.path();
            if path.is_dir() {
                collect_html(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
                out.push(path);
            }
        }
    }

    fn collect_named(dir: &std::path::Path, name: &str, out: &mut Vec<std::path::PathBuf>) {
        let rd = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("unreadable directory {}: {e}", dir.display()));
        for entry in rd {
            let entry = entry.unwrap_or_else(|e| panic!("unreadable directory entry in {}: {e}", dir.display()));
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
