//! Canonical names in rendered copy. Every visible spelling of a project,
//! product or vendor name must match `CANON`: "Aienos", "AIEN" in a
//! sentence, "Github" or a visible "Allen" (renamed to Aien) fail the build.
//!
//! Not copy, so never checked: attributes (href, id, data-speaker="allen"),
//! `<script>`, `<style>`, `<code>`, `<pre>`, `<kbd>`, `<samp>`, any element
//! marked `data-verbatim` (quoted third-party words such as GitHub repository
//! descriptions), identifier joins (aien-dev, aienos.com, nvidia.ko), the
//! phrases in `ALLOW` (published titles, CI check names) and text copied
//! verbatim from a record: the work ledger on /now/, the research status table
//! and the titles read from GitHub into `facts.json` (merged PR titles, decision
//! record titles, the receipt's commit title and status). Hand-written roles in
//! `facts.json` are not a record. A label written entirely in
//! capitals (AIEN VERIFICATION) uppercases every name, so it is not a misspelling.

use regex::Regex;
use std::fs;
use std::path::Path;

/// (every spelling, as a pattern; the accepted spellings). A match passes
/// only when it equals an accepted spelling exactly. Lowercase `aienos` is
/// the repository aien-dev/aienos, named in records as "aienos #440".
pub const CANON: &[(&str, &[&str])] = &[
    (r"(?i)\baien\s?os\b", &["AienOS", "aienos"]),
    (r"(?i)\baien\b", &["Aien"]),
    (r"\bAllen\b", &["Aien"]),
    (r"(?i)\baienix\b", &["aienix"]),
    (r"(?i)\baienux\b", &["aienux"]),
    (r"(?i)\bgithub\b", &["GitHub"]),
    (r"(?i)\bnvidia\b", &["NVIDIA"]),
    (r"(?i)\bwebassembly\b", &["WebAssembly"]),
    (r"(?i)\bimac\b", &["iMac"]),
    (r"(?i)\bzenodo\b", &["Zenodo"]),
    (r"(?i)\bopencl\b", &["OpenCL"]),
    (r"(?i)\bpocl\b", &["PoCL"]),
    (r"(?i)\bspir-?v\b", &["SPIR-V"]),
    (r"(?i)\buefi\b", &["UEFI"]),
    (r"(?i)\bdgx\b", &["DGX"]),
    (r"(?i)\bgb10\b", &["GB10"]),
    (r"(?i)\bqwen\b", &["Qwen"]),
    (r"(?i)\bcuda\+\+", &["CUDA++"]),
];

/// Exact phrases that keep a non-canonical spelling on purpose, on every page.
/// Each one is a published title, a CI check name or a dated record.
pub const ALLOW: &[&str] = &[
    // Zenodo-published paper titles, kept as published.
    "The AIEN Papers",
    "Atemporal AIEN",
    "Measure Emerging from AIEN",
    // The same titles as the zh-CN pages translate them.
    "AIEN 论文",
    "从 AIEN 中涌现",
    // Citation of a published preprint: the publisher line as printed.
    "AIEN Research, 2026",
    // The required CI check and its Spark context, named exactly as GitHub shows them.
    "AIEN Verification",
    "AIEN 验证",
    // A product name and an image caption, kept until Drake decides.
    "AIEN Rig",
    "AIEN headband",
];


const SKIP_TAGS: &[&str] = &["script", "style", "code", "pre", "kbd", "samp"];

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<').trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// Visible copy of an HTML page with every tag turned into a line break, so a
/// label stays one line. Skipped elements (see the module doc) are removed.
pub fn copy_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while let Some(off) = raw[i..].find('<') {
        out.push_str(&raw[i..i + off]);
        let start = i + off;
        if raw[start..].starts_with("<!--") {
            i = raw[start..].find("-->").map_or(raw.len(), |e| start + e + 3);
            out.push('\n');
            continue;
        }
        let end = raw[start..].find('>').map_or(raw.len(), |e| start + e + 1);
        let tag = &raw[start..end];
        let name = tag_name(tag);
        out.push('\n');
        i = end;
        let opening = !tag.starts_with("</") && !tag.ends_with("/>") && !name.is_empty();
        if opening && (SKIP_TAGS.contains(&name.as_str()) || tag.contains(" data-verbatim")) {
            // Skip to the matching close tag of the same name.
            let (open, close) = (format!("<{name}"), format!("</{name}"));
            let mut depth = 1;
            let mut j = end;
            while depth > 0 {
                let Some(n) = raw[j..].find('<') else { j = raw.len(); break };
                let at = j + n;
                let rest = &raw[at..];
                let te = rest.find('>').map_or(raw.len(), |e| at + e + 1);
                let t = &raw[at..te];
                if rest.starts_with(&close) && tag_name(t) == name {
                    depth -= 1;
                } else if rest.starts_with(&open) && tag_name(t) == name && !t.ends_with("/>") {
                    depth += 1;
                }
                j = te;
            }
            i = j;
        }
    }
    out.push_str(&raw[i.min(raw.len())..]);
    out.replace("&amp;", "&")
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

/// A line written entirely in capitals with at least two words is a styled label.
fn caps_label(line: &str) -> bool {
    let words = line.split(|c: char| !c.is_alphabetic()).filter(|w| w.chars().count() >= 2).count();
    // Every letter is a capital; CJK text has no case, so it is never a label.
    words >= 2 && line.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
}

/// Non-canonical names in the copy `text` of the page `rel`, after removing
/// `ALLOW` phrases and link targets. A match passes when the record holds it
/// with at least four characters of the same context on one side.
pub fn violations(text: &str, record: &str) -> Vec<String> {
    let links = Regex::new(r"https?://\S+").unwrap();
    let rules: Vec<(Regex, &[&str])> = CANON.iter().map(|(p, c)| (Regex::new(p).unwrap(), *c)).collect();
    let mut out = Vec::new();
    for raw_line in text.lines() {
        let mut line = raw_line.to_string();
        for phrase in ALLOW {
            line = line.replace(phrase, " ");
        }
        let line = squash(&links.replace_all(&line, " "));
        if line.is_empty() || caps_label(&line) {
            continue;
        }
        for (re, canon) in &rules {
            for m in re.find_iter(&line) {
                if canon.contains(&m.as_str()) {
                    continue;
                }
                // Identifier and address parts: aien-dev, aienos.com, nvidia.ko, @aien.
                let joined = |c: Option<char>| c.is_some_and(|c| matches!(c, '-' | '_' | '/' | '.' | '#' | '@' | '='));
                let next = line[m.end()..].chars().next();
                let trailing_dot = next == Some('.') && line[m.end() + 1..].chars().next().is_none_or(|c| c.is_whitespace());
                if joined(line[..m.start()].chars().last()) || (joined(next) && !trailing_dot) {
                    continue;
                }
                if in_record(&line, m.start(), m.end(), record) {
                    continue;
                }
                out.push(format!("\"{}\" should be \"{}\" in: {line}", m.as_str(), canon[0]));
            }
        }
    }
    out
}

/// The match with up to 12 characters of context on either side, read in
/// the record. A side shorter than four characters proves nothing.
fn in_record(line: &str, s: usize, e: usize, record: &str) -> bool {
    if record.is_empty() {
        return false;
    }
    let before: Vec<usize> = line[..s].char_indices().map(|(i, _)| i).collect();
    let after: Vec<usize> = line[e..].char_indices().map(|(i, _)| e + i).chain([line.len()]).collect();
    let b = before.len().min(12);
    let a = (after.len() - 1).min(12);
    (b >= 4 && record.contains(&line[before[before.len() - b]..e])) || (a >= 4 && record.contains(&line[s..after[a]]))
}

/// Every string in the research status record, one per line.
fn status_record(raw: &str) -> String {
    fn walk(v: &serde_json::Value, out: &mut String) {
        match v {
            serde_json::Value::String(s) => {
                out.push_str(&squash(s));
                out.push('\n');
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = String::new();
    walk(&serde_json::from_str(raw).expect("research status JSON"), &mut out);
    out
}

/// The strings `facts` copied from GitHub, one per line: each component's last
/// merged PR title, each decision record title, the receipt's commit title and
/// status description. Editorial fields (names, roles) are typed by hand and
/// stay checked.
fn facts_record(raw: &str) -> String {
    let facts: crate::facts::Facts = serde_json::from_str(raw).expect("facts.json parses");
    let mut out = String::new();
    let titles = facts.components.iter().filter_map(|c| c.last_merge.as_ref()).map(|m| m.title.as_str());
    let titles = titles.chain(facts.decisions.iter().map(|d| d.title.as_str()));
    for t in titles.chain([facts.receipt.commit_title.as_str(), facts.receipt.description.as_str()]) {
        out.push_str(&squash(t));
        out.push('\n');
    }
    out
}

fn walk(dir: &Path, root: &Path, record: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read dist dir") {
        let path = entry.expect("dist entry").path();
        if path.is_dir() {
            walk(&path, root, record, out);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if !rel.ends_with(".html") {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {rel}"));
        for v in violations(&copy_text(&raw), record) {
            out.push(format!("{rel}: {v}"));
        }
    }
}

/// Every rendered page spells project, product and vendor names canonically.
pub fn verify_canonical_names(dist: &Path) {
    // Records quoted on the pages: the work ledger (/now/), the research status
    // table and the GitHub titles in facts.json.
    let mut record = squash(&fs::read_to_string(dist.join("data/now.tsv")).unwrap_or_default().replace('\t', " "));
    record.push('\n');
    record.push_str(&status_record(super::research_status::RAW));
    record.push_str(&facts_record(&fs::read_to_string(dist.join("facts.json")).expect("read dist/facts.json")));
    let mut out = Vec::new();
    walk(dist, dist, &record, &mut out);
    assert!(
        out.is_empty(),
        "Non-canonical names in site copy (see the naming table in AGENTS.md):\n{}",
        out.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(html: &str) -> Vec<String> {
        violations(&copy_text(html), "")
    }

    #[test]
    fn visible_wrong_spelling_fails() {
        assert_eq!(check("<p>Welcome to Aienos.</p>").len(), 1);
        assert_eq!(check("<p>AIEN proposes structure.</p>").len(), 1);
        assert_eq!(check("<h2>Say hello to Allen?</h2>").len(), 1);
        assert_eq!(check("<p>Read it on Github</p>").len(), 1);
        assert_eq!(check("<p>The Aienux kernel</p>").len(), 1);
    }

    #[test]
    fn urls_attributes_and_code_pass() {
        assert!(check(r#"<a href="https://github.com/aien-dev/Aienos">the repository</a>"#).is_empty());
        assert!(check("<p>Run <code>Aienos --help</code> first.</p>").is_empty());
        assert!(check("<p>See https://example.org/Aienos today.</p>").is_empty());
        assert!(check(r#"<p data-speaker="allen" id="allen-intro">Hello, I am Aien.</p>"#).is_empty());
        assert!(check("<p>Visit aienos.com or aien-dev on GitHub; nvidia.ko stays.</p>").is_empty());
    }

    #[test]
    fn verbatim_elements_titles_and_labels_pass() {
        assert!(check(r#"<p class="desc" data-verbatim>Aienux, the AienOS kernel</p>"#).is_empty());
        assert!(check("<cite>The AIEN Papers: The Complete Collection</cite>").is_empty());
        assert!(check("<h3>AIEN VERIFICATION</h3>").is_empty());
        assert_eq!(check("<p>AIEN 研究阶梯上的每一个实验</p>").len(), 1);
        // A lone capitalised word is not a label.
        assert_eq!(check("<strong>AIEN</strong> is a machine").len(), 1);
        // The verbatim element ends at its own close tag.
        assert_eq!(check("<p data-verbatim><p>x</p></p><p>Aienos</p>").len(), 1);
    }

    #[test]
    fn sentence_end_is_not_an_identifier_join() {
        assert_eq!(check("<p>It runs on Aienos.</p>").len(), 1);
        assert!(check("<p>It runs on aienos.com.</p>").is_empty());
    }

    #[test]
    fn ledger_records_pass_only_with_the_record() {
        let rec = "L1 2026-10-26 Drake: boot the Imac";
        assert!(violations("Drake: boot the Imac", rec).is_empty());
        assert_eq!(violations("Drake: boot the Imac", "").len(), 1);
        // Too little shared context: one word on its own proves nothing.
        assert_eq!(violations("Imac", rec).len(), 1);
        assert_eq!(violations("Boot your Imac now", rec).len(), 1);
    }

    #[test]
    fn github_titles_in_facts_are_a_record_but_roles_are_not() {
        // aien-protocols #60, merged 2026-10-28: the last merged change of a component.
        let facts = r#"{"generated_at":"2026-10-29T00:00:00Z",
            "components":[{"key":"protocols","name":"Protocols","repo":"aien-dev/aien-protocols",
              "role":"Allen speaks the protocols.","url":"","pushed_at":"","open_prs":0,
              "last_merge":{"number":60,"title":"rename: Allen -> Aien (pi-spark-task spec)","merged_at":"","url":""}}],
            "decisions":[],"receipt":{"repo":"","sha":"","commit_title":"","committed_at":"","state":"none","description":"","target_url":"","updated_at":""}}"#;
        let rec = facts_record(facts);
        let page = |rec: &str, body: &str| violations(&copy_text(body), rec);
        let merge = r#"<td><a class="merge" href="https://github.com/aien-dev/aien-protocols/pull/60">rename: Allen -&gt; Aien (pi-spark-task spec)</a></td>"#;
        assert!(page(&rec, merge).is_empty());
        assert_eq!(page("", merge).len(), 1);
        // The hand-written role is in facts.json but not a record.
        assert_eq!(page(&rec, "<td>Allen speaks the protocols.</td>").len(), 1);
        // New prose that only shares the bare name is still caught.
        assert_eq!(page(&rec, "<p>Say hello to Allen today.</p>").len(), 1);
    }
}
