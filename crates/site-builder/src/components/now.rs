//! /now/: what the people and agents of AIEN are doing right now.
//!
//! The one source is the coordination ledger, a tab separated file the
//! coordinator and every agent write to on the Spark (`~/handoffs/LEDGER.tsv`).
//! A small publisher pushes that file to the `data/now` branch of this
//! repository whenever it changes; the pages workflow lays it into
//! `public/data/now.tsv` before the build, and this module turns it into
//! the page and into `/now.json` for machines. Nothing here is written by
//! hand: if a line is not in the ledger, it is not on the page.
//!
//! Columns, in order: id, opened_utc, owner, status, item, done_when,
//! evidence, updated_utc. `evidence` carries the running notes, newest
//! last, separated by ` | `.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use maud::{html, Markup};
use serde::Serialize;

use super::boxlink::{foot_labelled, Target};

pub const TITLE: &str = "Now | AIEN OS";
pub const DESCRIPTION: &str = "The live coordination ledger of AIEN: what every agent is doing right now, what is settled with evidence, and what waits on an operator decision.";
pub const URL: &str = "https://www.aienos.com/now/";

/// Where the pages workflow lays the published ledger before a build.
pub const LEDGER_PATH: &str = "public/data/now.tsv";
/// Where the pages workflow lays the machine pulse (optional).
pub const PULSE_PATH: &str = "public/data/now-pulse.tsv";
/// The published ledger on GitHub; `#L<n>` names one line of it.
pub const LEDGER_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/now.tsv";

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    /// The 1-based line of now.tsv this item was read from (the header is line 1).
    pub line: usize,
    pub opened_utc: String,
    pub owner: String,
    pub owner_kind: &'static str,
    pub status: String,
    pub item: String,
    pub done_when: String,
    pub notes: Vec<String>,
    pub updated_utc: String,
    pub stream: &'static str,
    /// Seconds since the last write to this line, at build time.
    pub age_s: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pulse {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Now {
    pub built_utc: String,
    pub ledger_written_utc: String,
    pub items: Vec<Item>,
    pub pulse: Vec<Pulse>,
}

/// The streams, in display order. Classification is by the item's wording
/// and owner, the same words the coordinator uses when opening a line.
pub const STREAMS: [&str; 8] = [
    "Engine speed",
    "Alpha release",
    "AIEN Rig",
    "AIENOS on the iMac",
    "Research",
    "Public record",
    "PR desk",
    "Infrastructure",
];

fn classify(item: &str, owner: &str) -> &'static str {
    let t = item.to_ascii_lowercase();
    if t.starts_with("research")
        || t.contains("symposium")
        || t.contains("#583")
        || t.contains("#584")
        || t.contains("inventory for site")
    {
        "Research"
    } else if t.starts_with("agent:")
        || t.contains("cascade")
        || t.contains("decode")
        || t.contains("tok/s")
        || t.contains("flashnext")
        || t.contains("omega#")
    {
        "Engine speed"
    } else if t.starts_with("rig") {
        "AIEN Rig"
    } else if t.contains("aienos.com")
        || t.contains("org profile")
        || t.contains("readme")
        || t.contains("dashboard")
    {
        "Public record"
    } else if t.contains("alpha")
        || t.contains("install")
        || t.contains("serve-27b")
        || t.contains("27b server")
    {
        "Alpha release"
    } else if t.contains("imac")
        || t.contains("horizon")
        || t.contains("allen")
        || t.contains("aienos")
        || t.contains("bundle")
    {
        "AIENOS on the iMac"
    } else if owner == "desk"
        || t.starts_with("merge")
        || t.starts_with("queue")
        || t.contains("label")
        || t.contains("squash")
    {
        "PR desk"
    } else {
        "Infrastructure"
    }
}

fn owner_kind(owner: &str) -> &'static str {
    match owner {
        "drake" => "operator",
        "desk" => "desk",
        "coordinator" => "coordinator",
        "unassigned" => "unassigned",
        _ => "agent",
    }
}

/// Em and en dashes never reach the page, whatever an agent typed.
fn clean(s: &str) -> String {
    s.replace(['\u{2014}', '\u{2013}'], "-").trim().to_string()
}

/// "2026-10-19T16:43Z" or "2026-10-19T16:43:07Z" to seconds since the epoch.
/// Anything else gives None and the age reads as unknown.
pub fn parse_utc(s: &str) -> Option<i64> {
    let s = s.trim().trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let y: i64 = d.next()?.parse().ok()?;
    let m: i64 = d.next()?.parse().ok()?;
    let day: i64 = d.next()?.parse().ok()?;
    let mut t = time.split(':');
    let hh: i64 = t.next()?.parse().ok()?;
    let mm: i64 = t.next()?.parse().ok()?;
    let ss: i64 = t.next().map(|x| x.parse().unwrap_or(0)).unwrap_or(0);
    // Days from civil, Howard Hinnant's algorithm.
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3_600 + mm * 60 + ss)
}

fn fmt_utc(secs: i64) -> String {
    // Civil from days, the inverse of the above.
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60
    )
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn age_words(age_s: i64) -> String {
    if age_s < 0 {
        return "age unknown".into();
    }
    let m = age_s / 60;
    if m < 1 {
        "just now".into()
    } else if m < 60 {
        format!("{m} min ago")
    } else if m < 60 * 36 {
        let h = m / 60;
        if h == 1 {
            "1 hour ago".into()
        } else {
            format!("{h} hours ago")
        }
    } else {
        let d = m / 1440;
        if d == 1 {
            "1 day ago".into()
        } else {
            format!("{d} days ago")
        }
    }
}

/// How warm a line is: written within the last half hour, the last two
/// hours, today, or older. The page colours by this, never by opinion.
pub fn warmth(age_s: i64) -> &'static str {
    if age_s < 0 {
        "cold"
    } else if age_s < 30 * 60 {
        "hot"
    } else if age_s < 2 * 3600 {
        "warm"
    } else if age_s < 24 * 3600 {
        "cool"
    } else {
        "cold"
    }
}

pub fn parse_ledger(tsv: &str, at: i64) -> Vec<Item> {
    let mut out = Vec::new();
    for (n, line) in tsv.lines().enumerate() {
        if n == 0 && line.starts_with("id\t") {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        let get = |i: usize| f.get(i).map(|s| clean(s)).unwrap_or_default();
        let owner = get(2);
        let item = get(4);
        let updated = get(7);
        let age_s = parse_utc(&updated)
            .or_else(|| parse_utc(&get(1)))
            .map(|t| at - t)
            .unwrap_or(-1);
        let notes: Vec<String> = get(6)
            .split(" | ")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        out.push(Item {
            id: get(0),
            line: n + 1,
            opened_utc: get(1),
            owner_kind: owner_kind(&owner),
            stream: classify(&item, &owner),
            owner,
            status: get(3).to_ascii_lowercase(),
            item,
            done_when: get(5),
            notes,
            updated_utc: updated,
            age_s,
        });
    }
    out
}

fn parse_pulse(tsv: &str) -> Vec<Pulse> {
    tsv.lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| Pulse {
            key: clean(k),
            value: clean(v),
        })
        .collect()
}

/// Load from the laid-in files. A missing ledger is not an error: the page
/// then says plainly that nothing has been published yet.
pub fn load() -> Now {
    let at = now_secs();
    let tsv = fs::read_to_string(Path::new(LEDGER_PATH)).unwrap_or_default();
    let pulse = fs::read_to_string(Path::new(PULSE_PATH))
        .map(|s| parse_pulse(&s))
        .unwrap_or_default();
    let items = parse_ledger(&tsv, at);
    let written = items
        .iter()
        .filter_map(|i| parse_utc(&i.updated_utc))
        .max()
        .map(fmt_utc)
        .unwrap_or_else(|| "never".into());
    Now {
        built_utc: fmt_utc(at),
        ledger_written_utc: written,
        items,
        pulse,
    }
}

pub fn to_json(now: &Now) -> String {
    serde_json::to_string_pretty(now).unwrap_or_else(|_| "{}".into())
}

// ---------------------------------------------------------------- rendering

fn owner_label(it: &Item) -> String {
    match it.owner_kind {
        "operator" => "Drake".into(),
        "desk" => "PR desk".into(),
        "coordinator" => "Coordinator".into(),
        "unassigned" => "Unassigned".into(),
        _ => {
            // sa-3-dfa217d1 reads as "agent dfa217"; anything else stays as written.
            let parts: Vec<&str> = it.owner.split('-').collect();
            if parts.len() >= 3 && parts[0] == "sa" {
                format!("agent {}", &parts[2][..parts[2].len().min(6)])
            } else {
                format!("agent {}", it.owner)
            }
        }
    }
}

fn last_note(it: &Item) -> Option<&String> {
    it.notes.last()
}

/// The item's own line of the published ledger.
fn line_url(it: &Item) -> String {
    format!("{LEDGER_URL}#L{}", it.line)
}

fn line_foot(it: &Item) -> Markup {
    foot_labelled(
        &line_url(it),
        &format!("Ledger line {} on GitHub", it.line),
        Target::Evidence,
    )
}

/// The repository a bare `branch@sha` in a note belongs to when the note
/// names none: the stream's home. Streams with no one home get None and
/// such a reference stays plain text.
fn default_repo(stream: &str) -> Option<&'static str> {
    match stream {
        "Engine speed" | "Research" => Some("omega-mv"),
        "Alpha release" => Some("aien-sovereign-core-mv"),
        "AIEN Rig" => Some("aien-rig"),
        "AIENOS on the iMac" => Some("aienos-mv"),
        "Public record" => Some("aienos.com"),
        _ => None,
    }
}

/// A note with its references turned into links (see `linkify`).
fn note_markup(note: &str, stream: &str) -> Markup {
    let segs = linkify(note, default_repo(stream));
    html! {
        @for seg in &segs {
            @if let Some(href) = &seg.href {
                a class="now-ref" href=(href) target="_blank" rel="noopener noreferrer" { (seg.text) }
            } @else {
                (seg.text)
            }
        }
    }
}

// ---------------------------------------------------------------- linkify

/// A run of note text, linked or plain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seg {
    pub text: String,
    pub href: Option<String>,
}

/// Short repository names the ledger uses, and the GitHub name of each.
const REPOS: [(&str, &str); 19] = [
    ("omega", "omega-mv"),
    ("omega-mv", "omega-mv"),
    ("sc", "aien-sovereign-core-mv"),
    ("aien-sovereign-core", "aien-sovereign-core-mv"),
    ("aien-sovereign-core-mv", "aien-sovereign-core-mv"),
    ("aienos", "aienos-mv"),
    ("aienos-mv", "aienos-mv"),
    ("arch", "aien-architecture-mv"),
    ("aien-architecture", "aien-architecture-mv"),
    ("aien-architecture-mv", "aien-architecture-mv"),
    ("physics", "physics-mv"),
    ("physics-mv", "physics-mv"),
    ("aien-protocols", "aien-protocols-mv"),
    ("aien-protocols-mv", "aien-protocols-mv"),
    ("benchmarks", "benchmarks-mv"),
    ("benchmarks-mv", "benchmarks-mv"),
    ("aien-rig", "aien-rig"),
    ("aienos.com", "aienos.com"),
    (".github", ".github"),
];

fn repo_name(short: &str) -> Option<&'static str> {
    REPOS
        .iter()
        .find(|(s, _)| *s == short)
        .map(|(_, full)| *full)
}

fn is_sha(s: &str) -> bool {
    (7..=40).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn is_branch(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('/')
        && !s.ends_with('/')
        && s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '/' | '.' | '_' | '-')
        })
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '@' | '#' | '-')
}

/// What one token refers to, if anything.
enum Ref {
    Issue { repo: &'static str, n: String },
    Commit { branch: String, sha: String },
    Evidence(String),
    Repo(&'static str),
    Url,
}

fn classify_token(tok: &str) -> Option<Ref> {
    if tok.starts_with("https://") || tok.starts_with("http://") {
        return Some(Ref::Url);
    }
    if let Some((repo, n)) = tok.rsplit_once('#') {
        let repo = repo.strip_prefix("aien-dev/").unwrap_or(repo);
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            if let Some(full) = repo_name(repo) {
                return Some(Ref::Issue {
                    repo: full,
                    n: n.to_string(),
                });
            }
        }
        return None;
    }
    if let Some((branch, sha)) = tok.split_once('@') {
        if is_branch(branch) && is_sha(sha) {
            return Some(Ref::Commit {
                branch: branch.to_string(),
                sha: sha.to_string(),
            });
        }
        return None;
    }
    if let Some(rest) = tok.strip_prefix("evidence/") {
        if !rest.is_empty() && !rest.contains("..") {
            return Some(Ref::Evidence(tok.to_string()));
        }
        return None;
    }
    repo_name(tok).map(Ref::Repo)
}

/// Turns the references in a ledger note into links and leaves everything
/// else as it was:
/// - `aien-dev/<repo>#<n>` and `<repo>#<n>` go to the issue or pull request
///   (GitHub serves either at /issues/<n>);
/// - `<branch>@<sha>` goes to that commit in the repository named earlier
///   in the note, else the stream's home repository, else stays text;
/// - `evidence/<path>` goes to that folder on the branch of the first
///   `branch@sha` in the note (the agent's own branch leads a note; a
///   branch in parentheses is a dependency), else stays text;
/// - a bare https URL links to itself.
///
/// Trailing punctuation stays outside the link. Anything that does not
/// match exactly (an upstream model name with a revision, a mail address,
/// `task#3`) is left alone.
pub fn linkify(note: &str, default_repo: Option<&str>) -> Vec<Seg> {
    // Tokens: maximal runs of token characters, with position.
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in note.char_indices() {
        match (is_token_char(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                tokens.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        tokens.push((s, note.len()));
    }
    // A URL runs to the next space, not just the next non-token character.
    let mut toks: Vec<(usize, usize)> = Vec::new();
    let mut skip_until = 0;
    for &(s, e) in &tokens {
        if s < skip_until {
            continue;
        }
        if note[s..].starts_with("https://") || note[s..].starts_with("http://") {
            let end = note[s..]
                .find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '>')
                .map(|k| s + k)
                .unwrap_or(note.len());
            toks.push((s, end));
            skip_until = end;
        } else {
            toks.push((s, e));
        }
    }
    // Strip trailing punctuation off each token.
    let toks: Vec<(usize, usize)> = toks
        .into_iter()
        .map(|(s, e)| {
            let t = &note[s..e];
            let trimmed = t.trim_end_matches(['.', ',', ';', ':']);
            (s, s + trimmed.len())
        })
        .filter(|(s, e)| e > s)
        .collect();

    // First pass: every branch@sha with the repository it resolves to.
    let mut last_repo: Option<&str> = default_repo;
    let mut commits: Vec<(usize, String, String)> = Vec::new(); // (pos, repo, branch)
    let mut refs: Vec<Option<(String, usize, usize)>> = Vec::new(); // (href, s, e)
    for &(s, e) in &toks {
        let tok = &note[s..e];
        match classify_token(tok) {
            Some(Ref::Repo(r)) => {
                last_repo = Some(r);
                refs.push(None);
            }
            Some(Ref::Issue { repo, n }) => refs.push(Some((
                format!("https://github.com/aien-dev/{repo}/issues/{n}"),
                s,
                e,
            ))),
            Some(Ref::Commit { branch, sha }) => match last_repo {
                Some(r) => {
                    commits.push((s, r.to_string(), branch));
                    refs.push(Some((
                        format!("https://github.com/aien-dev/{r}/commit/{sha}"),
                        s,
                        e,
                    )));
                }
                None => refs.push(None),
            },
            Some(Ref::Url) => refs.push(Some((tok.to_string(), s, e))),
            Some(Ref::Evidence(_)) | None => refs.push(None),
        }
    }
    // Second pass: evidence paths resolve against the note's first branch.
    for (k, &(s, e)) in toks.iter().enumerate() {
        let tok = &note[s..e];
        if let Some(Ref::Evidence(path)) = classify_token(tok) {
            if let Some((_, repo, branch)) = commits.first() {
                let path = path.trim_end_matches('/');
                refs[k] = Some((
                    format!("https://github.com/aien-dev/{repo}/tree/{branch}/{path}"),
                    s,
                    e,
                ));
            }
        }
    }
    // Emit.
    let mut out: Vec<Seg> = Vec::new();
    let mut cursor = 0;
    for r in refs.into_iter().flatten() {
        let (href, s, e) = r;
        if s > cursor {
            out.push(Seg {
                text: note[cursor..s].to_string(),
                href: None,
            });
        }
        out.push(Seg {
            text: note[s..e].to_string(),
            href: Some(href),
        });
        cursor = e;
    }
    if cursor < note.len() {
        out.push(Seg {
            text: note[cursor..].to_string(),
            href: None,
        });
    }
    out
}

fn card(it: &Item) -> Markup {
    let w = warmth(it.age_s);
    html! {
        article class=(format!("now-card now-{w} now-k-{}", it.owner_kind)) id=(it.id) {
            header class="now-card-top" {
                span class="now-id" { (it.id) }
                span class=(format!("now-owner now-owner-{}", it.owner_kind)) { (owner_label(it)) }
                span class="now-age" title=(it.updated_utc) {
                    span class="now-dot" aria-hidden="true" {}
                    (age_words(it.age_s))
                }
            }
            h3 class="now-item" { (it.item) }
            @if let Some(n) = last_note(it) {
                p class="now-note" { (note_markup(n, it.stream)) }
            }
            @if !it.done_when.is_empty() {
                p class="now-done" { span class="now-done-k" { "done when" } " " (it.done_when) }
            }
            (line_foot(it))
        }
    }
}

fn river(items: &[&Item]) -> Markup {
    // One segment per line, warm to cold, newest first: the stream's pulse at a glance.
    let mut sorted: Vec<&Item> = items.to_vec();
    sorted.sort_by_key(|i| if i.age_s < 0 { i64::MAX } else { i.age_s });
    html! {
        div class="now-river" aria-hidden="true" {
            @for it in sorted {
                span class=(format!("now-seg now-{}", warmth(it.age_s))) title=(format!("{} {}", it.id, age_words(it.age_s))) {}
            }
        }
    }
}

fn stat(n: usize, label: &str, class: &str) -> Markup {
    html! {
        div class=(format!("now-stat {class}")) {
            span class="now-stat-n" { (n) }
            span class="now-stat-l" { (label) }
        }
    }
}

pub fn render_now_page(now: &Now) -> Markup {
    let open: Vec<&Item> = now.items.iter().filter(|i| i.status == "open").collect();
    let settled: Vec<&Item> = now
        .items
        .iter()
        .filter(|i| i.status == "done" || i.status == "dropped")
        .collect();
    let hands: usize = open
        .iter()
        .filter(|i| i.owner_kind == "agent" && i.age_s >= 0 && i.age_s < 2 * 3600)
        .map(|i| i.owner.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let for_drake: Vec<&Item> = open
        .iter()
        .copied()
        .filter(|i| i.owner_kind == "operator")
        .collect();
    let next: Vec<&Item> = open
        .iter()
        .copied()
        .filter(|i| {
            i.owner_kind != "operator" && (i.owner_kind == "unassigned" || i.notes.is_empty())
        })
        .collect();
    let moving: Vec<&Item> = open
        .iter()
        .copied()
        .filter(|i| {
            i.owner_kind != "operator" && !(i.owner_kind == "unassigned" || i.notes.is_empty())
        })
        .collect();
    let settled_today = settled
        .iter()
        .filter(|i| i.age_s >= 0 && i.age_s < 24 * 3600)
        .count();
    let mut settled_sorted = settled.clone();
    settled_sorted.sort_by_key(|i| if i.age_s < 0 { i64::MAX } else { i.age_s });

    html! {
        section class="now-hero" aria-labelledby="now-title" {
            div class="container" {
                p class="kicker" { "Now" }
                h1 id="now-title" class="now-title" { "What is being done, right now." }
                p class="now-lead" {
                    "This is the working ledger of AIEN, published as it is written. Every line is a promise with an owner and an observable finish. Agents write their progress here; the operator writes decisions here. Nothing on this page is composed by hand."
                }
                @if now.items.is_empty() {
                    p class="now-empty" { "No ledger has been published yet. The page will fill itself the moment one arrives." }
                } @else {
                    div class="now-stats" {
                        (stat(hands, "agents writing in the last two hours", "now-stat-hot"))
                        (stat(moving.len(), "lines in motion", ""))
                        (stat(next.len(), "lines queued", ""))
                        (stat(settled_today, "settled in the last day", "now-stat-done"))
                        (stat(for_drake.len(), "waiting on the operator", "now-stat-op"))
                    }
                    p class="now-stamp" {
                        "Ledger last written " span class="mono" { (now.ledger_written_utc) }
                        ". Page built " span class="mono" { (now.built_utc) }
                        ". It rebuilds within the hour of every change."
                    }
                    @if !now.pulse.is_empty() {
                        ul class="now-pulse" {
                            @for p in &now.pulse {
                                li { span class="now-pulse-k" { (p.key) } span class="now-pulse-v mono" { (p.value) } }
                            }
                        }
                    }
                }
            }
        }

        @if !for_drake.is_empty() {
            section class="now-section now-op-section" id="operator" aria-labelledby="now-op-h" {
                div class="container" {
                    p class="kicker" { "On the operator's desk" }
                    h2 id="now-op-h" class="now-h2" { "Decisions Drake is making" }
                    p class="now-sub" { "Work that cannot move until a human answers. Each line names the answer it needs." }
                    div class="now-grid" {
                        @for it in &for_drake { (card(it)) }
                    }
                }
            }
        }

        section class="now-section" id="streams" aria-labelledby="now-streams-h" {
            div class="container" {
                p class="kicker" { "In motion" }
                h2 id="now-streams-h" class="now-h2" { "What the agents are doing" }
                p class="now-sub" { "Grouped by stream. The bar under each name is the stream's pulse: one segment per line, bright when written in the last half hour, fading as it cools." }
                @for s in STREAMS {
                    @let in_stream: Vec<&Item> = moving.iter().copied().filter(|i| i.stream == s).collect();
                    @if !in_stream.is_empty() {
                        div class="now-stream" id=(format!("stream-{}", s.to_ascii_lowercase().replace(' ', "-"))) {
                            header class="now-stream-head" {
                                h3 class="now-stream-name" { (s) }
                                span class="now-stream-n" { (in_stream.len()) " " @if in_stream.len() == 1 { "line" } @else { "lines" } }
                            }
                            (river(&in_stream))
                            div class="now-grid" {
                                @for it in &in_stream { (card(it)) }
                            }
                        }
                    }
                }
            }
        }

        @if !next.is_empty() {
            section class="now-section" id="next" aria-labelledby="now-next-h" {
                div class="container" {
                    p class="kicker" { "Next" }
                    h2 id="now-next-h" class="now-h2" { "What is queued" }
                    p class="now-sub" { "Promised, with a finish line, not yet picked up or not yet reported on." }
                    ol class="now-queue" {
                        @for it in &next {
                            li class="now-queue-row" id=(it.id) {
                                span class="now-id" { (it.id) }
                                span class="now-queue-item" { (it.item) }
                                span class=(format!("now-owner now-owner-{}", it.owner_kind)) { (owner_label(it)) }
                                @if !it.done_when.is_empty() {
                                    span class="now-queue-done" { "done when " (it.done_when) }
                                }
                                (line_foot(it))
                            }
                        }
                    }
                }
            }
        }

        @if !settled_sorted.is_empty() {
            section class="now-section" id="settled" aria-labelledby="now-settled-h" {
                div class="container" {
                    p class="kicker" { "Settled" }
                    h2 id="now-settled-h" class="now-h2" { "Decisions made, with their evidence" }
                    p class="now-sub" { "A line closes on evidence (a commit, a receipt, a file) or is dropped for a stated reason. Newest first." }
                    ol class="now-timeline" {
                        @for it in &settled_sorted {
                            li class=(format!("now-tl now-tl-{}", it.status)) id=(it.id) {
                                span class="now-tl-when mono" title=(it.updated_utc) { (age_words(it.age_s)) }
                                div class="now-tl-body" {
                                    span class=(format!("now-verdict now-verdict-{}", it.status)) { (it.status) }
                                    span class="now-id" { (it.id) }
                                    p class="now-tl-item" { (it.item) }
                                    @if let Some(n) = last_note(it) {
                                        p class="now-tl-evidence" { (note_markup(n, it.stream)) }
                                    }
                                    (line_foot(it))
                                }
                            }
                        }
                    }
                }
            }
        }

        section class="now-section now-how" id="how" aria-labelledby="now-how-h" {
            div class="container" {
                p class="kicker" { "How this page is made" }
                h2 id="now-how-h" class="now-h2" { "One file, no hands" }
                p class="now-sub" {
                    "The coordinator and every agent append to one tab separated file on the Spark. A publisher pushes that file to the "
                    a class="prose-link" href="https://github.com/aien-dev/aienos.com/tree/data/now" { "data/now branch" }
                    " whenever it changes. The site builder reads it at build time and writes this page and "
                    a class="prose-link" href="/now.json" { "/now.json" }
                    ". A line that is not in the file is not here. Every card, queue row and settled entry links to its own line of that file, and the commits, pull requests and evidence folders named in a note link to GitHub. Owners shown as \"agent\" are autonomous coding agents; the operator is one person."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "id\topened_utc\towner\tstatus\titem\tdone_when\tevidence\tupdated_utc\n\
L001\t2026-10-19T16:43Z\tsa-0-abc123def\topen\tAgent: w4-gemv\thandoff written\tstarted | chip PASS 15.63 tok/s\t2026-10-19T18:00Z\n\
L002\t2026-10-19T16:43Z\tdrake\topen\tDecide: register the speed gate\tDrake answer recorded\t\t2026-10-19T16:43Z\n\
L003\t2026-10-19T16:43Z\tdesk\tdone\tCreate label alpha-tester\tlabel exists\tgh label list shows it \u{2014} done, aien-dev/aien-sovereign-core-mv#8\t2026-10-19T17:00Z\n\
L004\t2026-10-19T16:43Z\tunassigned\topen\tResearch: omega#583 simulator\tPR merged\t\t2026-10-19T16:43Z\n";

    #[test]
    fn parses_and_classifies() {
        let items = parse_ledger(SAMPLE, parse_utc("2026-10-19T18:10Z").unwrap());
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].line, 2, "the header is line 1");
        assert_eq!(items[3].line, 5);
        assert_eq!(items[0].stream, "Engine speed");
        assert_eq!(items[0].owner_kind, "agent");
        assert_eq!(items[0].age_s, 600);
        assert_eq!(items[0].notes.len(), 2);
        assert_eq!(items[1].owner_kind, "operator");
        assert_eq!(items[3].stream, "Research");
        assert!(
            !items[2].notes[0].contains('\u{2014}'),
            "dash must be cleaned"
        );
    }

    #[test]
    fn time_round_trips() {
        for s in [
            "2026-10-19T16:43Z",
            "2000-02-29T00:00Z",
            "2026-01-01T23:59Z",
        ] {
            assert_eq!(fmt_utc(parse_utc(s).unwrap()), s);
        }
        assert_eq!(age_words(0), "just now");
        assert_eq!(age_words(59 * 60), "59 min ago");
        assert_eq!(age_words(3 * 3600), "3 hours ago");
        assert_eq!(warmth(10 * 60), "hot");
        assert_eq!(warmth(90 * 60), "warm");
    }

    #[test]
    fn page_holds_invariants() {
        let at = parse_utc("2026-10-19T18:10Z").unwrap();
        let now = Now {
            built_utc: fmt_utc(at),
            ledger_written_utc: "2026-10-19T18:00Z".into(),
            items: parse_ledger(SAMPLE, at),
            pulse: vec![Pulse {
                key: "memory free".into(),
                value: "50 GiB".into(),
            }],
        };
        let page = render_now_page(&now).into_string();
        for must in [
            "id=\"operator\"",
            "id=\"streams\"",
            "id=\"next\"",
            "id=\"settled\"",
            "Engine speed",
            "15.63 tok/s",
            "now-verdict-done",
            "L004",
        ] {
            assert!(page.contains(must), "missing {must}");
        }
        assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'));
        // Every card, queue row and timeline entry links to its own ledger line.
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(4));
        for line in 2..=5 {
            assert!(
                page.contains(&format!("href=\"{LEDGER_URL}#L{line}\"")),
                "line {line} link missing"
            );
        }
        assert!(page.contains("Ledger line 2 on GitHub"));
        // The pull request named in L003's note became a link in the timeline.
        assert!(page.contains("class=\"now-ref\" href=\"https://github.com/aien-dev/aien-sovereign-core-mv/issues/8\""), "note reference not linked");
        let json = to_json(&now);
        assert!(json.contains("\"L001\"") && json.contains("\"stream\": \"Engine speed\""));
    }

    #[test]
    fn empty_ledger_is_honest() {
        let now = Now {
            built_utc: "2026-10-19T18:10Z".into(),
            ledger_written_utc: "never".into(),
            items: vec![],
            pulse: vec![],
        };
        let page = render_now_page(&now).into_string();
        assert!(page.contains("No ledger has been published yet"));
    }

    fn hrefs(segs: &[Seg]) -> Vec<(String, String)> {
        segs.iter()
            .filter_map(|s| s.href.as_ref().map(|h| (s.text.clone(), h.clone())))
            .collect()
    }

    fn joined(segs: &[Seg]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn linkify_issues_and_pull_requests() {
        let segs = linkify(
            "Research: omega#583 simulator; aien-dev/aienos.com#44 merged, arch#276.",
            Some("omega-mv"),
        );
        assert_eq!(
            hrefs(&segs),
            vec![
                (
                    "omega#583".into(),
                    "https://github.com/aien-dev/omega-mv/issues/583".into()
                ),
                (
                    "aien-dev/aienos.com#44".into(),
                    "https://github.com/aien-dev/aienos.com/issues/44".into()
                ),
                (
                    "arch#276".into(),
                    "https://github.com/aien-dev/aien-architecture-mv/issues/276".into()
                ),
            ]
        );
        assert_eq!(
            joined(&segs),
            "Research: omega#583 simulator; aien-dev/aienos.com#44 merged, arch#276."
        );
    }

    #[test]
    fn linkify_commit_uses_the_repo_named_earlier_in_the_note() {
        let segs = linkify(
            "sc alpha/ceiling-and-model@56d42869 commit 39e79fad docs (omega rig/speech@3aebf9ee),",
            None,
        );
        assert_eq!(
            hrefs(&segs),
            vec![
                (
                    "alpha/ceiling-and-model@56d42869".into(),
                    "https://github.com/aien-dev/aien-sovereign-core-mv/commit/56d42869".into()
                ),
                (
                    "rig/speech@3aebf9ee".into(),
                    "https://github.com/aien-dev/omega-mv/commit/3aebf9ee".into()
                ),
            ]
        );
        assert_eq!(
            segs.last().unwrap().text,
            "),",
            "trailing punctuation stays outside the link"
        );
    }

    #[test]
    fn linkify_commit_falls_back_to_the_stream_repo_or_stays_text() {
        let segs = linkify("feat/goodbye-panel-off@2954f5e pushed", Some("aienos-mv"));
        assert_eq!(
            hrefs(&segs),
            vec![(
                "feat/goodbye-panel-off@2954f5e".into(),
                "https://github.com/aien-dev/aienos-mv/commit/2954f5e".into()
            )]
        );
        let segs = linkify("feat/goodbye-panel-off@2954f5e pushed", None);
        assert!(
            hrefs(&segs).is_empty(),
            "no repository to resolve against: stays text"
        );
        assert_eq!(joined(&segs), "feat/goodbye-panel-off@2954f5e pushed");
    }

    #[test]
    fn linkify_evidence_folder_on_the_branch_in_the_same_note() {
        let segs = linkify(
            "receipts alpha/fresh-account-evidence@997d22bd evidence/alpha-fresh-account-20261019/; see evidence/other/x.json.",
            Some("aien-sovereign-core-mv"),
        );
        let h = hrefs(&segs);
        assert_eq!(h[1], ("evidence/alpha-fresh-account-20261019/".into(), "https://github.com/aien-dev/aien-sovereign-core-mv/tree/alpha/fresh-account-evidence/evidence/alpha-fresh-account-20261019".into()));
        assert_eq!(h[2], ("evidence/other/x.json".into(), "https://github.com/aien-dev/aien-sovereign-core-mv/tree/alpha/fresh-account-evidence/evidence/other/x.json".into()));
        assert_eq!(joined(&segs), "receipts alpha/fresh-account-evidence@997d22bd evidence/alpha-fresh-account-20261019/; see evidence/other/x.json.");
        // Without a branch in the note the path stays text.
        let segs = linkify(
            "evidence/symbolic-rewrite/VERDICT.md written",
            Some("omega-mv"),
        );
        assert!(hrefs(&segs).is_empty());
        // The first branch wins over one in parentheses: the Rig's own branch, not Omega's.
        let segs = linkify("aien-rig speech/v0@af71f46 (omega rig/speech@3aebf9ee); evidence/speech/2026-10-19-gb10/README.md", Some("aien-rig"));
        assert_eq!(hrefs(&segs)[2].1, "https://github.com/aien-dev/aien-rig/tree/speech/v0/evidence/speech/2026-10-19-gb10/README.md");
    }

    #[test]
    fn linkify_bare_urls() {
        let segs = linkify("live: https://www.aienos.com/now/ and /now.json", None);
        assert_eq!(
            hrefs(&segs),
            vec![(
                "https://www.aienos.com/now/".into(),
                "https://www.aienos.com/now/".into()
            )]
        );
    }

    #[test]
    fn linkify_leaves_lookalikes_alone() {
        // An upstream model name with a revision, a mail address, a number
        // with a hash that names no repository, a bare sha, a dotted version.
        for plain in [
            "NVFP4 nvidia/Qwen3.8-Flash-Next-NVFP4@fc694b54 132.7GB",
            "mail drake@example.com",
            "task#3 and #276 alone",
            "worktree omega-research-584 @a8ffd2b9",
            "v0.1.0 tagged, pid 3643684",
            "z-lab/Qwen3.8-27B-DFlash2 rev 50307d4c",
        ] {
            let segs = linkify(plain, Some("omega-mv"));
            assert!(
                hrefs(&segs).is_empty(),
                "{plain:?} must stay plain: {:?}",
                hrefs(&segs)
            );
            assert_eq!(joined(&segs), plain);
        }
    }

    #[test]
    fn notes_render_links_with_escaping_intact() {
        let m = note_markup("omega#583 <b>x</b> & done", "Research").into_string();
        assert!(m.contains(
            "<a class=\"now-ref\" href=\"https://github.com/aien-dev/omega-mv/issues/583\""
        ));
        assert!(m.contains("&lt;b&gt;x&lt;/b&gt; &amp; done"), "{m}");
    }
}
