//! Drake writes the site alone, in first person singular (I, me, my).
//! Plural first-person words (we, us, our, ours, ourselves and their
//! contractions) in rendered copy fail the build. Quoted third-party words
//! and code identifiers are the only exceptions, listed in `ALLOW`. Text
//! copied verbatim from the published work ledger (`data/now.tsv`, shown on
//! /now/) is a dated record of what was said, not copy, and is matched out.

use regex::Regex;
use std::fs;
use std::path::Path;

/// (file under dist, exact phrase containing the plural word). Keep it small:
/// every entry is someone else's words or a code identifier, never site copy.
pub const ALLOW: &[(&str, &str)] = &[
    // Yann LeCun, quoted on the post-LLM case.
    ("post-llm-case/index.html", "We\u{2019}re never going to get to human-level intelligence by training LLMs or by training on text only. We need the real world."),
    // The Continuation Test quotes the objections it answers, in the objectors' words.
    ("engineer/works/the-continuation-test/index.html", "It only does what we tell it."),
    ("engineer/works/the-continuation-test/index.md", "It only does what we tell it."),
    ("engineer/works/the-continuation-test.md", "It only does what we tell it."),
    ("engineer/papers/the-continuation-test.md", "It only does what we tell it."),
    ("llms-full.txt", "It only does what we tell it."),
    ("engineer/works/the-continuation-test/index.html", "If we can't write down the rules for good thinking"),
    ("engineer/works/the-continuation-test/index.md", "If we can't write down the rules for good thinking"),
    ("engineer/works/the-continuation-test.md", "If we can't write down the rules for good thinking"),
    ("engineer/papers/the-continuation-test.md", "If we can't write down the rules for good thinking"),
    ("llms-full.txt", "If we can't write down the rules for good thinking"),
    // The first Atlas Symphony run, quoted verbatim as the historical record of 30 July 2026.
    ("engineer/works/atlas-symphony/index.html", "What's left in our development plan?"),
    ("engineer/works/atlas-symphony/index.md", "What's left in our development plan?"),
    ("engineer/works/atlas-symphony.md", "What's left in our development plan?"),
    ("llms-full.txt", "What's left in our development plan?"),
    ("engineer/works/atlas-symphony/index.html", "Please proceed with our Symphony."),
    ("engineer/works/atlas-symphony/index.md", "Please proceed with our Symphony."),
    ("engineer/works/atlas-symphony.md", "Please proceed with our Symphony."),
    ("llms-full.txt", "Please proceed with our Symphony."),
];

/// Pages that embed ledger rows; only these may match text out of the ledger.
pub const LEDGER_PAGES: &[&str] = &["now/index.html", "now/index.md", "now.md", "llms-full.txt"];

fn plural() -> Regex {
    Regex::new(r"(?i)\b(?:we|we['\u{2019}](?:re|ve|ll|d)|us|our|ours|ourselves)\b").unwrap()
}

/// Visible text of an HTML page: scripts, styles and tags removed, common
/// entities decoded. Markdown and plain text pass through unchanged.
pub fn visible_text(name: &str, raw: &str) -> String {
    if !name.ends_with(".html") {
        return raw.to_string();
    }
    let blocks = Regex::new(r"(?is)<script\b.*?</script>|<style\b.*?</style>").unwrap();
    let tags = Regex::new(r"(?s)<[^>]*>").unwrap();
    let t = blocks.replace_all(raw, " ");
    let t = tags.replace_all(&t, " ");
    t.replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Plural first-person words in `text`, after removing allowed phrases,
/// link targets, units (`50 us`) and phrases found verbatim in `record`.
pub fn violations(rel: &str, text: &str, record: &str) -> Vec<String> {
    let mut t = text.to_string();
    for (file, phrase) in ALLOW {
        if *file == rel {
            t = t.replace(phrase, " ");
        }
    }
    let links = Regex::new(r"\]\([^)\s]*\)|https?://\S+").unwrap();
    let t = squash(&links.replace_all(&t, " "));
    plural()
        .find_iter(&t)
        .filter(|m| m.as_str() != "US")
        .filter(|m| {
            // Path and identifier parts (docs/how-we-did-it, HOW_WE_DID_IT) and units (50 us).
            let joined = |c: Option<char>| c.is_some_and(|c| matches!(c, '-' | '_' | '/' | '.' | '#'));
            let before = t[..m.start()].trim_end();
            let unit = m.as_str().eq_ignore_ascii_case("us") && before.ends_with(|c: char| c.is_ascii_digit());
            !(unit || joined(t[..m.start()].chars().last()) || joined(t[m.end()..].chars().next()))
        })
        .filter(|m| {
            let s = t[..m.start()].char_indices().rev().nth(11).map_or(0, |(i, _)| i);
            let e = t[m.end()..].char_indices().nth(12).map_or(t.len(), |(i, _)| m.end() + i);
            // A redacted span ([private path]) may sit on one side, so either half counts.
            record.is_empty() || !(record.contains(&t[s..m.end()]) || record.contains(&t[m.start()..e]))
        })
        .map(|m| {
            let s = t[..m.start()].char_indices().rev().nth(50).map_or(0, |(i, _)| i);
            let e = t[m.end()..].char_indices().nth(30).map_or(t.len(), |(i, _)| m.end() + i);
            t[s..e].to_string()
        })
        .collect()
}

fn walk(dir: &Path, root: &Path, record: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read dist dir") {
        let path = entry.expect("dist entry").path();
        if path.is_dir() {
            walk(&path, root, record, out);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if !(rel.ends_with(".html") || rel.ends_with(".md") || rel.ends_with(".txt")) {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {rel}"));
        let rec = if LEDGER_PAGES.contains(&rel.as_str()) { record } else { "" };
        for v in violations(&rel, &visible_text(&rel, &raw), rec) {
            out.push(format!("{rel}: ...{v}..."));
        }
    }
}

/// Every rendered page, markdown twin and llms file is in Drake's voice.
pub fn verify_singular_voice(dist: &Path) {
    let record = squash(&fs::read_to_string(dist.join("data/now.tsv")).unwrap_or_default().replace('\t', " "));
    let mut out = Vec::new();
    walk(dist, dist, &record, &mut out);
    assert!(
        out.is_empty(),
        "First person plural in site copy (write I, me, my; see AGENTS.md):\n{}",
        out.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_words_fail_and_singular_passes() {
        for bad in ["We build it.", "tell us where", "our own engine", "not ours", "we're open", "We\u{2019}ve run", "ourselves"] {
            assert!(!violations("x.html", bad, "").is_empty(), "{bad}");
        }
        for ok in ["I build it with my agents.", "tell me where", "my own engine", "US dollars", "user, trust, focus, pour"] {
            assert!(violations("x.html", ok, "").is_empty(), "{ok}");
        }
    }

    #[test]
    fn allowlist_is_per_file_and_exact() {
        let (file, phrase) = ALLOW[0];
        assert!(violations(file, phrase, "").is_empty());
        assert!(!violations("index.html", phrase, "").is_empty());
    }

    #[test]
    fn ledger_records_links_and_units_are_not_copy() {
        let rec = "L1 Drake: why cant we train Allen to do it";
        assert!(violations("now.md", "Drake: why cant we train Allen to do it", rec).is_empty());
        assert!(!violations("now.md", "Today we ship it.", rec).is_empty());
        assert!(violations("x.md", "[doc](https://x/how-we-did-it) poll 50 us", "").is_empty());
    }

    #[test]
    fn markup_and_scripts_are_not_copy() {
        let html = "<p class=\"our\">mine</p><script>var us=1</script><style>.we{}</style>";
        assert!(violations("x.html", &visible_text("x.html", html), "").is_empty());
    }
}
