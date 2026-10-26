//! /now/: what the people and agents of AIEN are doing right now.
//!
//! The one private source is the coordination ledger, a tab separated file the
//! coordinator and agents write to on the Spark (`~/handoffs/LEDGER.tsv`).
//! The publisher projects it into a line-preserving, privacy-screened TSV
//! before pushing `data/now`. Pages refuses an unprojected input before
//! copying it into `public/data/now.tsv`; this module renders that public
//! projection as the page and `/now.json`. Redaction changes private text,
//! never the row count or source line links.
//!
//! Columns, in order: id, opened_utc, owner, status, item, done_when,
//! evidence, updated_utc. `evidence` carries the running notes, newest
//! last, separated by ` | `.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use super::boxlink::{foot_labelled, Target};

pub const TITLE: &str = "Now | AienOS";
pub const DESCRIPTION: &str = "The live coordination ledger of AIEN: what every agent is doing right now, what is settled with evidence, and what waits on an operator decision.";
pub const URL: &str = "https://www.aienos.com/now/";

/// Where the pages workflow lays the published ledger before a build.
pub const LEDGER_PATH: &str = "public/data/now.tsv";
/// Where the pages workflow lays the machine pulse (optional).
pub const PULSE_PATH: &str = "public/data/now-pulse.tsv";
/// The published ledger on GitHub; `#L<n>` names one line of it.
pub const LEDGER_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/now.tsv";

/// Where the pages workflow lays the Spark's reading of Drake's Claude plan.
pub const USAGE_PATH: &str = "public/data/claude-usage.tsv";
/// The same reading on GitHub, the evidence behind the usage card.
pub const USAGE_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/claude-usage.tsv";
/// Where a visitor joins in: the public Open Humanity repository (issues and
/// discussions open), for agents working on their own plans and Sparks
/// qualifying the same gates.
pub const HUMANITY_URL: &str = "https://github.com/aien-dev/open-humanity";

/// One limit of the Claude plan: how much of it is used, and when it resets.
#[derive(Debug, Clone, Serialize)]
pub struct UsageLimit {
    pub label: String,
    pub used_pct: u32,
    pub resets_utc: String,
}

/// The Spark's reading of Drake's Claude plan meter. The publisher writes
/// `checked`, `plan` and `limit` rows (kind, percent used, reset time,
/// model scope); it publishes percentages only, never the login.
#[derive(Debug, Clone, Serialize)]
pub struct Usage {
    pub checked_utc: String,
    pub plan: String,
    pub limits: Vec<UsageLimit>,
}

/// Plain words and a safe character set for a scope the meter names.
fn usage_words(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '.').take(40).collect::<String>().trim().to_string()
}

pub fn parse_usage(tsv: &str) -> Option<Usage> {
    let mut checked = None;
    let mut plan = String::new();
    let mut limits = Vec::new();
    for line in tsv.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["checked", t, ..] => checked = parse_utc(t).map(fmt_utc),
            ["plan", p, ..] => plan = usage_words(p),
            ["limit", kind, pct, resets, rest @ ..] => {
                let Ok(used) = pct.trim().parse::<f64>() else { continue };
                let scope = rest.first().map(|s| usage_words(s)).unwrap_or_default();
                let label = match (*kind, scope.is_empty()) {
                    ("session", _) => "This five-hour session".to_string(),
                    ("weekly_all", _) => "This week, all models".to_string(),
                    ("weekly_scoped", false) => format!("This week, {scope} only"),
                    _ => continue,
                };
                let resets_utc = parse_utc(resets).map(fmt_utc).unwrap_or_default();
                limits.push(UsageLimit { label, used_pct: used.round().clamp(0.0, 100.0) as u32, resets_utc });
            }
            _ => {}
        }
    }
    let checked_utc = checked?;
    if limits.is_empty() {
        return None;
    }
    Some(Usage { checked_utc, plan: if plan.is_empty() { "Claude".into() } else { plan }, limits })
}

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


/// Where the pages workflow lays the Spark's model check (optional).
/// Lines: `checked<TAB>utc`, then one `model<TAB>served id` per model that
/// answered the Spark's local check. No ports, no addresses.
pub const MODELS_PATH: &str = "public/data/models.tsv";
/// Which models the plan names, their roles and their measured figures.
pub const ROSTER: &str = include_str!("../../data/models.json");

#[derive(Debug, Clone, Deserialize)]
struct Roster {
    plan: Link,
    models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Figure {
    pub label: String,
    pub value: String,
    pub receipt: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelEntry {
    key: String,
    name: String,
    /// Lowercase text a served id must contain to count as this model.
    served_match: Option<String>,
    /// What the card says when no served id matches: not-answering, planned or not-trained.
    absent: String,
    track: String,
    role: String,
    plan_ledger: Option<String>,
    source: Option<Link>,
    figures: Vec<Figure>,
}

/// One model as the page shows it, after the Spark's check.
#[derive(Debug, Clone, Serialize)]
pub struct Model {
    pub key: String,
    pub name: String,
    /// answering, not-answering, planned, not-trained or unknown.
    pub state: String,
    pub served_as: Option<String>,
    pub track: String,
    pub role: String,
    pub plan_ledger: Option<String>,
    pub source: Option<Link>,
    pub figures: Vec<Figure>,
}

/// The check: when the Spark last looked, and which served ids answered.
pub fn parse_models_check(tsv: &str) -> (Option<String>, Vec<String>) {
    let mut checked = None;
    let mut ids = Vec::new();
    for (k, v) in tsv.lines().filter_map(|l| l.split_once('\t')) {
        match k.trim() {
            "checked" if parse_utc(v).is_some() => checked = Some(v.trim().to_string()),
            "model" if !v.trim().is_empty() => ids.push(clean(v)),
            _ => {}
        }
    }
    (checked, ids)
}

/// Join the roster with the check. With no check at all, a model that
/// can be served reads as unknown, never as answering or down.
pub fn models_from(roster_json: &str, checked: &Option<String>, ids: &[String]) -> (Link, Vec<Model>) {
    let roster: Roster = serde_json::from_str(roster_json).expect("data/models.json must parse");
    let models = roster
        .models
        .into_iter()
        .map(|m| {
            let served_as = m
                .served_match
                .as_deref()
                .and_then(|pat| ids.iter().find(|id| id.to_ascii_lowercase().contains(pat)).cloned());
            let state = match (&served_as, &m.served_match, checked) {
                (Some(_), _, _) => "answering".to_string(),
                (None, Some(_), None) if m.absent == "not-answering" => "unknown".to_string(),
                _ => m.absent.clone(),
            };
            Model {
                key: m.key,
                name: m.name,
                state,
                served_as,
                track: m.track,
                role: m.role,
                plan_ledger: m.plan_ledger,
                source: m.source,
                figures: m.figures,
            }
        })
        .collect();
    (roster.plan, models)
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
    /// When the Spark last checked its model servers, or "never".
    pub models_checked_utc: String,
    pub plan: Link,
    pub models: Vec<Model>,
    /// The last reading of Drake's Claude plan, if the Spark has published one.
    pub claude: Option<Usage>,
}

/// The streams, in display order. Classification is by the item's wording
/// and owner, the same words the coordinator uses when opening a line.
pub const STREAMS: [&str; 8] = [
    "Engine speed",
    "Alpha release",
    "AIEN Rig",
    "AienOS on the iMac",
    "Research",
    "Public record",
    "PR desk",
    "Infrastructure",
];

fn classify(item: &str, owner: &str) -> &'static str {
    let t = item.to_ascii_lowercase();
    if t.starts_with("research") || t.contains("symposium") || t.contains("#583") || t.contains("#584") || t.contains("inventory for site") {
        "Research"
    } else if t.starts_with("agent:") || t.contains("cascade") || t.contains("decode") || t.contains("tok/s") || t.contains("flashnext") || t.contains("omega#") {
        "Engine speed"
    } else if t.starts_with("rig") {
        "AIEN Rig"
    } else if t.contains("aienos.com") || t.contains("org profile") || t.contains("readme") || t.contains("dashboard") {
        "Public record"
    } else if t.contains("alpha") || t.contains("install") || t.contains("serve-27b") || t.contains("27b server") {
        "Alpha release"
    } else if t.contains("imac") || t.contains("horizon") || t.contains("allen") || t.contains("aienos") || t.contains("bundle") {
        "AienOS on the iMac"
    } else if owner == "desk" || t.starts_with("merge") || t.starts_with("queue") || t.contains("label") || t.contains("squash") {
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

pub fn fmt_utc(secs: i64) -> String {
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
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60)
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
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
        if h == 1 { "1 hour ago".into() } else { format!("{h} hours ago") }
    } else {
        let d = m / 1440;
        if d == 1 { "1 day ago".into() } else { format!("{d} days ago") }
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
        let age_s = parse_utc(&updated).or_else(|| parse_utc(&get(1))).map(|t| at - t).unwrap_or(-1);
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
        .map(|(k, v)| Pulse { key: clean(k), value: clean(v) })
        .collect()
}

/// Load from the laid-in files. A missing ledger is not an error: the page
/// then says plainly that nothing has been published yet.
pub fn load() -> Now {
    let at = now_secs();
    let tsv = fs::read_to_string(Path::new(LEDGER_PATH)).unwrap_or_default();
    let pulse = fs::read_to_string(Path::new(PULSE_PATH)).map(|s| parse_pulse(&s)).unwrap_or_default();
    let items = parse_ledger(&tsv, at);
    let written = items
        .iter()
        .filter_map(|i| parse_utc(&i.updated_utc))
        .max()
        .map(fmt_utc)
        .unwrap_or_else(|| "never".into());
    let (checked, ids) = fs::read_to_string(Path::new(MODELS_PATH)).map(|s| parse_models_check(&s)).unwrap_or_default();
    let (plan, models) = models_from(ROSTER, &checked, &ids);
    let models_checked_utc = checked.unwrap_or_else(|| "never".into());
    let claude = fs::read_to_string(Path::new(USAGE_PATH)).ok().and_then(|s| parse_usage(&s));
    Now { built_utc: fmt_utc(at), ledger_written_utc: written, items, pulse, models_checked_utc, plan, models, claude }
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
            // Keeper, and the retired Renda workers (renda, renda1, renda2), read as Keeper.
            if let Some(n) = it.owner.strip_prefix("keeper").or_else(|| it.owner.strip_prefix("renda")).filter(|n| n.chars().all(|c| c.is_ascii_digit())) {
                return if n.is_empty() { "Keeper".into() } else { format!("Keeper {n}") };
            }
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

/// Keeper, the merge desk, was called Renda until 2026-10-24 (Drake). The page
/// shows the new name in ledger prose; the ledger on GitHub keeps every line as
/// written. Paths, file names and identifiers that contain the old name (a
/// slash, dot, underscore or letter next to it) stay as they are so they still
/// point at real things: `renda1` reads Keeper's `keeper1`, `~/renda-queue/` stays.
pub fn keeper_name(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let hit = i + 5 <= b.len() && b[i..i + 5].eq_ignore_ascii_case(b"renda");
        // A capital R after a lowercase letter, or before a capital, is a word
        // boundary an agent squashed ("soleRenda", "RendaL708"), not an identifier.
        let capital = b[i] == b'R';
        let before_ok = i == 0 || {
            let p = b[i - 1];
            !(p.is_ascii_uppercase() || p.is_ascii_digit() || b"/._~-".contains(&p)) && (!p.is_ascii_lowercase() || capital)
        };
        let after_ok = i + 5 >= b.len() || {
            let n = b[i + 5];
            let then = b.get(i + 6).copied();
            match n {
                b'/' | b'_' => false,
                b'.' => !then.is_some_and(|t| t.is_ascii_alphanumeric()),
                _ if n.is_ascii_lowercase() => false,
                _ if n.is_ascii_uppercase() => capital,
                _ => true,
            }
        };
        if hit && before_ok && after_ok {
            let word = &s[i..i + 5];
            out.push_str(if word == "RENDA" {
                "KEEPER"
            } else if word.starts_with('R') {
                "Keeper"
            } else {
                "keeper"
            });
            i += 5;
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

fn last_note(it: &Item) -> Option<&String> {
    it.notes.last()
}

/// The item's own line of the published ledger.
fn line_url(it: &Item) -> String {
    format!("{LEDGER_URL}#L{}", it.line)
}

fn line_foot(it: &Item) -> Markup {
    foot_labelled(&line_url(it), &format!("Ledger line {} on GitHub", it.line), Target::Evidence)
}

/// The repository a bare `branch@sha` in a note belongs to when the note
/// names none: the stream's home. Streams with no one home get None and
/// such a reference stays plain text.
fn default_repo(stream: &str) -> Option<&'static str> {
    match stream {
        "Engine speed" | "Research" => Some("omega-mv"),
        "Alpha release" => Some("aien-sovereign-core-mv"),
        "AIEN Rig" => Some("aien-rig"),
        "AienOS on the iMac" => Some("aienux"),
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
                (keeper_name(&seg.text))
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
/// `aienos` and `aienos-mv` are the kernel's former names: ledger lines
/// written before the rename mean the kernel, now `aienux`.
const REPOS: [(&str, &str); 20] = [
    ("omega", "omega-mv"),
    ("omega-mv", "omega-mv"),
    ("sc", "aien-sovereign-core-mv"),
    ("aien-sovereign-core", "aien-sovereign-core-mv"),
    ("aien-sovereign-core-mv", "aien-sovereign-core-mv"),
    ("aienux", "aienux"),
    ("aienos", "aienux"),
    ("aienos-mv", "aienux"),
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
    REPOS.iter().find(|(s, _)| *s == short).map(|(_, full)| *full)
}

fn is_sha(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn is_branch(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('/')
        && !s.ends_with('/')
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '/' | '.' | '_' | '-'))
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
                return Some(Ref::Issue { repo: full, n: n.to_string() });
            }
        }
        return None;
    }
    if let Some((branch, sha)) = tok.split_once('@') {
        if is_branch(branch) && is_sha(sha) {
            return Some(Ref::Commit { branch: branch.to_string(), sha: sha.to_string() });
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
            let end = note[s..].find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '>').map(|k| s + k).unwrap_or(note.len());
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
            Some(Ref::Issue { repo, n }) => refs.push(Some((format!("https://github.com/aien-dev/{repo}/issues/{n}"), s, e))),
            Some(Ref::Commit { branch, sha }) => match last_repo {
                Some(r) => {
                    commits.push((s, r.to_string(), branch));
                    refs.push(Some((format!("https://github.com/aien-dev/{r}/commit/{sha}"), s, e)));
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
                refs[k] = Some((format!("https://github.com/aien-dev/{repo}/tree/{branch}/{path}"), s, e));
            }
        }
    }
    // Emit.
    let mut out: Vec<Seg> = Vec::new();
    let mut cursor = 0;
    for r in refs.into_iter().flatten() {
        let (href, s, e) = r;
        if s > cursor {
            out.push(Seg { text: note[cursor..s].to_string(), href: None });
        }
        out.push(Seg { text: note[s..e].to_string(), href: Some(href) });
        cursor = e;
    }
    if cursor < note.len() {
        out.push(Seg { text: note[cursor..].to_string(), href: None });
    }
    out
}

fn card(it: &Item) -> Markup {
    let w = warmth(it.age_s);
    html! {
        article class=(format!("now-card now-{w} now-k-{}", it.owner_kind)) id=(it.id) {
            header class="now-card-top" {
                span class="now-id" { (it.id) }
                (owner_chip(it))
                span class="now-age" title=(it.updated_utc) {
                    span class="now-dot" aria-hidden="true" {}
                    (age_words(it.age_s))
                }
            }
            h3 class="now-item" { (keeper_name(&it.item)) }
            @if let Some(n) = last_note(it) {
                p class="now-note" { (note_markup(n, it.stream)) }
            }
            @if !it.done_when.is_empty() {
                p class="now-done" { span class="now-done-k" { "done when" } " " (keeper_name(&it.done_when)) }
            }
            (line_foot(it))
        }
    }
}

/// A line counts as current when it was written in the last day. Current
/// lines are shown open; older ones are folded, still in the HTML.
const CURRENT_S: i64 = 24 * 3600;

fn is_current(it: &Item) -> bool {
    it.age_s >= 0 && it.age_s < CURRENT_S
}

/// One folded line: id, item, its latest note, owner, age, ledger link.
fn row(it: &Item) -> Markup {
    html! {
        li class="now-queue-row now-row" id=(it.id) {
            span class="now-id" { (it.id) }
            span class="now-queue-item" {
                (keeper_name(&it.item))
                @if let Some(n) = last_note(it) {
                    span class="now-row-note" { (note_markup(n, it.stream)) }
                }
                @if !it.done_when.is_empty() {
                    span class="now-row-done" { "done when " (keeper_name(&it.done_when)) }
                }
            }
            span class="now-row-meta" {
                (owner_chip(it))
                span class="now-row-age" title=(it.updated_utc) { (age_words(it.age_s)) }
            }
            (line_foot(it))
        }
    }
}

/// A closed `details` block. Its rows stay in the page for every reader
/// and crawler; a visitor opens it with one click, and a link to a line
/// inside opens it in the browser.
fn fold(id: &str, title: &str, items: &[&Item]) -> Markup {
    let newest = items.iter().filter(|i| i.age_s >= 0).map(|i| i.age_s).min();
    html! {
        details class="now-fold" id=(id) {
            summary class="now-fold-sum" {
                span class="now-fold-name" { (title) }
                span class="now-fold-n" { (items.len()) " " @if items.len() == 1 { "line" } @else { "lines" } }
                @if let Some(a) = newest {
                    span class="now-fold-age" { "last written " (age_words(a)) }
                }
            }
            ol class="now-queue now-fold-rows" {
                @for it in items { (row(it)) }
            }
        }
    }
}

fn slug(s: &str) -> String {
    s.to_ascii_lowercase().replace(' ', "-")
}

fn settled_entry(it: &Item) -> Markup {
    html! {
        li class=(format!("now-tl now-tl-{}", it.status)) id=(it.id) {
            span class="now-tl-when mono" title=(it.updated_utc) { (age_words(it.age_s)) }
            div class="now-tl-body" {
                span class=(format!("now-verdict now-verdict-{}", it.status)) { (it.status) }
                span class="now-id" { (it.id) }
                p class="now-tl-item" { (keeper_name(&it.item)) }
                @if let Some(n) = last_note(it) {
                    p class="now-tl-evidence" { (note_markup(n, it.stream)) }
                }
                (line_foot(it))
            }
        }
    }
}

/// The UTC day a line was last written, "2026-10-24", or "date unknown".
fn day_of(it: &Item) -> String {
    let s = if it.updated_utc.is_empty() { &it.opened_utc } else { &it.updated_utc };
    if parse_utc(s).is_some() { s[..10].to_string() } else { "date unknown".into() }
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

fn state_words(state: &str) -> &'static str {
    match state {
        "answering" => "Answering",
        "not-answering" => "Not answering",
        "planned" => "Planned",
        "not-trained" => "Not trained yet",
        _ => "Status unknown",
    }
}

/// One model card. Its box link goes to the ledger line that carries its
/// plan, else to its design source, else to the plan itself.
fn model_card(m: &Model, now: &Now) -> Markup {
    let foot = match m.plan_ledger.as_deref().and_then(|id| now.items.iter().find(|i| i.id == id)) {
        Some(it) => foot_labelled(&line_url(it), &format!("Plan: ledger {} on GitHub", it.id), Target::Evidence),
        None => match &m.source {
            Some(s) => foot_labelled(&s.url, &s.label, Target::Source),
            None => foot_labelled(&now.plan.url, &now.plan.label, Target::Source),
        },
    };
    html! {
        article class=(format!("now-card now-model now-model-{}", m.state)) id=(m.key) {
            header class="now-card-top" {
                span class="now-model-track" { (m.track) }
                span class=(format!("now-model-state now-model-state-{}", m.state)) { (state_words(&m.state)) }
            }
            h3 class="now-item" { (m.name) }
            p class="now-note now-model-role" { (m.role) }
            @match m.state.as_str() {
                "answering" => {
                    p class="now-done" { span class="now-done-k" { "serving" } " " span class="mono" { (m.served_as.as_deref().unwrap_or("")) } ", answered the Spark's check at " span class="mono" { (now.models_checked_utc) } }
                }
                "not-answering" => {
                    p class="now-done" { "Did not answer the Spark's check at " span class="mono" { (now.models_checked_utc) } "." }
                }
                "unknown" => {
                    p class="now-done" { "The Spark has not published a model check yet." }
                }
                _ => {}
            }
            @if !m.figures.is_empty() {
                dl class="now-model-figs" {
                    @for f in &m.figures {
                        dt { (f.label) }
                        dd { (f.value) " " a class="now-ref" href=(f.receipt) target="_blank" rel="noopener noreferrer" { "receipt" } }
                    }
                }
            }
            (foot)
        }
    }
}

fn models_section(now: &Now) -> Markup {
    html! {
        section class="now-section" id="models" aria-labelledby="now-models-h" {
            div class="container" {
                p class="kicker" { "On the Spark" }
                h2 id="now-models-h" class="now-h2" { "Models doing the work" }
                p class="now-sub" {
                    "The models the plan names, what each one is for, and whether it answered the Spark's own check of its model servers. The Spark checks every five minutes; this page shows the check from its last build"
                    @if now.models_checked_utc != "never" { " (" span class="mono" { (now.models_checked_utc) } ")" }
                    ". Every speed links to its receipt. A token is a word or a piece of one. The plan: "
                    a class="prose-link" href=(now.plan.url) { (now.plan.label) }
                    "."
                }
                div class="now-grid" {
                    @for m in &now.models { (model_card(m, now)) }
                }
            }
        }
    }
}

/// One limit's row: its label, its meter, what is left, and when it resets.
fn usage_row(l: &UsageLimit) -> Markup {
    html! {
        div class="now-usage-row" {
            span class="now-usage-k" { (l.label) }
            meter class="now-usage-meter" min="0" max="100" low="70" high="90" optimum="0" value=(l.used_pct) {
                (l.used_pct) "% used"
            }
            span class="now-usage-v" { strong { (100 - l.used_pct) "% left" } " " span class="now-usage-used" { "(" (l.used_pct) "% used)" } }
            @if !l.resets_utc.is_empty() {
                span class="now-usage-reset" { "resets " span class="mono" { (l.resets_utc) } }
            }
        }
    }
}

/// A reading older than this is called out as old.
const USAGE_STALE_S: i64 = 2 * 3600;

/// Drake's Claude plans: how much of each limit is left, and the two ways a
/// visitor can give compute instead of money (an agent on their own plan, or a
/// Spark qualifying the same gates).
fn usage_section(now: &Now) -> Markup {
    let built = parse_utc(&now.built_utc);
    html! {
        section class="now-section" id="claude" aria-labelledby="now-claude-h" {
            div class="container" {
                p class="kicker" { "Paying for the work" }
                h2 id="now-claude-h" class="now-h2" { "How much Claude is left" }
                p class="now-sub" {
                    "Many of the agents on this page are Claude models made by Anthropic (Opus, Fable, Sonnet and Haiku), running on Drake's own three Claude subscriptions. The plans allow a set amount of use in each five-hour session and in each week. The Spark reads the plan's own usage meter every five minutes and publishes the percentages, never the login."
                }
                p class="now-sub" {
                    "The same work also runs on subscriptions Drake pays for himself: OpenAI Codex, Google Gemini and xAI Grok. Those plans publish no usage meter here, so this page shows no percentage for them. The Claude meter is the only one the Spark can read."
                }
                div class="now-usage-wrap" {
                    article class="now-usage" {
                        @match &now.claude {
                            Some(u) => {
                                header class="now-card-top" {
                                    span class="now-usage-plan" { "Claude " (u.plan) " plan" }
                                    span class="now-age mono" { "read " (u.checked_utc) }
                                }
                                @if built.zip(parse_utc(&u.checked_utc)).is_some_and(|(b, c)| b - c > USAGE_STALE_S) {
                                    p class="now-done" { "This is the last reading the Spark published; it has not read a newer one since." }
                                }
                                @if u.limits.len() > 1 {
                                    div class="now-usage-menu" {
                                        @for (i, _) in u.limits.iter().enumerate() {
                                            input type="radio" name="now-usage" id=(format!("now-usage-{i}")) class="now-usage-radio" checked[i == 0];
                                        }
                                        div class="now-usage-tabs" role="tablist" aria-label="Usage limits: choose one to read" {
                                            @for (i, l) in u.limits.iter().enumerate() {
                                                label class="now-usage-tab" for=(format!("now-usage-{i}")) {
                                                    span class="now-usage-tab-k" { (l.label) }
                                                    span class="now-usage-tab-v" { (100 - l.used_pct) "% left" }
                                                }
                                            }
                                        }
                                        div class="now-usage-panels" {
                                            @for (i, l) in u.limits.iter().enumerate() {
                                                div class=(format!("now-usage-panel now-usage-panel-{i}")) {
                                                    (usage_row(l))
                                                }
                                            }
                                        }
                                    }
                                } @else {
                                    @for l in &u.limits { (usage_row(l)) }
                                }
                            }
                            None => {
                                p class="now-done" { "The Spark has not published a reading of the plan yet." }
                            }
                        }
                        (foot_labelled(USAGE_URL, "Usage reading on GitHub", Target::Evidence))
                    }
                    aside class="now-give" aria-labelledby="now-give-h" {
                        h3 id="now-give-h" class="now-give-h" { "Give compute, not money" }
                        p {
                            "The work runs on plans and on one Spark. Both can come from you, and Drake takes no payment for either."
                        }
                        ul class="now-give-steps" {
                            li {
                                strong { "Your agent, your plan. " }
                                "Pick any repository on " a class="prose-link" href="/agents/" { "Agents" } " and give your agent its instructions: the rules ours follow and that repository's own AGENTS.md. It claims an open issue and opens a pull request that leads with its evidence, like ours do. Its own subscription pays for it (Claude, Codex, Gemini or Grok); nothing of yours is sent to the Spark."
                            }
                            li {
                                strong { "Your Spark. " }
                                "A second GB10 can qualify the same gates and publish its own receipt. The GPU gates are written to run on any Spark, and a PASS from your machine is evidence, not a claim."
                            }
                        }
                        p class="now-give-note" { "No keys are collected. Your agent runs on your account, your Spark on your network, and only merged evidence reaches this page." }
                        a class="now-give-btn" href=(HUMANITY_URL) target="_blank" rel="noopener noreferrer" { "Join in Open Humanity " span aria-hidden="true" { "\u{2197}" } }
                    }
                }
            }
        }
    }
}

/// Keeper's name tag carries a small dachshund, the picture in "How this page is made".
fn owner_chip(it: &Item) -> Markup {
    let label = owner_label(it);
    html! {
        span class=(format!("now-owner now-owner-{}", it.owner_kind)) {
            @if label.starts_with("Keeper") {
                img class="now-owner-dog" src="/assets/keeper.svg" alt="" width="42" height="14";
            }
            (label)
        }
    }
}

pub fn render_now_page(now: &Now) -> Markup {
    let open: Vec<&Item> = now.items.iter().filter(|i| i.status == "open").collect();
    let settled: Vec<&Item> = now.items.iter().filter(|i| i.status == "done" || i.status == "dropped").collect();
    let hands: usize = open.iter().filter(|i| i.owner_kind == "agent" && i.age_s >= 0 && i.age_s < 2 * 3600).map(|i| i.owner.as_str()).collect::<std::collections::BTreeSet<_>>().len();
    let for_drake: Vec<&Item> = open.iter().copied().filter(|i| i.owner_kind == "operator").collect();
    let next: Vec<&Item> = open.iter().copied().filter(|i| i.owner_kind != "operator" && (i.owner_kind == "unassigned" || i.notes.is_empty())).collect();
    let moving: Vec<&Item> = open.iter().copied().filter(|i| i.owner_kind != "operator" && !(i.owner_kind == "unassigned" || i.notes.is_empty())).collect();
    let mut settled_sorted = settled.clone();
    settled_sorted.sort_by_key(|i| if i.age_s < 0 { i64::MAX } else { i.age_s });
    let (moving_now, moving_quiet): (Vec<&Item>, Vec<&Item>) = moving.iter().copied().partition(|i| is_current(i));
    let (next_now, next_quiet): (Vec<&Item>, Vec<&Item>) = next.iter().copied().partition(|i| is_current(i));
    let (settled_now, settled_old): (Vec<&Item>, Vec<&Item>) = settled_sorted.iter().copied().partition(|i| is_current(i));
    // Older settled lines, one fold per UTC day, newest day first.
    let mut settled_days: Vec<(String, Vec<&Item>)> = Vec::new();
    for it in &settled_old {
        let d = day_of(it);
        match settled_days.last_mut() {
            Some((day, list)) if *day == d => list.push(it),
            _ => settled_days.push((d, vec![it])),
        }
    }

    html! {
        section class="now-hero" aria-labelledby="now-title" {
            div class="container" {
                div class="now-hero-top" {
                    div class="now-hero-text" {
                        p class="kicker" { "Now" }
                        h1 id="now-title" class="now-title" { "What is being done, right now." }
                        p class="now-lead" {
                        "This is the working ledger of AIEN, published as it is written. Every line is a promise with an owner and an observable finish. Agents write their progress here; the operator writes decisions here. Nothing on this page is composed by hand. Lines written in the last day are shown open; older lines are folded below, each one click away."
                        }
                        p class="now-lead" {
                            a href="/hive/" { strong { "Hive" } }
                            " / Watch the agents work, see their exact models, and follow Drake's instructions and corrections live. "
                            a href="/learn/" { "Learn to structure your own work with AI." }
                        }
                    }
                    figure class="now-keeper" {
                        img src="/assets/keeper.svg" width="600" height="200" alt="Keeper, the merge desk that lands finished changes: a big cartoon dachshund, long and low, wearing a gold key on its red collar.";
                        figcaption { "Keeper, the merge desk" }
                    }
                }
                @if now.items.is_empty() {
                    p class="now-empty" { "No ledger has been published yet. The page will fill itself the moment one arrives." }
                } @else {
                    div class="now-stats" {
                        (stat(hands, "agents writing in the last two hours", "now-stat-hot"))
                        (stat(moving_now.len(), "lines in motion, written in the last day", ""))
                        (stat(moving_quiet.len() + next_quiet.len(), "open lines quiet for over a day, quiet and queued", ""))
                        (stat(settled_now.len(), "settled in the last day", "now-stat-done"))
                        (stat(for_drake.len(), "waiting on the operator", "now-stat-op"))
                    }
                    nav class="now-jump" aria-label="On this page" {
                        a href="#claude" { "Paying for the work" }
                        @if !for_drake.is_empty() { a href="#operator" { "Operator " span { (for_drake.len()) } } }
                        a href="#streams" { "In motion " span { (moving_now.len()) } }
                        a href="#models" { "Models " span { (now.models.len()) } }
                        @if !moving_quiet.is_empty() { a href="#quiet" { "Quiet " span { (moving_quiet.len()) } } }
                        @if !next.is_empty() { a href="#next" { "Queued " span { (next.len()) } } }
                        @if !settled_sorted.is_empty() { a href="#settled" { "Settled " span { (settled_sorted.len()) } } }
                        a href="#how" { "How it is made" }
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

        (usage_section(now))

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
                p class="now-sub" { "Lines written in the last day, grouped by stream. The bar under each name is the stream's pulse: one segment per line, bright when written in the last half hour, fading as it cools." }
                @if moving_now.is_empty() {
                    p class="now-empty" { "No line in motion was written in the last day." }
                }
                @for s in STREAMS {
                    @let in_stream: Vec<&Item> = moving_now.iter().copied().filter(|i| i.stream == s).collect();
                    @if !in_stream.is_empty() {
                        div class="now-stream" id=(format!("stream-{}", slug(s))) {
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

        (models_section(now))

        @if !moving_quiet.is_empty() {
            section class="now-section" id="quiet" aria-labelledby="now-quiet-h" {
                div class="container" {
                    p class="kicker" { "Quiet" }
                    h2 id="now-quiet-h" class="now-h2" { "Open, not written in the last day" }
                    p class="now-sub" { "Still open, with an owner and notes, but nobody has written to them for over a day. Folded by stream; open a stream to read every line and its latest note." }
                    div class="now-folds" {
                        @for s in STREAMS {
                            @let in_stream: Vec<&Item> = moving_quiet.iter().copied().filter(|i| i.stream == s).collect();
                            @if !in_stream.is_empty() {
                                (fold(&format!("quiet-{}", slug(s)), s, &in_stream))
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
                    @if !next_now.is_empty() {
                        ol class="now-queue" {
                            @for it in &next_now { (row(it)) }
                        }
                    }
                    @if !next_quiet.is_empty() {
                        div class="now-folds" {
                            (fold("queued-older", "Queued, not written in the last day", &next_quiet))
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
                    p class="now-sub" { "A line closes on evidence (a commit, a receipt, a file) or is dropped for a stated reason. The last day is shown open, newest first; earlier days are folded by date (UTC)." }
                    @if settled_now.is_empty() {
                        p class="now-empty" { "Nothing settled in the last day." }
                    } @else {
                        ol class="now-timeline" {
                            @for it in &settled_now { (settled_entry(it)) }
                        }
                    }
                    div class="now-folds" {
                        @for (day, list) in &settled_days {
                            details class="now-fold" id=(format!("settled-{day}")) {
                                summary class="now-fold-sum" {
                                    span class="now-fold-name" { "Settled " (day) }
                                    span class="now-fold-n" { (list.len()) " " @if list.len() == 1 { "line" } @else { "lines" } }
                                }
                                ol class="now-timeline now-fold-rows" {
                                    @for it in list { (settled_entry(it)) }
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
                    ". A line that is not in the file is not here. Every card, queue row and settled entry links to its own line of that file, and the commits, pull requests and evidence folders named in a note link to GitHub. Owners shown as \"agent\" are autonomous coding agents; the operator is one person. Keeper, the merge desk that lands finished changes, was called Renda until 24 October 2026: this page shows the new name, and the ledger on GitHub keeps each line as it was written. Keeper is the dachshund at the top of this page; a small one marks every line Keeper owns."
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page with no ledger and no model check; tests override what they need.
    fn blank() -> Now {
        let (plan, models) = models_from(ROSTER, &None, &[]);
        Now { built_utc: String::new(), ledger_written_utc: String::new(), items: vec![], pulse: vec![], models_checked_utc: "never".into(), plan, models, claude: None }
    }

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
        assert!(!items[2].notes[0].contains('\u{2014}'), "dash must be cleaned");
    }

    #[test]
    fn time_round_trips() {
        for s in ["2026-10-19T16:43Z", "2000-02-29T00:00Z", "2026-01-01T23:59Z"] {
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
            pulse: vec![Pulse { key: "memory free".into(), value: "50 GiB".into() }],
            ..blank()
        };
        let page = render_now_page(&now).into_string();
        for must in ["id=\"operator\"", "id=\"streams\"", "id=\"next\"", "id=\"settled\"", "Engine speed", "15.63 tok/s", "now-verdict-done", "L004"] {
            assert!(page.contains(must), "missing {must}");
        }
        assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'));
        // The Renda workers were retired; their dials must not come back.
        assert!(!page.contains("renda") && !page.contains("<script"), "no Renda dials and no script on /now/");
        // Every card, queue row and timeline entry links to its own ledger line.
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(4 + 3 + 1), "four ledger boxes, three model cards and the Claude plan card");
        for line in 2..=5 {
            assert!(page.contains(&format!("href=\"{LEDGER_URL}#L{line}\"")), "line {line} link missing");
        }
        assert!(page.contains("Ledger line 2 on GitHub"));
        // The pull request named in L003's note became a link in the timeline.
        assert!(page.contains("class=\"now-ref\" href=\"https://github.com/aien-dev/aien-sovereign-core-mv/issues/8\""), "note reference not linked");
        let json = to_json(&now);
        assert!(json.contains("\"L001\"") && json.contains("\"stream\": \"Engine speed\""));
    }

    #[test]
    fn old_lines_fold_but_stay_in_the_page() {
        // Three days after SAMPLE was written every line is old.
        let at = parse_utc("2026-10-22T18:10Z").unwrap();
        let now = Now { built_utc: fmt_utc(at), ledger_written_utc: "2026-10-19T18:00Z".into(), items: parse_ledger(SAMPLE, at), pulse: vec![], ..blank() };
        let page = render_now_page(&now).into_string();
        assert!(page.contains("No line in motion was written in the last day."));
        assert!(page.contains("Nothing settled in the last day."));
        let quiet = page.find("<details class=\"now-fold\" id=\"quiet-engine-speed\">").expect("quiet fold missing");
        let queued = page.find("<details class=\"now-fold\" id=\"queued-older\">").expect("queued fold missing");
        let settled = page.find("<details class=\"now-fold\" id=\"settled-2026-10-19\">").expect("settled day fold missing");
        // Each old line sits inside its fold, keeps its anchor, note and ledger link.
        assert!(page[quiet..].find("id=\"L001\"").unwrap() < page[quiet..].find("</details>").unwrap());
        assert!(page[queued..].find("id=\"L004\"").unwrap() < page[queued..].find("</details>").unwrap());
        assert!(page[settled..].find("id=\"L003\"").unwrap() < page[settled..].find("</details>").unwrap());
        assert!(page.contains("15.63 tok/s") && page.contains("done when PR merged"));
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(4 + 3 + 1), "four ledger boxes, three model cards and the Claude plan card");
        // Operator lines never fold.
        let op = page.find("id=\"operator\"").unwrap();
        assert!(page[op..].find("id=\"L002\"").unwrap() < page[op..].find("id=\"streams\"").unwrap());
        assert!(page.contains("open lines quiet for over a day"));
        assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'));
    }

    #[test]
    fn models_follow_the_check() {
        // No check published: the 35B reads unknown, never answering or down.
        let (_, m) = models_from(ROSTER, &None, &[]);
        let by = |m: &[Model], k: &str| m.iter().find(|x| x.key == k).unwrap().state.clone();
        assert_eq!(by(&m, "model-35b"), "unknown");
        assert_eq!(by(&m, "model-27b"), "planned");
        assert_eq!(by(&m, "model-own"), "not-trained");
        // The check names the 35B: answering. The 27B stays planned until it answers.
        let (checked, ids) = parse_models_check("checked\t2026-10-24T20:00:01Z\nmodel\tqwen3.6-35b-a3b-fp8\nmodel\tTandemLLM\nbogus line\n");
        assert_eq!(checked.as_deref(), Some("2026-10-24T20:00:01Z"));
        let (_, m) = models_from(ROSTER, &checked, &ids);
        assert_eq!(by(&m, "model-35b"), "answering");
        assert_eq!(by(&m, "model-27b"), "planned");
        let (_, m) = models_from(ROSTER, &checked, &["Qwen3.8-27B".to_string()]);
        assert_eq!(by(&m, "model-35b"), "not-answering");
        assert_eq!(by(&m, "model-27b"), "answering");
        // On the page: every card has its GitHub link, ids outside the roster never show.
        let now = Now { models_checked_utc: checked.unwrap(), models: models_from(ROSTER, &Some("x".into()), &ids).1, ..blank() };
        let page = render_now_page(&now).into_string();
        assert!(page.contains("id=\"models\"") && page.contains("Models doing the work") && page.contains("qwen3.6-35b-a3b-fp8"));
        assert!(!page.contains("TandemLLM"), "only roster models are shown");
        assert!(page.contains("Q36_CEILING.md"), "35B figures link their receipt");
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(3 + 1));
        assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'));
    }

    #[test]
    fn keeper_replaces_renda_in_prose_only() {
        for (from, to) in [
            ("Renda reviewing on the motor", "Keeper reviewing on the motor"),
            ("renda1 pass done; RENDA paused", "keeper1 pass done; KEEPER paused"),
            ("publication remains Renda-owned.", "publication remains Keeper-owned."),
            ("(Renda, desk)", "(Keeper, desk)"),
            ("throughsoleRenda, installedRenda11:13, RendaL708", "throughsoleKeeper, installedKeeper11:13, KeeperL708"),
            ("handed to Renda. Then renda.json stays", "handed to Keeper. Then renda.json stays"),
            // Paths, file names and identifiers keep the old name so they still resolve.
            ("~/.local/state/aien-local-ci/renda-queue/acks", "~/.local/state/aien-local-ci/renda-queue/acks"),
            ("evidence/renda-pr-fastpath-L593 and renda_metrics.py", "evidence/renda-pr-fastpath-L593 and renda_metrics.py"),
            ("agent now-renda-dashboard, Rendan, rendas", "agent now-renda-dashboard, Rendan, rendas"),
        ] {
            assert_eq!(keeper_name(from), to);
        }
        let tsv = "id\topened_utc\towner\tstatus\titem\tdone_when\tevidence\tupdated_utc\nL009\t2026-10-19T16:43Z\trenda2\topen\tRenda merges the queue\tRenda idle\tsee evidence/renda-x/ | Renda done\t2026-10-19T17:00Z\n";
        let at = parse_utc("2026-10-19T18:10Z").unwrap();
        let now = Now { items: parse_ledger(tsv, at), ..blank() };
        let page = render_now_page(&now).into_string();
        let owned = parse_ledger(&tsv.replace("\trenda2\t", "\tkeeper\t"), at);
        assert_eq!(owner_label(&owned[0]), "Keeper");
        assert!(page.contains("Keeper merges the queue") && page.contains("Keeper done") && page.contains(">Keeper 2<"), "prose and owner renamed");
        assert!(!page.contains("Renda merges"), "old name left in prose");
        // The machine record keeps the ledger as written.
        assert!(to_json(&now).contains("Renda merges the queue"));
    }

    #[test]
    fn empty_ledger_is_honest() {
        let now = Now { built_utc: "2026-10-19T18:10Z".into(), ledger_written_utc: "never".into(), items: vec![], pulse: vec![], ..blank() };
        let page = render_now_page(&now).into_string();
        assert!(page.contains("No ledger has been published yet"));
    }

    #[test]
    fn claude_plan_shows_what_is_left_and_how_to_give() {
        let tsv = "checked\t2026-10-24T22:40:00Z\nplan\tMax 20x\n\
limit\tsession\t39\t2026-10-25T02:09:59Z\t\n\
limit\tweekly_all\t23.4\t2026-10-31T15:59:59Z\t\n\
limit\tweekly_scoped\t34\t2026-10-31T15:59:59Z\tFable<script>\n\
limit\tsomething_new\t5\t2026-10-31T15:59:59Z\t\n";
        let u = parse_usage(tsv).expect("a reading");
        assert_eq!(u.checked_utc, "2026-10-24T22:40Z");
        assert_eq!(u.plan, "Max 20x");
        let labels: Vec<&str> = u.limits.iter().map(|l| l.label.as_str()).collect();
        assert_eq!(labels, ["This five-hour session", "This week, all models", "This week, Fablescript only"], "unknown kinds are left out");
        assert_eq!(u.limits[1].used_pct, 23);
        assert!(parse_usage("plan\tMax\nlimit\tsession\t1\tx\t\n").is_none(), "no time, no reading");

        let now = Now { built_utc: "2026-10-24T22:45Z".into(), claude: Some(u.clone()), ..blank() };
        let page = render_now_page(&now).into_string();
        assert!(page.contains("61% left") && page.contains("77% left") && page.contains("66% left"));
        // The limits are one menu: a radio and a chip per window, one panel open.
        assert_eq!(page.matches("class=\"now-usage-radio\"").count(), 3, "one radio per limit");
        assert_eq!(page.matches("class=\"now-usage-tab\"").count(), 3, "one chip per limit");
        assert_eq!(page.matches("now-usage-panel-").count(), 3, "one panel per limit");
        assert!(page.contains("id=\"now-usage-0\"") && page.contains("for=\"now-usage-2\""));
        assert!(page.contains("checked") && !page.contains("checked id=\"now-usage-1\""), "the first limit opens");
        assert!(page.contains("61% left") && page.contains("77% left"), "each chip carries its own figure");
        assert!(page.contains(HUMANITY_URL), "the way in is Open Humanity");
        assert!(!page.contains("X Money") && !page.contains("Send Money"), "no money ask is rendered");
        assert!(!page.to_ascii_lowercase().contains("aienscore"), "retired X handle must not be rendered");
        assert!(!page.contains("has not read a newer one"), "a fresh reading is not called old");
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(3 + 1));

        let old = Now { built_utc: "2026-10-25T03:00Z".into(), claude: Some(u), ..blank() };
        assert!(render_now_page(&old).into_string().contains("has not read a newer one"));
        let none = render_now_page(&blank()).into_string();
        assert!(none.contains("has not published a reading of the plan yet") && none.contains(HUMANITY_URL));
    }

    #[test]
    fn keeper_wears_the_dachshund() {
        let tsv = "id\topened_utc\towner\tstatus\titem\tdone_when\tevidence\tupdated_utc\nL031\t2026-10-19T16:43Z\tkeeper\topen\tLand the queue\tqueue empty\tstarted\t2026-10-19T17:00Z\nL032\t2026-10-19T16:43Z\tsa-0-abc123def\topen\tOther work\tdone\tstarted\t2026-10-19T17:00Z\n";
        let now = Now { items: parse_ledger(tsv, parse_utc("2026-10-19T18:10Z").unwrap()), ..blank() };
        let page = render_now_page(&now).into_string();
        assert_eq!(page.matches("class=\"now-owner-dog\"").count(), 1, "only Keeper's tag carries the dog");
        assert_eq!(page.matches("src=\"/assets/keeper.svg\"").count(), 2, "the big dog in the hero and the one tag");
        assert!(page.contains("a big cartoon dachshund") && !page.contains("two cartoon dachshunds"), "one big dog, no old card");
        assert!(page.contains("href=\"#claude\""), "the jump bar reaches the Claude plan");
        assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../public/assets/keeper.svg").exists());
    }

    fn hrefs(segs: &[Seg]) -> Vec<(String, String)> {
        segs.iter().filter_map(|s| s.href.as_ref().map(|h| (s.text.clone(), h.clone()))).collect()
    }

    fn joined(segs: &[Seg]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn linkify_issues_and_pull_requests() {
        let segs = linkify("Research: omega#583 simulator; aien-dev/aienos.com#44 merged, arch#276.", Some("omega-mv"));
        assert_eq!(
            hrefs(&segs),
            vec![
                ("omega#583".into(), "https://github.com/aien-dev/omega-mv/issues/583".into()),
                ("aien-dev/aienos.com#44".into(), "https://github.com/aien-dev/aienos.com/issues/44".into()),
                ("arch#276".into(), "https://github.com/aien-dev/aien-architecture-mv/issues/276".into()),
            ]
        );
        assert_eq!(joined(&segs), "Research: omega#583 simulator; aien-dev/aienos.com#44 merged, arch#276.");
    }

    #[test]
    fn linkify_commit_uses_the_repo_named_earlier_in_the_note() {
        let segs = linkify("sc alpha/ceiling-and-model@56d42869 commit 39e79fad docs (omega rig/speech@3aebf9ee),", None);
        assert_eq!(
            hrefs(&segs),
            vec![
                ("alpha/ceiling-and-model@56d42869".into(), "https://github.com/aien-dev/aien-sovereign-core-mv/commit/56d42869".into()),
                ("rig/speech@3aebf9ee".into(), "https://github.com/aien-dev/omega-mv/commit/3aebf9ee".into()),
            ]
        );
        assert_eq!(segs.last().unwrap().text, "),", "trailing punctuation stays outside the link");
    }

    #[test]
    fn linkify_commit_falls_back_to_the_stream_repo_or_stays_text() {
        let segs = linkify("feat/goodbye-panel-off@2954f5e pushed", Some("aienux"));
        assert_eq!(hrefs(&segs), vec![("feat/goodbye-panel-off@2954f5e".into(), "https://github.com/aien-dev/aienux/commit/2954f5e".into())]);
        let segs = linkify("feat/goodbye-panel-off@2954f5e pushed", None);
        assert!(hrefs(&segs).is_empty(), "no repository to resolve against: stays text");
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
        let segs = linkify("evidence/symbolic-rewrite/VERDICT.md written", Some("omega-mv"));
        assert!(hrefs(&segs).is_empty());
        // The first branch wins over one in parentheses: the Rig's own branch, not Omega's.
        let segs = linkify("aien-rig speech/v0@af71f46 (omega rig/speech@3aebf9ee); evidence/speech/2026-10-19-gb10/README.md", Some("aien-rig"));
        assert_eq!(hrefs(&segs)[2].1, "https://github.com/aien-dev/aien-rig/tree/speech/v0/evidence/speech/2026-10-19-gb10/README.md");
    }

    #[test]
    fn linkify_bare_urls() {
        let segs = linkify("live: https://www.aienos.com/now/ and /now.json", None);
        assert_eq!(hrefs(&segs), vec![("https://www.aienos.com/now/".into(), "https://www.aienos.com/now/".into())]);
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
            assert!(hrefs(&segs).is_empty(), "{plain:?} must stay plain: {:?}", hrefs(&segs));
            assert_eq!(joined(&segs), plain);
        }
    }

    #[test]
    fn notes_render_links_with_escaping_intact() {
        let m = note_markup("omega#583 <b>x</b> & done", "Research").into_string();
        assert!(m.contains("<a class=\"now-ref\" href=\"https://github.com/aien-dev/omega-mv/issues/583\""));
        assert!(m.contains("&lt;b&gt;x&lt;/b&gt; &amp; done"), "{m}");
    }
}
