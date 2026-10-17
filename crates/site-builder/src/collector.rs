//! Project progress for /progress/. `progress` reads the retained public history of every
//! public project repository and writes `public/data/project-progress.json` (schemaVersion 2),
//! the file `public/js/project-progress.js` draws.
//!
//! Every figure comes from a public record: Git objects in mirror clones of public repositories,
//! and the public pull request and Actions run listings read through `gh api`. Nothing here
//! estimates or fills in a number. A snapshot that does not reconcile is refused before it is
//! written, so a failed refresh leaves the last good file in place.
//!
//! Port of the former `scripts/collect-progress.mjs`; the rules below are the same rules.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

pub const OWNER: &str = "aien-dev";
/// Every named project repository, in one place: the project name the page shows (and the
/// Core and Sites groups match on), then where GitHub keeps it now (owner, repository name).
/// Since 2026-10-16 the AIEN repositories live in aien-dev; twelve carry a temporary `-mv`
/// suffix while GitHub's retired-name block holds. When a suffix goes away or a repository
/// moves, that is a one-line edit here. Other public repositories in OWNER that match the name
/// rule are read as well, under their GitHub name.
pub const REPOS: &[(&str, &str, &str)] = &[
    ("aien-sovereign-core", "aien-dev", "aien-sovereign-core-mv"),
    ("aienos", "aien-dev", "aienos-mv"),
    ("omega", "aien-dev", "omega-mv"),
    ("physics", "aien-dev", "physics-mv"),
    ("atlas", "aien-dev", "atlas"),
    ("aien-protocols", "aien-dev", "aien-protocols-mv"),
    ("aien-architecture", "aien-dev", "aien-architecture-mv"),
    ("benchmarks", "aien-dev", "benchmarks-mv"),
    ("aegis-runtime", "aien-dev", "aegis-runtime-mv"),
    ("spark-rsi", "aien-dev", "spark-rsi-mv"),
    ("open-humanity", "aien-dev", "open-humanity"),
    ("aien-edge", "aien-dev", "aien-edge"),
    ("aienos.com", "aien-dev", "aienos.com"),
    (".github", "aien-dev", ".github"),
    ("drakestapleton.com", "aien-spark", "drakestapleton.com"),
    ("aien-dev", "aien-spark", "aien-dev"),
    ("crumb-spec", "aien-dev", "crumb-spec-mv"),
    ("spark-crumbs", "aien-dev", "spark-crumbs-mv"),
    ("spark-hive", "aien-dev", "spark-hive"),
    ("aien-harness", "aien-dev", "aien-harness"),
    ("spark-dream", "aien-dev", "spark-dream"),
    ("cortex-rs", "aien-dev", "cortex-rs"),
    ("spark-supervisor", "aien-dev", "spark-supervisor"),
    ("spark-debugger", "aien-dev", "spark-debugger"),
    ("rad-id-sync", "aien-dev", "rad-id-sync"),
    ("spark-adapters", "aien-dev", "spark-adapters"),
    ("harvester", "aien-dev", "harvester"),
    ("spark-inquisitor", "aien-dev", "spark-inquisitor"),
    ("aien-local-stack", "aien-dev", "aien-local-stack"),
];
pub const TIMEZONE: &str = "America/Chicago";

const EXTENSIONS: &[(&str, &str)] = &[
    (".rs", "Rust"), (".c", "C"), (".h", "C"), (".inc", "C"), (".mojo", "Mojo"), (".py", "Python"),
    (".sh", "Shell"), (".bash", "Shell"), (".js", "Web"), (".mjs", "Web"), (".cjs", "Web"), (".jsx", "Web"),
    (".ts", "Web"), (".tsx", "Web"), (".html", "Web"), (".css", "Web"), (".S", "Assembly"), (".s", "Assembly"),
    (".asm", "Assembly"), (".cu", "CUDA"), (".cuh", "CUDA"), (".cc", "C++"), (".cpp", "C++"), (".hpp", "C++"),
    (".zig", "Zig"),
];
/// A path with any of these directory names (any case) is not measured.
const EXCLUDED_DIRS: &[&str] = &[
    "vendor", "vendored", "third_party", "third-party", "node_modules", "target", "dist", "build", "generated",
    "evidence", "receipts", "artifacts", "data", "results", "fixtures",
];
const CORE_NAMES: &[&str] = &["aien-sovereign-core", "aienos", "omega", "physics", "forge", "atlas", "aien-protocols", "aien-architecture", "benchmarks"];
const SITE_NAMES: &[&str] = &["aienos.com", "drakestapleton.com", ".github", "aien-dev"];

/// The accounts whose public repository listings are read: OWNER, then every owner in REPOS.
fn owners() -> Vec<&'static str> {
    let mut list = vec![OWNER];
    for (_, o, _) in REPOS {
        if !list.contains(o) {
            list.push(o);
        }
    }
    list
}

/// A repository the collector reads.
#[derive(Debug)]
struct Target {
    /// The project name the page shows.
    name: String,
    /// Where GitHub keeps it now.
    owner: String,
    github_name: String,
    /// GitHub's own address for it, from the listing.
    url: String,
    info: Value,
}

/// Pick the repositories to read from GitHub's public listings. A repository in REPOS is
/// matched by its current owner and GitHub name and read under its project name. Any other
/// public, non-fork repository of OWNER that matches the name rule is read under its GitHub
/// name, unless a REPOS entry was found under that project name. Returns the targets in listing
/// order and the REPOS entries that GitHub did not list.
fn select_repos(listing: &[Value]) -> (Vec<Target>, Vec<String>) {
    let public = |r: &&Value| {
        let flag = |k: &str| r.get(k).and_then(Value::as_bool).unwrap_or(false);
        !flag("private") && !flag("fork")
    };
    let identity = |r: &Value| -> Option<(String, String)> {
        Some((r.pointer("/owner/login")?.as_str()?.to_string(), r.get("name")?.as_str()?.to_string()))
    };
    let mapped = |owner: &str, name: &str| REPOS.iter().find(|(_, o, g)| *o == owner && *g == name);
    let found: HashSet<&str> = listing
        .iter()
        .filter(public)
        .filter_map(identity)
        .filter_map(|(owner, name)| mapped(&owner, &name).map(|(p, _, _)| *p))
        .collect();
    let mut targets = Vec::new();
    for r in listing.iter().filter(public) {
        let Some((owner, github_name)) = identity(r) else { continue };
        let name = match mapped(&owner, &github_name) {
            Some((project, _, _)) => project.to_string(),
            None if owner == OWNER && project_named(&github_name) && !found.contains(github_name.as_str()) => github_name.clone(),
            None => continue,
        };
        let url = s(r, "html_url").unwrap_or_else(|| format!("https://github.com/{owner}/{github_name}"));
        targets.push(Target { name, owner, github_name, url, info: r.clone() });
    }
    let missing = REPOS.iter().filter(|(p, _, _)| !found.contains(p)).map(|(p, o, g)| format!("{o}/{g} ({p})")).collect();
    (targets, missing)
}

/// The group the page files a repository under, by project name.
fn group_of(name: &str, archived: bool) -> &'static str {
    if archived {
        "Archived"
    } else if CORE_NAMES.contains(&name) {
        "Core"
    } else if SITE_NAMES.contains(&name) {
        "Sites"
    } else {
        "Experimental"
    }
}

/// The name rule for automatically discovered project repositories.
fn project_named(name: &str) -> bool {
    let word = |w: &str| name == w || name.starts_with(&format!("{w}-"));
    name.starts_with("aien-")
        || name.starts_with("aienos")
        || ["aegis-", "spark-", "cortex-", "crumb-"].iter().any(|p| name.starts_with(p))
        || ["omega", "forge", "physics", "atlas"].iter().any(|w| word(w))
}

// ---------------------------------------------------------------------------------------------
// Time. No time crate: civil dates from day numbers, and the US daylight saving rules for
// America/Chicago written out (UTC-6 standard, UTC-5 daylight).
// ---------------------------------------------------------------------------------------------

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    days_from_civil(if m == 12 { y + 1 } else { y }, if m == 12 { 1 } else { m + 1 }, 1) - days_from_civil(y, m, 1)
}

/// 0 = Sunday. 1970-01-01 was a Thursday.
fn weekday(days: i64) -> i64 {
    (days + 4).rem_euclid(7)
}

fn nth_sunday(y: i64, m: i64, n: i64) -> i64 {
    let first = days_from_civil(y, m, 1);
    first + (7 - weekday(first)) % 7 + 7 * (n - 1)
}

fn last_sunday(y: i64, m: i64) -> i64 {
    let last = days_from_civil(y, m, 1) + days_in_month(y, m) - 1;
    last - weekday(last)
}

/// UTC offset of America/Chicago, in seconds, at a UTC instant. Daylight time starts at
/// 02:00 standard (08:00 UTC) and ends at 02:00 daylight (07:00 UTC).
/// 2007 onward: second Sunday of March to first Sunday of November (Energy Policy Act 2005).
/// 1987 to 2006: first Sunday of April to last Sunday of October.
/// 1967 to 1986: last Sunday of April to last Sunday of October (the 1974 and 1975 emergency
/// dates are not modelled). Before 1967: standard time only. Measured history starts in 2026.
pub fn chicago_offset(unix_secs: i64) -> i64 {
    let (y, _, _) = civil_from_days(unix_secs.div_euclid(86_400));
    let (start, end) = if y >= 2007 {
        (nth_sunday(y, 3, 2), nth_sunday(y, 11, 1))
    } else if y >= 1987 {
        (nth_sunday(y, 4, 1), last_sunday(y, 10))
    } else if y >= 1967 {
        (last_sunday(y, 4), last_sunday(y, 10))
    } else {
        return -6 * 3600;
    };
    let (start, end) = (start * 86_400 + 8 * 3600, end * 86_400 + 7 * 3600);
    if unix_secs >= start && unix_secs < end { -5 * 3600 } else { -6 * 3600 }
}

fn ymd(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The America/Chicago calendar day of a UTC instant given in milliseconds.
pub fn day_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    ymd((secs + chicago_offset(secs)).div_euclid(86_400))
}

/// The America/Chicago calendar day of an RFC 3339 timestamp.
pub fn day(timestamp: &str) -> Result<String, String> {
    parse_time_ms(timestamp).map(day_ms).ok_or_else(|| format!("Invalid timestamp: {timestamp}"))
}

/// Milliseconds since the epoch for `YYYY-MM-DD` (midnight UTC) or
/// `YYYY-MM-DDTHH:MM:SS[.fraction](Z|+HH:MM|-HH:MM)`. A time without a zone is refused.
pub fn parse_time_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let num = |from: usize, to: usize| -> Option<i64> {
        let part = b.get(from..to)?;
        if part.iter().all(u8::is_ascii_digit) { std::str::from_utf8(part).ok()?.parse().ok() } else { None }
    };
    let (y, m, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    let date_ms = days_from_civil(y, m, d) * 86_400_000;
    if b.len() == 10 {
        return Some(date_ms);
    }
    if b.get(10) != Some(&b'T') || b.get(13) != Some(&b':') || b.get(16) != Some(&b':') {
        return None;
    }
    let (hh, mm, ss) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if hh > 23 || mm > 59 || ss > 59 {
        return None;
    }
    let mut rest = &s[19..];
    let mut millis = 0;
    if let Some(frac) = rest.strip_prefix('.') {
        let digits = frac.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let first3: String = frac[..digits].chars().chain("00".chars()).take(3).collect();
        millis = first3.parse::<i64>().ok()?;
        rest = &frac[digits..];
    }
    let offset_secs = match rest {
        "Z" | "z" => 0,
        _ => {
            let rb = rest.as_bytes();
            if rb.len() != 6 || rb[3] != b':' || !(rb[0] == b'+' || rb[0] == b'-') {
                return None;
            }
            let oh: i64 = rest[1..3].parse().ok()?;
            let om: i64 = rest[4..6].parse().ok()?;
            if !rest[1..3].bytes().chain(rest[4..6].bytes()).all(|c| c.is_ascii_digit()) || oh > 23 || om > 59 {
                return None;
            }
            (oh * 3600 + om * 60) * if rb[0] == b'-' { -1 } else { 1 }
        }
    };
    Some(date_ms + ((hh * 3600 + mm * 60 + ss) - offset_secs) * 1000 + millis)
}

/// `2026-10-16T17:04:05.123Z`, the shape JavaScript's `toISOString` writes.
pub fn iso_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let rem = secs.rem_euclid(86_400);
    format!(
        "{}T{:02}:{:02}:{:02}.{:03}Z",
        ymd(secs.div_euclid(86_400)),
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        ms.rem_euclid(1000)
    )
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// Measurement boundaries.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Source,
    Test,
    Docs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub kind: Kind,
    pub lang: &'static str,
}

/// Node's `path.posix.extname`: from the last dot of the last component, unless that dot opens
/// the name (`.bashrc`) or the name is `..`.
fn extname(file: &str) -> &str {
    let base = file.rsplit('/').next().unwrap_or(file);
    match base.rfind('.') {
        Some(0) | None => "",
        Some(_) if base == ".." => "",
        Some(i) => &base[i..],
    }
}

fn excluded(file: &str) -> bool {
    file.split('/').any(|part| EXCLUDED_DIRS.iter().any(|d| part.eq_ignore_ascii_case(d)))
}

/// Test footprint by file-path naming: a `test`/`tests` directory or name, `test_*`, `*_test(s)`,
/// or `*.test.*` / `*.spec.*` (any case).
fn test_path(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    if lower.contains(".test.") || lower.contains(".spec.") {
        return true;
    }
    lower.split('/').any(|part| {
        part.char_indices()
            .filter(|(_, c)| *c == '.')
            .map(|(i, _)| i)
            .chain(std::iter::once(part.len()))
            .any(|cut| {
                let p = &part[..cut];
                p == "test" || p == "tests" || p.starts_with("test_") || p.ends_with("_test") || p.ends_with("_tests")
            })
    })
}

pub fn classify(file: &str, mode: &str) -> Option<Spec> {
    if mode == "120000" || excluded(file) {
        return None;
    }
    let ext = extname(file);
    if let Some((_, lang)) = EXTENSIONS.iter().find(|(e, _)| *e == ext) {
        return Some(Spec { kind: if test_path(file) { Kind::Test } else { Kind::Source }, lang });
    }
    let lower = ext.to_ascii_lowercase();
    if [".md", ".rst", ".txt"].contains(&lower.as_str()) && !["LICENSE", "NOTICE", "COPYING"].iter().any(|w| file.contains(w)) {
        return Some(Spec { kind: Kind::Docs, lang: "Docs" });
    }
    None
}

/// Physical lines: newline count, plus one for an unterminated last line. A blob with a NUL
/// byte is binary and counts zero.
pub fn physical_lines(bytes: &[u8]) -> u64 {
    if bytes.contains(&0) {
        return 0;
    }
    let lines = bytes.iter().filter(|b| **b == b'\n').count() as u64;
    lines + u64::from(bytes.last().is_some_and(|b| *b != b'\n'))
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub oid: String,
    pub kind: Kind,
    pub lang: &'static str,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Size {
    pub source: u64,
    pub test: u64,
    pub docs: u64,
    pub files: u64,
    pub testfiles: u64,
    pub docfiles: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counts {
    pub size: Size,
    pub langs: BTreeMap<String, u64>,
}

impl Counts {
    fn add(&mut self, other: &Counts) {
        let (a, b) = (&mut self.size, &other.size);
        a.source += b.source;
        a.test += b.test;
        a.docs += b.docs;
        a.files += b.files;
        a.testfiles += b.testfiles;
        a.docfiles += b.docfiles;
        for (lang, n) in &other.langs {
            *self.langs.entry(lang.clone()).or_insert(0) += n;
        }
    }
}

/// Sum a tree's measured blobs. `seen` is shared across every repository of one snapshot, so
/// a blob with identical content in two repositories (or twice in one) is counted once.
pub fn count_entries(entries: &[Entry], blob_lines: &HashMap<String, u64>, seen: &mut HashSet<String>) -> Result<Counts, String> {
    let mut c = Counts::default();
    for e in entries {
        if !seen.insert(e.oid.clone()) {
            continue;
        }
        let lines = *blob_lines.get(&e.oid).ok_or("Invalid blob line count")?;
        match e.kind {
            Kind::Source => c.size.source += lines,
            Kind::Test => c.size.test += lines,
            Kind::Docs => c.size.docs += lines,
        }
        if e.kind == Kind::Docs {
            c.size.docfiles += 1;
        } else {
            c.size.files += 1;
            if e.kind == Kind::Test {
                c.size.testfiles += 1;
            }
            *c.langs.entry(e.lang.to_string()).or_insert(0) += lines;
        }
    }
    Ok(c)
}

// ---------------------------------------------------------------------------------------------
// Snapshot validation: refuse before writing.
// ---------------------------------------------------------------------------------------------

const REQUIRED: &[&str] = &["source", "test", "docs", "loc", "files", "testfiles", "docfiles", "repos", "commits", "prs", "merged", "ci"];

/// JavaScript's `Number.isSafeInteger(x) && x >= 0`.
fn safe_count(v: Option<&Value>) -> Option<u64> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return (n < (1u64 << 53)).then_some(n);
    }
    let f = v.as_f64()?;
    (f >= 0.0 && f.fract() == 0.0 && f <= ((1u64 << 53) - 1) as f64).then_some(f as u64)
}

fn is_day(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
}

pub fn validate_snapshot(data: &Value) -> Result<(), String> {
    let series = data.get("series").and_then(Value::as_array).filter(|a| !a.is_empty());
    let repos = data.get("repos").and_then(Value::as_array).filter(|a| !a.is_empty());
    let asof_ok = data.get("asof").and_then(Value::as_str).and_then(parse_time_ms).is_some();
    let (Some(series), Some(repos), true) = (series, repos, asof_ok) else {
        return Err("Empty or invalid snapshot".into());
    };
    let mut prev = String::new();
    for row in series {
        let date = row.get("date").and_then(Value::as_str).unwrap_or("");
        if !is_day(date) || date <= prev.as_str() {
            return Err("Unordered snapshot dates".into());
        }
        prev = date.to_string();
        let mut m = HashMap::new();
        for key in REQUIRED {
            let n = safe_count(row.get(*key)).ok_or_else(|| format!("Missing or invalid metric: {key}"))?;
            m.insert(*key, n);
        }
        if m["loc"] != m["source"] + m["test"] || m["merged"] > m["prs"] || m["testfiles"] > m["files"] {
            return Err("Snapshot totals do not reconcile".into());
        }
        let langs = row.get("langs").and_then(Value::as_object).ok_or("Language total mismatch")?;
        let total: f64 = langs.values().map(|v| v.as_f64().unwrap_or(f64::NAN)).sum();
        if total != m["loc"] as f64 {
            return Err("Language total mismatch".into());
        }
    }
    let latest = series.last().expect("non-empty");
    let summary = data.get("summary").ok_or("Summary differs from the latest snapshot: source")?;
    for key in REQUIRED {
        if summary.get(*key) != latest.get(*key) {
            return Err(format!("Summary differs from the latest snapshot: {key}"));
        }
    }
    let names: HashSet<&str> = repos.iter().filter_map(|r| r.get("name").and_then(Value::as_str)).collect();
    if names.len() != repos.len() {
        return Err("Duplicate repositories".into());
    }
    let sum = |key: &str| -> f64 { repos.iter().map(|r| r.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN)).sum() };
    let summary_n = |key: &str| summary.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN);
    if summary_n("prs") != sum("prCount") {
        return Err("PR totals do not reconcile".into());
    }
    if summary_n("ci") != sum("ciCount") {
        return Err("CI totals do not reconcile".into());
    }
    let conclusions: f64 = data
        .get("ciConclusions")
        .and_then(Value::as_object)
        .map(|o| o.values().map(|v| v.as_f64().unwrap_or(f64::NAN)).sum())
        .unwrap_or(f64::NAN);
    if summary_n("ci") != conclusions {
        return Err("CI conclusions do not reconcile".into());
    }
    for repo in repos {
        let head = repo.get("head").and_then(Value::as_str).unwrap_or("");
        if head.len() != 40 || !head.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
            return Err("Missing pinned head".into());
        }
        // Snapshots written before 2026-10-17 carry no link; a link that is present must be a
        // GitHub repository address.
        if let Some(url) = repo.get("url") {
            let path = url.as_str().and_then(|u| u.strip_prefix("https://github.com/")).unwrap_or("");
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() != 2 || !parts.iter().all(|p| valid_repo_name(p) && !p.bytes().all(|c| c == b'.')) {
                return Err("Invalid repository link".into());
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// GitHub: public listings through `gh api`, every page, duplicates removed.
// ---------------------------------------------------------------------------------------------

/// Follow `per_page=100&page=N` until a short page. Records are keyed by `id`, else `url`,
/// else `number`; a record seen twice keeps its first position and its latest value.
pub fn paginate<F>(endpoint: &str, key: Option<&str>, mut get: F) -> Result<Vec<Value>, String>
where
    F: FnMut(&str) -> Result<Value, String>,
{
    let mut order: Vec<String> = Vec::new();
    let mut records: HashMap<String, Value> = HashMap::new();
    let join = if endpoint.contains('?') { '&' } else { '?' };
    for page in 1..=10_000 {
        let response = get(&format!("{endpoint}{join}per_page=100&page={page}"))?;
        let rows = match key {
            Some(k) => response.get(k),
            None => Some(&response),
        }
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Invalid GitHub collection: {endpoint}"))?;
        for row in rows {
            let id = ["id", "url", "number"]
                .iter()
                .find_map(|k| row.get(*k).filter(|v| !v.is_null()))
                .map(|v| v.to_string())
                .unwrap_or_else(|| "undefined".into());
            if records.insert(id.clone(), row.clone()).is_none() {
                order.push(id);
            }
        }
        if rows.len() < 100 {
            return Ok(order.into_iter().map(|id| records.remove(&id).expect("recorded")).collect());
        }
    }
    Err("Pagination exceeded the supported limit".into())
}

static API_CALLS: AtomicUsize = AtomicUsize::new(0);

/// One GET through `gh api`. Rate limits and server errors are retried three times.
fn github_get(endpoint: &str) -> Result<Value, String> {
    let path = endpoint.trim_start_matches('/');
    for attempt in 0..4u64 {
        API_CALLS.fetch_add(1, Ordering::Relaxed);
        let out = Command::new("gh")
            .args(["api", "-H", "Accept: application/vnd.github+json", "-H", "X-GitHub-Api-Version: 2022-11-28", path])
            .output()
            .map_err(|e| format!("cannot run gh: {e}"))?;
        if out.status.success() {
            return serde_json::from_slice(&out.stdout).map_err(|e| format!("GitHub sent bad JSON for {endpoint}: {e}"));
        }
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let retry = err.contains("HTTP 429") || err.contains("HTTP 5") || err.contains("rate limit");
        if retry && attempt < 3 {
            std::thread::sleep(std::time::Duration::from_secs(2 * (attempt + 1)));
            continue;
        }
        return Err(format!("GitHub refused {endpoint}: {err}"));
    }
    unreachable!("the last attempt returns")
}

// ---------------------------------------------------------------------------------------------
// Git: mirror clones and the objects inside them.
// ---------------------------------------------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .arg(format!("--git-dir={}", dir.display()))
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {} failed in {}: {}", args.join(" "), dir.display(), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

/// Clone or fetch, keeping only the end of the error output.
fn git_network(args: &[String]) -> Result<(), String> {
    let out = Command::new("git")
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let tail: String = err.chars().rev().take(3000).collect::<Vec<_>>().into_iter().rev().collect();
    Err(format!("git failed: {tail}"))
}

#[derive(Debug, Clone)]
struct Commit {
    sha: String,
    ts: i64,
    date: String,
}

fn commits_at(dir: &Path, head: &str, first_parent: bool) -> Result<Vec<Commit>, String> {
    let mut args = vec!["log"];
    if first_parent {
        args.push("--first-parent");
    }
    args.extend(["--format=%H%x09%ct", head]);
    let text = String::from_utf8(git(dir, &args)?).map_err(|_| "git log is not UTF-8")?;
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let (sha, ts) = line.split_once('\t').ok_or("Malformed git log line")?;
            let ts: i64 = ts.parse().map_err(|_| "Malformed git log time")?;
            Ok(Commit { sha: sha.to_string(), ts, date: day_ms(ts * 1000) })
        })
        .collect()
}

/// Parse `git cat-file --batch` output for `oids`, in order, storing each blob's line count.
pub fn read_blob_batch<R: BufRead>(mut out: R, oids: &[String], cache: &mut HashMap<String, u64>) -> Result<(), String> {
    let mut header = Vec::new();
    let mut body = Vec::new();
    for oid in oids {
        header.clear();
        out.read_until(b'\n', &mut header).map_err(|e| e.to_string())?;
        if header.last() != Some(&b'\n') {
            return Err("Truncated Git blob batch".into());
        }
        let text = String::from_utf8_lossy(&header[..header.len() - 1]).to_string();
        let parts: Vec<&str> = text.split(' ').collect();
        let size = parts.get(2).and_then(|s| s.parse::<usize>().ok());
        let (Some(size), true) = (size, parts.len() == 3 && parts[0] == oid && parts[1] == "blob") else {
            return Err("Malformed Git blob batch".into());
        };
        body.resize(size + 1, 0);
        out.read_exact(&mut body).map_err(|_| "Malformed Git blob batch")?;
        cache.insert(oid.clone(), physical_lines(&body[..size]));
    }
    Ok(())
}

fn cat_blobs(dir: &Path, oids: &[String], cache: &mut HashMap<String, u64>) -> Result<(), String> {
    let mut child = Command::new("git")
        .arg(format!("--git-dir={}", dir.display()))
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run git: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped");
    let input: String = oids.iter().map(|o| format!("{o}\n")).collect();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let parsed = read_blob_batch(BufReader::with_capacity(1 << 20, child.stdout.take().expect("piped")), oids, cache);
    let wrote = writer.join().map_err(|_| "git cat-file writer panicked")?;
    let status = child.wait().map_err(|e| e.to_string())?;
    parsed?;
    wrote.map_err(|e| format!("git cat-file input: {e}"))?;
    if !status.success() {
        return Err(format!("git cat-file failed in {}", dir.display()));
    }
    Ok(())
}

/// Measured blobs of one tree. Trees are cached per repository and commit, blob line counts
/// per object id (identical content has an identical id in every repository).
fn read_entries(
    repo: &Repo,
    head: &str,
    blobs: &mut HashMap<String, u64>,
    trees: &mut HashMap<String, Rc<Vec<Entry>>>,
) -> Result<Rc<Vec<Entry>>, String> {
    let key = format!("{}:{}", repo.name, head);
    if let Some(t) = trees.get(&key) {
        return Ok(t.clone());
    }
    let listing = git(&repo.dir, &["ls-tree", "-r", "-z", head])?;
    let mut entries = Vec::new();
    for row in listing.split(|b| *b == 0) {
        if row.is_empty() {
            continue;
        }
        let row = String::from_utf8_lossy(row);
        let (meta, file) = row.split_once('\t').ok_or("Malformed git ls-tree row")?;
        let mut it = meta.split(' ');
        let (mode, kind, oid) = (it.next().unwrap_or(""), it.next().unwrap_or(""), it.next().unwrap_or(""));
        if kind == "blob" {
            if let Some(spec) = classify(file, mode) {
                entries.push(Entry { oid: oid.to_string(), kind: spec.kind, lang: spec.lang });
            }
        }
    }
    let mut missing: Vec<String> = Vec::new();
    let mut queued = HashSet::new();
    for e in &entries {
        if !blobs.contains_key(&e.oid) && queued.insert(e.oid.clone()) {
            missing.push(e.oid.clone());
        }
    }
    if !missing.is_empty() {
        cat_blobs(&repo.dir, &missing, blobs)?;
    }
    let entries = Rc::new(entries);
    trees.insert(key, entries.clone());
    Ok(entries)
}

// ---------------------------------------------------------------------------------------------
// Collection.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Pr {
    number: Value,
    title: Value,
    created: String,
    merged: Option<String>,
    merged_ms: Option<i64>,
    state: String,
    draft: bool,
    url: Value,
    created_day: String,
    merged_day: Option<String>,
}

#[derive(Debug, Clone)]
struct Run {
    conclusion: Option<String>,
    status: Option<String>,
    created_day: String,
}

#[derive(Debug, Clone)]
struct Repo {
    name: String,
    url: String,
    dir: PathBuf,
    head: String,
    branch: String,
    commits: Vec<Commit>,
    fp: Vec<Commit>,
    prs: Vec<Pr>,
    runs: Vec<Run>,
    archived: bool,
    group: &'static str,
    first: String,
    last: String,
}

#[derive(Serialize)]
struct Row {
    date: String,
    #[serde(flatten)]
    size: Size,
    langs: BTreeMap<String, u64>,
    loc: u64,
    repos: u64,
    commits: u64,
    daily: u64,
    prs: u64,
    merged: u64,
    ci: u64,
    #[serde(rename = "ciDaily")]
    ci_daily: u64,
}

#[derive(Serialize)]
struct Summary {
    #[serde(flatten)]
    size: Size,
    loc: u64,
    repos: u64,
    commits: u64,
    daily: u64,
    prs: u64,
    merged: u64,
    ci: u64,
    #[serde(rename = "ciDaily")]
    ci_daily: u64,
    active: u64,
    archived: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicRepo {
    name: String,
    url: String,
    head: String,
    branch: String,
    first: String,
    last: String,
    archived: bool,
    group: &'static str,
    size: Size,
    langs: BTreeMap<String, u64>,
    revisions: u64,
    pr_count: u64,
    merged_count: u64,
    ci_count: u64,
}

#[derive(Serialize)]
struct Activity {
    repo: String,
    title: Value,
    number: Value,
    url: Value,
    state: &'static str,
    date: String,
    timestamp: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    schema_version: u32,
    asof: String,
    timezone: &'static str,
    first: String,
    last: String,
    series: Vec<Row>,
    summary: Summary,
    ci_conclusions: BTreeMap<String, u64>,
    repos: Vec<PublicRepo>,
    events: Value,
    activity: Vec<Activity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_policy: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit_policy: Option<Value>,
}

pub struct Options {
    pub output: PathBuf,
    pub cache_dir: PathBuf,
    /// Measurement cutoff (RFC 3339). Default: the moment the history has been read.
    pub asof: Option<String>,
    /// false: measure the mirrors as they are, without fetching (reproducible reruns).
    pub fetch: bool,
}

pub struct Report {
    pub repos: usize,
    pub loc: u64,
    pub commits: u64,
    pub prs: u64,
    pub ci: u64,
    pub asof: String,
    pub api_calls: usize,
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Run `f` over `items` with at most `limit` at a time, keeping order. Stops taking new items
/// after the first failure and returns that failure.
fn limited<T: Sync, R: Send>(items: &[T], limit: usize, f: impl Fn(&T) -> Result<R, String> + Sync) -> Result<Vec<R>, String> {
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let results: Vec<Mutex<Option<Result<R, String>>>> = items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..limit.min(items.len()) {
            scope.spawn(|| loop {
                if failed.load(Ordering::Relaxed) {
                    break;
                }
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= items.len() {
                    break;
                }
                let r = f(&items[i]);
                if r.is_err() {
                    failed.store(true, Ordering::Relaxed);
                }
                *results[i].lock().expect("unpoisoned") = Some(r);
            });
        }
    });
    let mut out = Vec::with_capacity(items.len());
    for r in results {
        match r.into_inner().expect("unpoisoned") {
            Some(Ok(v)) => out.push(v),
            Some(Err(e)) => return Err(e),
            None => {}
        }
    }
    if out.len() != items.len() {
        return Err("Repository collection stopped early".into());
    }
    Ok(out)
}

fn valid_repo_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

fn collect_repo(target: &Target, opts: &Options, cutoff: i64) -> Result<Repo, String> {
    let Target { name, owner, github_name, url, info } = target;
    let name = name.clone();
    let branch = s(info, "default_branch").unwrap_or_default();
    if ![&name, owner, github_name].iter().all(|n| valid_repo_name(n)) || branch.is_empty() {
        return Err("Invalid repository identity".into());
    }
    std::fs::create_dir_all(&opts.cache_dir).map_err(|e| format!("cannot create {}: {e}", opts.cache_dir.display()))?;
    let cache = std::fs::canonicalize(&opts.cache_dir).map_err(|e| e.to_string())?;
    let dir = cache.join(format!("{github_name}.git"));
    let remote = format!("https://github.com/{owner}/{github_name}.git");
    if opts.fetch {
        if dir.exists() {
            // A mirror made before a move still points at the old address; read from the current one.
            git(&dir, &["remote", "set-url", "origin", &remote])?;
            git_network(&[format!("--git-dir={}", dir.display()), "fetch".into(), "--prune".into(), "origin".into()])?;
        } else {
            git_network(&["clone".into(), "--mirror".into(), remote, dir.display().to_string()])?;
        }
    } else if !dir.exists() {
        return Err(format!("No mirror for {owner}/{github_name} in {} (run without --no-fetch first)", cache.display()));
    }
    let head = String::from_utf8_lossy(&git(&dir, &["rev-parse", &format!("refs/heads/{branch}")])?).trim().to_string();
    let commits = commits_at(&dir, &head, false)?;
    let fp = commits_at(&dir, &head, true)?;
    // Normal Git history cannot place the candidate's parent after its measurement cutoff.
    if commits.iter().any(|c| c.ts.saturating_mul(1000) > cutoff) {
        return Err(format!("Pinned repository contains a future-dated commit: {name}"));
    }
    let raw_prs = paginate(&format!("/repos/{owner}/{github_name}/pulls?state=all"), None, github_get)?;
    let raw_runs = paginate(&format!("/repos/{owner}/{github_name}/actions/runs"), Some("workflow_runs"), github_get)?;
    let mut prs = Vec::new();
    for p in &raw_prs {
        let Some(created) = s(p, "created_at") else { continue };
        if parse_time_ms(&created).is_none_or(|t| t > cutoff) {
            continue;
        }
        let merged = s(p, "merged_at").filter(|m| !m.is_empty());
        let merged_ms = merged.as_deref().and_then(parse_time_ms);
        prs.push(Pr {
            number: p.get("number").cloned().unwrap_or(Value::Null),
            title: p.get("title").cloned().unwrap_or(Value::Null),
            created_day: day(&created)?,
            merged_day: merged.as_deref().map(day).transpose()?,
            created,
            merged,
            merged_ms,
            state: s(p, "state").unwrap_or_default(),
            draft: p.get("draft").and_then(Value::as_bool).unwrap_or(false),
            url: p.get("html_url").cloned().unwrap_or(Value::Null),
        });
    }
    // The same run reached through two pages counts once.
    let mut by_url: Vec<String> = Vec::new();
    let mut unique: HashMap<String, &Value> = HashMap::new();
    for r in &raw_runs {
        let key = r.get("html_url").filter(|v| !v.is_null()).or_else(|| r.get("url")).map(|v| v.to_string()).unwrap_or_else(|| "undefined".into());
        if unique.insert(key.clone(), r).is_none() {
            by_url.push(key);
        }
    }
    let mut runs = Vec::new();
    for key in &by_url {
        let r = unique[key];
        let Some(created) = s(r, "created_at") else { continue };
        if parse_time_ms(&created).is_none_or(|t| t > cutoff) {
            continue;
        }
        runs.push(Run {
            conclusion: s(r, "conclusion").filter(|c| !c.is_empty()),
            status: s(r, "status"),
            created_day: day(&created)?,
        });
    }
    let archived = info.get("archived").and_then(Value::as_bool).unwrap_or(false);
    let group = group_of(&name, archived);
    let first = commits.iter().map(|c| c.date.as_str()).fold("9999-12-31", |a, b| if b < a { b } else { a }).to_string();
    let last = commits.iter().map(|c| c.date.as_str()).fold("0000-01-01", |a, b| if b > a { b } else { a }).to_string();
    Ok(Repo { name, url: url.clone(), dir, head, branch, commits, fp, prs, runs, archived, group, first, last })
}

pub fn collect(opts: &Options) -> Result<Report, String> {
    let mut cutoff = match &opts.asof {
        Some(a) => parse_time_ms(a).ok_or("Invalid measurement cutoff")?,
        None => i64::MAX,
    };
    let existing: Value = serde_json::from_str(
        &std::fs::read_to_string(&opts.output).map_err(|e| format!("cannot read the existing {}: {e}", opts.output.display()))?,
    )
    .map_err(|e| format!("{} is not JSON: {e}", opts.output.display()))?;

    let mut listing = Vec::new();
    for owner in owners() {
        match paginate(&format!("/users/{owner}/repos?type=owner&sort=full_name"), None, github_get) {
            Ok(rows) => listing.extend(rows),
            Err(e) => eprintln!("Warning: skipping repositories of {owner}: {e}"),
        }
    }
    let (targets, missing) = select_repos(&listing);
    for m in &missing {
        eprintln!("Warning: expected public project repository missing, skipping: {m}");
    }
    let started = std::time::Instant::now();
    let repos = limited(&targets, 4, |target| collect_repo(target, opts, cutoff))?;
    println!("  [PROGRESS] read {} repositories (git history, PRs, runs) in {:.2?}", repos.len(), started.elapsed());

    let asof = match &opts.asof {
        Some(a) => a.clone(),
        None => {
            let a = iso_ms(now_ms());
            cutoff = parse_time_ms(&a).ok_or("Invalid measurement cutoff")?;
            a
        }
    };
    if repos.iter().any(|r| r.commits.iter().any(|c| c.ts.saturating_mul(1000) > cutoff)) {
        return Err("Pinned history contains a future-dated commit".into());
    }
    let first = repos.iter().map(|r| r.first.as_str()).fold("9999-12-31", |a, b| if b < a { b } else { a }).to_string();
    let last = day_ms(cutoff);
    let first_days = parse_time_ms(&first).ok_or("Invalid first day")?.div_euclid(86_400_000);
    let last_days = parse_time_ms(&last).ok_or("Invalid last day")?.div_euclid(86_400_000);
    if last_days - first_days + 1 > 10_000 {
        return Err("Unexpected historical range".into());
    }
    let dates: Vec<String> = (first_days..=last_days).map(ymd).collect();

    let mut blobs: HashMap<String, u64> = HashMap::new();
    let mut trees: HashMap<String, Rc<Vec<Entry>>> = HashMap::new();
    // Distinct commit SHAs across every repository, by Chicago day.
    let mut commit_days: HashMap<&str, &str> = HashMap::new();
    for r in &repos {
        for c in &r.commits {
            commit_days.insert(&c.sha, &c.date);
        }
    }
    let mut commit_dates: Vec<&str> = commit_days.into_values().collect();
    commit_dates.sort_unstable();
    let all_prs: Vec<&Pr> = repos.iter().flat_map(|r| r.prs.iter()).collect();
    let all_runs: Vec<&Run> = repos.iter().flat_map(|r| r.runs.iter()).collect();

    let mut series = Vec::with_capacity(dates.len());
    for date in &dates {
        let mut seen = HashSet::new();
        let mut total = Counts::default();
        let mut number = 0;
        for r in &repos {
            let Some(candidate) = r.fp.iter().find(|c| c.date.as_str() <= date.as_str()) else { continue };
            number += 1;
            let entries = read_entries(r, &candidate.sha, &mut blobs, &mut trees)?;
            total.add(&count_entries(&entries, &blobs, &mut seen)?);
        }
        let upto = commit_dates.partition_point(|d| *d <= date.as_str());
        let before = commit_dates.partition_point(|d| *d < date.as_str());
        let ps: Vec<&&Pr> = all_prs.iter().filter(|p| p.created_day.as_str() <= date.as_str()).collect();
        let merged = ps
            .iter()
            .filter(|p| p.merged.is_some() && p.merged_ms.is_some_and(|m| m <= cutoff) && p.merged_day.as_deref().is_some_and(|d| d <= date.as_str()))
            .count();
        let ci = all_runs.iter().filter(|r| r.created_day.as_str() <= date.as_str()).count();
        let ci_daily = all_runs.iter().filter(|r| r.created_day == *date).count();
        series.push(Row {
            date: date.clone(),
            loc: total.size.source + total.size.test,
            size: total.size,
            langs: total.langs,
            repos: number,
            commits: upto as u64,
            daily: (upto - before) as u64,
            prs: ps.len() as u64,
            merged: merged as u64,
            ci: ci as u64,
            ci_daily: ci_daily as u64,
        });
    }

    println!("  [PROGRESS] measured {} daily snapshots in {:.2?}", series.len(), started.elapsed());
    let mut ci_conclusions: BTreeMap<String, u64> = BTreeMap::new();
    for r in &repos {
        for run in &r.runs {
            let key = run.conclusion.clone().or_else(|| run.status.clone()).unwrap_or_else(|| "null".into());
            *ci_conclusions.entry(key).or_insert(0) += 1;
        }
    }
    let mut public_repos = Vec::with_capacity(repos.len());
    for r in &repos {
        let entries = read_entries(r, &r.head, &mut blobs, &mut trees)?;
        let counts = count_entries(&entries, &blobs, &mut HashSet::new())?;
        public_repos.push(PublicRepo {
            name: r.name.clone(),
            url: r.url.clone(),
            head: r.head.clone(),
            branch: r.branch.clone(),
            first: r.first.clone(),
            last: r.last.clone(),
            archived: r.archived,
            group: r.group,
            size: counts.size,
            langs: counts.langs,
            revisions: r.commits.len() as u64,
            pr_count: r.prs.len() as u64,
            merged_count: r.prs.iter().filter(|p| p.merged_ms.is_some_and(|m| m <= cutoff)).count() as u64,
            ci_count: r.runs.len() as u64,
        });
    }
    public_repos.sort_by_key(|r| std::cmp::Reverse(r.size.source + r.size.test));

    let latest = series.last().ok_or("Empty or invalid snapshot")?;
    let active = repos.iter().filter(|r| !r.archived).count() as u64;
    let summary = Summary {
        size: latest.size.clone(),
        loc: latest.loc,
        repos: latest.repos,
        commits: latest.commits,
        daily: latest.daily,
        prs: latest.prs,
        merged: latest.merged,
        ci: latest.ci,
        ci_daily: latest.ci_daily,
        active,
        archived: repos.len() as u64 - active,
    };

    let mut activity: Vec<(i64, Activity)> = Vec::new();
    for r in &repos {
        for p in r.prs.iter().filter(|p| p.state == "open" || p.merged.is_some()) {
            let timestamp = p.merged.clone().unwrap_or_else(|| p.created.clone());
            let state = if p.merged.is_some() { "Merged" } else if p.draft { "Draft" } else { "Open" };
            let ms = parse_time_ms(&timestamp).ok_or_else(|| format!("Invalid timestamp: {timestamp}"))?;
            activity.push((
                ms,
                Activity { repo: r.name.clone(), title: p.title.clone(), number: p.number.clone(), url: p.url.clone(), state, date: day_ms(ms), timestamp },
            ));
        }
    }
    activity.sort_by_key(|a| std::cmp::Reverse(a.0));
    let activity: Vec<Activity> = activity.into_iter().take(12).map(|(_, a)| a).collect();

    let events = existing.get("events").filter(|e| truthy(e)).cloned().unwrap_or_else(|| Value::Array(vec![]));
    let snapshot = Snapshot {
        schema_version: 2,
        asof: asof.clone(),
        timezone: TIMEZONE,
        first,
        last,
        series,
        summary,
        ci_conclusions,
        repos: public_repos,
        events,
        activity,
        source_policy: existing.get("sourcePolicy").cloned(),
        commit_policy: existing.get("commitPolicy").cloned(),
    };
    let value = serde_json::to_value(&snapshot).map_err(|e| e.to_string())?;
    validate_snapshot(&value)?;

    if let Some(parent) = opts.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let tmp = PathBuf::from(format!("{}.tmp", opts.output.display()));
    let text = serde_json::to_string(&snapshot).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &opts.output).map_err(|e| format!("cannot replace {}: {e}", opts.output.display()))?;
    Ok(Report {
        repos: repos.len(),
        loc: snapshot.summary.loc,
        commits: snapshot.summary.commits,
        prs: snapshot.summary.prs,
        ci: snapshot.summary.ci,
        asof,
        api_calls: API_CALLS.load(Ordering::Relaxed),
    })
}

/// JavaScript truthiness for a JSON value (`existing.events || []`).
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(file: &str) -> Option<Kind> {
        classify(file, "100644").map(|s| s.kind)
    }

    #[test]
    fn measurement_boundaries_keep_named_tests_and_exclude_evidence_third_party_and_symlinks() {
        assert_eq!(kind("src/runtime/main.c"), Some(Kind::Source));
        assert_eq!(kind("tests/compiler/check.inc"), Some(Kind::Test));
        assert_eq!(kind("src/lib.rs"), Some(Kind::Source));
        for p in ["vendor/d3.js", "public/vendor/d3.js", "evidence/old.c", "tests/fixtures/sample.rs", "data/generated.c"] {
            assert_eq!(classify(p, "100644"), None, "{p}");
        }
        assert_eq!(classify("alias.c", "120000"), None);
        assert_eq!(classify("scripts/collect-progress.mjs", "100644").map(|s| s.lang), Some("Web"));
    }

    #[test]
    fn test_path_and_extension_rules_match_the_former_regular_expressions() {
        for p in ["src/foo_test.rs", "src/test_foo.py", "Tests/x.c", "a/test.c", "a/tests.rs", "web/app.spec.ts", "web/App.TEST.js", "x/unit_tests/y.rs", "x/my_tests.c"] {
            assert_eq!(kind(p), Some(Kind::Test), "{p}");
        }
        for p in ["src/testing.rs", "src/contest.rs", "src/latest/x.rs", "src/attest_x.c"] {
            assert_eq!(kind(p), Some(Kind::Source), "{p}");
        }
        assert_eq!(classify("Build/x.c", "100644"), None);
        assert_eq!(classify("README.md", "100644").map(|s| s.lang), Some("Docs"));
        assert_eq!(classify("docs/Notes.TXT", "100644").map(|s| s.kind), Some(Kind::Docs));
        assert_eq!(classify("LICENSE.md", "100644"), None);
        assert_eq!(classify("third-party/NOTICE.txt", "100644"), None);
        assert_eq!(classify(".bashrc", "100644"), None);
        assert_eq!(classify("boot/start.S", "100644").map(|s| s.lang), Some("Assembly"));
        assert_eq!(classify("photo.PNG", "100644"), None);
        assert_eq!(extname("a/..x"), ".x");
        assert_eq!(extname(".."), "");
        assert_eq!(extname("a/b.tar.gz"), ".gz");
    }

    #[test]
    fn physical_lines_handle_empty_unterminated_and_binary_blobs() {
        assert_eq!(physical_lines(b""), 0);
        assert_eq!(physical_lines(b"x"), 1);
        assert_eq!(physical_lines(b"x\n"), 1);
        assert_eq!(physical_lines(b"x\n\n"), 2);
        assert_eq!(physical_lines(&[65, 0, 10]), 0);
    }

    #[test]
    fn the_shared_blob_set_prevents_double_counting_migrated_files_across_repositories() {
        let lines: HashMap<String, u64> = [("shared".to_string(), 10), ("test".to_string(), 5)].into_iter().collect();
        let entry = |oid: &str, kind, lang| Entry { oid: oid.into(), kind, lang };
        let mut seen = HashSet::new();
        let a = count_entries(&[entry("shared", Kind::Source, "C")], &lines, &mut seen).unwrap();
        let b = count_entries(&[entry("shared", Kind::Source, "C"), entry("test", Kind::Test, "Rust")], &lines, &mut seen).unwrap();
        assert_eq!(a.size.source + b.size.source, 10);
        assert_eq!(b.size.test, 5);
        assert_eq!(b.size.files, 1);
        assert!(count_entries(&[entry("missing", Kind::Test, "C")], &lines, &mut HashSet::new()).is_err());
    }

    #[test]
    fn github_collections_follow_every_page_and_deduplicate_overlapping_records() {
        let mut calls = Vec::new();
        let rows = paginate("/example", Some("workflow_runs"), |url| {
            calls.push(url.to_string());
            let rows: Vec<Value> = if url.ends_with("page=1") {
                (0..100).map(|id| serde_json::json!({ "id": id })).collect()
            } else {
                vec![serde_json::json!({ "id": 99 }), serde_json::json!({ "id": 100 })]
            };
            Ok(serde_json::json!({ "workflow_runs": rows }))
        })
        .unwrap();
        assert_eq!(rows.len(), 101);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "/example?per_page=100&page=1");
        let bad = paginate("/bad", Some("workflow_runs"), |_| Ok(serde_json::json!({}))).unwrap_err();
        assert!(bad.contains("Invalid GitHub collection"), "{bad}");
        let mut seen = Vec::new();
        paginate("/x?state=all", None, |url| {
            seen.push(url.to_string());
            Ok(serde_json::json!([]))
        })
        .unwrap();
        assert_eq!(seen, ["/x?state=all&per_page=100&page=1"]);
    }

    #[test]
    fn daily_boundaries_use_chicago_rather_than_utc_including_daylight_saving_time() {
        assert_eq!(day("2026-10-01T03:00:00Z").unwrap(), "2026-09-30");
        assert_eq!(day("2026-12-01T05:30:00Z").unwrap(), "2026-11-30");
        // The switches themselves: 2026-03-08 08:00 UTC and 2026-11-01 07:00 UTC.
        let at = |t: &str| chicago_offset(parse_time_ms(t).unwrap() / 1000) / 3600;
        assert_eq!(at("2026-03-08T07:59:59Z"), -6);
        assert_eq!(at("2026-03-08T08:00:00Z"), -5);
        assert_eq!(at("2026-11-01T06:59:59Z"), -5);
        assert_eq!(at("2026-11-01T07:00:00Z"), -6);
        assert_eq!(at("2027-03-14T08:00:00Z"), -5);
        assert_eq!(at("2006-04-02T08:00:00Z"), -5);
        assert_eq!(at("2006-10-29T07:00:00Z"), -6);
        assert_eq!(day("2026-10-01T05:00:00Z").unwrap(), "2026-10-01");
        assert_eq!(day("2026-12-01T06:00:00Z").unwrap(), "2026-12-01");
    }

    #[test]
    fn timestamps_parse_and_print_like_javascript() {
        assert_eq!(parse_time_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_time_ms("2026-10-01T19:37:24.600923+00:00"), parse_time_ms("2026-10-01T19:37:24.600Z"));
        assert_eq!(parse_time_ms("2026-10-01T14:37:24-05:00"), parse_time_ms("2026-10-01T19:37:24Z"));
        assert_eq!(parse_time_ms("2026-10-16"), parse_time_ms("2026-10-16T00:00:00Z"));
        for bad in ["", "main", "2026-02-30T00:00:00Z", "2026-10-01T19:37:24", "2026-10-01T25:00:00Z"] {
            assert_eq!(parse_time_ms(bad), None, "{bad}");
        }
        let t = parse_time_ms("2026-10-16T17:04:05.123Z").unwrap();
        assert_eq!(iso_ms(t), "2026-10-16T17:04:05.123Z");
        assert_eq!(ymd(days_from_civil(2024, 2, 29)), "2024-02-29");
    }

    #[test]
    fn blob_batches_are_read_in_order_and_malformed_batches_refused() {
        let oids = vec!["a".to_string(), "b".to_string()];
        let mut cache = HashMap::new();
        read_blob_batch(&b"a blob 4\nx\ny\n\nb blob 0\n\n"[..], &oids, &mut cache).unwrap();
        assert_eq!(cache["a"], 2);
        assert_eq!(cache["b"], 0);
        assert!(read_blob_batch(&b"a missing\n"[..], &oids, &mut HashMap::new()).is_err());
        assert!(read_blob_batch(&b"a blob 9\nshort"[..], &oids, &mut HashMap::new()).is_err());
        assert!(read_blob_batch(&b"b blob 0\n\n"[..], &oids, &mut HashMap::new()).is_err());
    }

    #[test]
    fn repository_names_follow_the_project_rule() {
        for n in ["aien-edge", "aienos", "aienos.com", "omega", "omega-mv", "spark-hive", "crumb-spec", "atlas"] {
            assert!(project_named(n), "{n}");
        }
        for n in ["omegaman", "aien", "forgery", "physicsx", "cortex", "sparky"] {
            assert!(!project_named(n), "{n}");
        }
        assert_eq!(owners(), vec!["aien-dev", "aien-spark"]);
    }

    /// The public repository listing of aien-dev and aien-spark as GitHub returned it on
    /// 2026-10-17 (`/users/{owner}/repos?type=owner`): owner, name, fork, archived.
    const LISTING_2026_10_17: &[&str] = &[
        "aien-dev .github - -",
        "aien-dev aegis-runtime-mv - -",
        "aien-dev agent-archive - -",
        "aien-dev aien-architecture-mv - -",
        "aien-dev aien-edge - -",
        "aien-dev aien-harness - archived",
        "aien-dev aien-local-stack - archived",
        "aien-dev aien-protocols-mv - -",
        "aien-dev aien-receipts - -",
        "aien-dev aien-sovereign-core-mv - -",
        "aien-dev aien-yardsticks - archived",
        "aien-dev aienix - -",
        "aien-dev aienos-mv - -",
        "aien-dev aienos.com - -",
        "aien-dev arm-trusted-firmware fork archived",
        "aien-dev atlas - -",
        "aien-dev benchmarks-mv - -",
        "aien-dev cortex-rs - archived",
        "aien-dev crumb-spec-mv - archived",
        "aien-dev edk2 fork archived",
        "aien-dev edk2-non-osi fork -",
        "aien-dev edk2-platforms fork archived",
        "aien-dev encounter - archived",
        "aien-dev harvester - archived",
        "aien-dev interplane-mv - -",
        "aien-dev interplane.io - -",
        "aien-dev mobhub - -",
        "aien-dev MojoLlama - archived",
        "aien-dev odysseus fork archived",
        "aien-dev omega-mv - -",
        "aien-dev open-humanity - -",
        "aien-dev physics-mv - -",
        "aien-dev physics0 - archived",
        "aien-dev pi-register - -",
        "aien-dev rad-id-sync - archived",
        "aien-dev readings-window - -",
        "aien-dev rpi5-uefi fork -",
        "aien-dev spark-adapters - archived",
        "aien-dev spark-crumbs-mv - archived",
        "aien-dev spark-debugger - archived",
        "aien-dev spark-dream - archived",
        "aien-dev spark-hive - archived",
        "aien-dev spark-inquisitor - archived",
        "aien-dev spark-rsi-mv - -",
        "aien-dev spark-supervisor - archived",
        "aien-dev synapse - -",
        "aien-dev t3code fork -",
        "aien-spark aien-dev - -",
        "aien-spark drakestapleton.com - -",
        "aien-spark jan fork archived",
        "aien-spark NightDriverStrip fork archived",
        "aien-spark waldo fork archived",
    ];

    fn listing(rows: &[&str]) -> Vec<Value> {
        rows.iter()
            .map(|row| {
                let f: Vec<&str> = row.split(' ').collect();
                serde_json::json!({
                    "owner": { "login": f[0] }, "name": f[1], "fork": f[2] == "fork", "archived": f[3] == "archived",
                    "private": f[2] == "private", "html_url": format!("https://github.com/{}/{}", f[0], f[1]),
                })
            })
            .collect()
    }

    fn find<'a>(targets: &'a [Target], name: &str) -> &'a Target {
        targets.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name} not selected"))
    }

    #[test]
    fn the_repository_map_follows_the_move_to_aien_dev() {
        let mut rows = LISTING_2026_10_17.to_vec();
        rows.push("aien-dev aien-hidden private -");
        let (targets, missing) = select_repos(&listing(&rows));
        assert!(missing.is_empty(), "missing: {missing:?}");
        // Every named repository is read from where GitHub keeps it now, under its project name.
        for (project, owner, github_name) in REPOS {
            let t = find(&targets, project);
            assert_eq!((t.owner.as_str(), t.github_name.as_str()), (*owner, *github_name), "{project}");
            assert_eq!(t.url, format!("https://github.com/{owner}/{github_name}"), "{project}");
        }
        // The -mv core repositories stay Core; the two the old map lost are counted again.
        for core in ["aien-sovereign-core", "aienos", "omega", "physics", "aien-protocols", "aien-architecture", "benchmarks", "atlas"] {
            let t = find(&targets, core);
            assert_eq!(group_of(&t.name, t.info["archived"].as_bool().unwrap()), "Core", "{core}");
        }
        assert_eq!(find(&targets, "benchmarks").github_name, "benchmarks-mv");
        assert_eq!(find(&targets, "open-humanity").owner, "aien-dev");
        // Links point at the real owner: still aien-spark for these two, aien-dev for the archived spark-crumbs.
        assert_eq!(find(&targets, "drakestapleton.com").url, "https://github.com/aien-spark/drakestapleton.com");
        assert_eq!(find(&targets, "aien-dev").url, "https://github.com/aien-spark/aien-dev");
        assert_eq!(find(&targets, "spark-crumbs").url, "https://github.com/aien-dev/spark-crumbs-mv");
        assert_eq!(group_of("spark-crumbs", true), "Archived");
        assert_eq!(group_of("drakestapleton.com", false), "Sites");
        // The name rule still finds new repositories; forks, private and unrelated names stay out.
        let mut names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        names.sort_unstable();
        let mut expected: Vec<&str> = REPOS.iter().map(|(p, _, _)| *p).chain(["aien-receipts", "aien-yardsticks"]).collect();
        expected.sort_unstable();
        assert_eq!(names, expected);
        assert_eq!(targets.len(), 31);
    }

    #[test]
    fn a_lifted_suffix_is_still_read_as_the_same_project() {
        // GitHub lifts the block and omega-mv becomes omega before the map is edited.
        let rows: Vec<String> = LISTING_2026_10_17.iter().map(|r| r.replace(" omega-mv ", " omega ")).collect();
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let (targets, missing) = select_repos(&listing(&rows));
        assert_eq!(missing, vec!["aien-dev/omega-mv (omega)".to_string()]);
        let t = find(&targets, "omega");
        assert_eq!((t.github_name.as_str(), t.url.as_str(), group_of(&t.name, false)), ("omega", "https://github.com/aien-dev/omega", "Core"));
        // Both names listed at once: the map wins and the project is read once.
        let mut both = LISTING_2026_10_17.to_vec();
        both.push("aien-dev omega - -");
        let (targets, _) = select_repos(&listing(&both));
        assert_eq!(targets.iter().filter(|t| t.name == "omega").count(), 1);
        assert_eq!(find(&targets, "omega").github_name, "omega-mv");
    }

    #[test]
    fn repository_links_must_be_github_addresses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../public/data/project-progress.json");
        let data: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let with = |url: Value| {
            let mut d = data.clone();
            d["repos"][0]["url"] = url;
            validate_snapshot(&d)
        };
        assert!(with(Value::from("https://github.com/aien-dev/omega-mv")).is_ok());
        assert!(with(Value::from("https://github.com/aien-spark/.github")).is_ok());
        for bad in ["https://example.com/aien-dev/omega", "https://github.com/aien-dev", "https://github.com/aien-dev/omega/tree/x", "https://github.com/../..", "javascript:alert(1)"] {
            assert!(with(Value::from(bad)).is_err(), "{bad}");
        }
        assert!(with(Value::from(3)).is_err());
    }

    #[test]
    fn incomplete_and_inconsistent_observations_are_refused_before_publication() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../public/data/project-progress.json");
        let data: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        validate_snapshot(&data).unwrap();
        let corruptions: Vec<fn(&mut Value)> = vec![
            |d| {
                let row = d["series"].as_array_mut().unwrap().last_mut().unwrap();
                row["loc"] = Value::from(row["loc"].as_u64().unwrap() + 1);
            },
            |d| d["summary"]["ci"] = Value::from(d["summary"]["ci"].as_u64().unwrap() + 1),
            |d| d["repos"][0]["head"] = Value::from("main"),
            |d| d["series"] = Value::Array(vec![]),
            |d| {
                d["series"][0].as_object_mut().unwrap().remove("testfiles");
            },
        ];
        for corrupt in corruptions {
            let mut copy = data.clone();
            corrupt(&mut copy);
            assert!(validate_snapshot(&copy).is_err());
        }
    }
}
