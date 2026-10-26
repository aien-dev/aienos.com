//! Drake writes the site alone, in first person singular (I, me, my).
//! Plural first-person words (we, us, our, ours, ourselves and their
//! contractions) in rendered copy fail the build. Quoted third-party words
//! and code identifiers are the only exceptions, listed in `ALLOW`. Text
//! copied verbatim from the published work ledger (`data/now.tsv`, shown on
//! /now/) is a dated record of what was said, not copy, and is matched out.
//! Drake's own sourced words in `data/quotes.json` are a record too: an HTML
//! element marked `data-quote-id="qNN"` whose text is exactly quote qNN is
//! removed before matching, and a marked element that does not match its
//! quote fails the build. Markdown and text mirrors carry no markup, so
//! there only the exact quote strings are removed.

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

/// Drake's sourced quotes, compiled in from the repository so a generated
/// tree cannot add its own exemptions.
pub const QUOTES_RAW: &str = include_str!("../../../../data/quotes.json");

/// The attribute that marks one quote in HTML.
pub const QUOTE_ATTR: &str = "data-quote-id=\"";

/// (id, quote) for every entry of `data/quotes.json`. Ids are unique and
/// quotes non-empty, or the verifier refuses to run.
pub fn quotes() -> Vec<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(QUOTES_RAW).expect("data/quotes.json parses");
    let mut out: Vec<(String, String)> = Vec::new();
    for q in v["quotes"].as_array().expect("data/quotes.json has quotes") {
        let id = q["id"].as_str().expect("every quote has an id").to_string();
        let text = q["quote"].as_str().expect("every quote has its text").to_string();
        assert!(!text.trim().is_empty(), "quote {id} is empty");
        assert!(out.iter().all(|(i, _)| *i != id), "quote id {id} is used twice");
        out.push((id, text));
    }
    out
}

/// Removes every element marked [`QUOTE_ATTR`] whose reader text equals
/// its quote exactly (entities decoded, whitespace squashed). An unknown
/// id, a different text, or an element that never closes is an error.
pub fn without_sourced_quotes(raw: &str, quotes: &[(String, String)]) -> Result<String, String> {
    use crate::oracle::{element_end, reader_text};
    let mut s = raw.to_string();
    let mut from = 0;
    while let Some(at) = s[from..].find(QUOTE_ATTR).map(|i| from + i) {
        let open = s[..at].rfind('<');
        let gt = open.and_then(|o| s[o..].find('>').map(|g| o + g));
        let (Some(open), Some(gt)) = (open, gt) else {
            from = at + QUOTE_ATTR.len();
            continue;
        };
        if gt < at || !s[..at].ends_with(|c: char| c.is_ascii_whitespace()) {
            // Plain text or a lookalike attribute: not a marker, exempts nothing.
            from = at + QUOTE_ATTR.len();
            continue;
        }
        let id_start = at + QUOTE_ATTR.len();
        let id = &s[id_start..id_start + s[id_start..].find('"').ok_or("unterminated data-quote-id")?];
        let quote = &quotes.iter().find(|(i, _)| i == id).ok_or(format!("data-quote-id {id:?} is not in data/quotes.json"))?.1;
        let name = crate::seo::tag_name(&s[open..]);
        let end = element_end(&s, gt + 1, &name).ok_or(format!("quote {id} never closes"))?;
        let inner = &s[gt + 1..end - format!("</{name}>").len()];
        let shown = reader_text(inner);
        if shown != squash(quote) {
            return Err(format!("quote {id} is not verbatim: shows {shown:?}, data/quotes.json has {quote:?}"));
        }
        s.replace_range(open..end, " ");
        from = open;
    }
    Ok(s)
}

/// A markdown or text mirror with every exact quote string removed.
pub fn without_quote_strings(raw: &str, quotes: &[(String, String)]) -> String {
    let mut s = raw.to_string();
    for (_, q) in quotes {
        s = s.replace(q.as_str(), " ");
    }
    s
}

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

fn walk(dir: &Path, root: &Path, record: &str, quotes: &[(String, String)], out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read dist dir") {
        let path = entry.expect("dist entry").path();
        if path.is_dir() {
            walk(&path, root, record, quotes, out);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if !(rel.ends_with(".html") || rel.ends_with(".md") || rel.ends_with(".txt")) {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {rel}"));
        let raw = if rel.ends_with(".html") {
            match without_sourced_quotes(&raw, quotes) {
                Ok(s) => s,
                Err(e) => {
                    out.push(format!("{rel}: {e}"));
                    continue;
                }
            }
        } else {
            without_quote_strings(&raw, quotes)
        };
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
    walk(dist, dist, &record, &quotes(), &mut out);
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

    fn check_html(html: &str) -> Result<Vec<String>, String> {
        let q = quotes();
        without_sourced_quotes(html, &q).map(|s| violations("learn/index.html", &visible_text("learn/index.html", &s), ""))
    }

    fn q05() -> String {
        let text = quotes().into_iter().find(|(i, _)| i == "q05").expect("q05 exists").1;
        assert!(!violations("x.md", &text, "").is_empty(), "the fixture quote must say we or our");
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
    }

    #[test]
    fn quotes_file_loads_with_unique_ids() {
        assert!(quotes().len() >= 2);
    }

    #[test]
    fn an_exact_marked_quote_is_exempt_and_nothing_around_it() {
        let q = q05();
        assert!(check_html(&format!("<blockquote class=\"quote\" data-quote-id=\"q05\"><p>{q}</p></blockquote><p>I build it.</p>")).unwrap().is_empty());
        assert!(check_html(&format!("<p data-quote-id=\"q05\">\n  {q}\n</p>")).unwrap().is_empty(), "whitespace is squashed");
        // The page around a quote is still checked.
        assert!(!check_html(&format!("<p data-quote-id=\"q05\">{q}</p><p>We ship it.</p>")).unwrap().is_empty());
    }

    #[test]
    fn quote_negative_controls() {
        let q = q05();
        // A marked element that is not the quote fails outright.
        for bad in [
            format!("<p data-quote-id=\"q05\">{q} and we added this</p>"),
            format!("<p data-quote-id=\"q05\">We wrote something else.</p>"),
            format!("<p data-quote-id=\"q99\">{q}</p>"),
            format!("<p data-quote-id=\"q01\">{q}</p>"),
            format!("<p data-quote-id=\"q05\">{q}"),
        ] {
            assert!(check_html(&bad).is_err(), "must reject: {bad}");
        }
        // Unmarked, lookalike-marked or speaker-marked quote text is copy and is checked.
        for copy in [
            format!("<p>{q}</p>"),
            format!("<p xdata-quote-id=\"q05\">{q}</p>"),
            format!("<p data-speaker=\"drake\">{q}</p>"),
            format!("<p>data-quote-id=\"q05\" {q}</p>"),
        ] {
            assert!(!check_html(&copy).unwrap().is_empty(), "must flag: {copy}");
        }
        // Mirrors: the exact string is exempt; an altered one is not.
        let raw = quotes().into_iter().find(|(i, _)| i == "q05").unwrap().1;
        let qs = quotes();
        assert!(violations("learn.md", &without_quote_strings(&format!("> {raw}\n\nI build it."), &qs), "").is_empty());
        assert!(!violations("learn.md", &without_quote_strings(&format!("> {raw} and ours"), &qs), "").is_empty());
        assert!(!violations("learn.md", &without_quote_strings(&raw.replacen("our", "OUR", 1), &qs), "").is_empty());
    }
}
