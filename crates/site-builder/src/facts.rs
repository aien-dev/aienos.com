//! Live facts for the site. `facts` gathers them from GitHub through the `gh` CLI and writes
//! `facts.json`; `build` reads that file and renders it. Nothing on the page that can change is
//! typed by hand: it either comes from here or it is not on the page.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Editorial part: what each component is. Live part is filled in by `gather`.
pub const COMPONENTS: &[(&str, &str, &str, &str)] = &[
    // key, display name, repository, role (one line, ours)
    ("aienos", "AIENOS", "aien-dev/aienos", "The operating system. Agent-first; the model is never the kernel."),
    ("omega", "Omega", "aien-dev/omega", "The language and GPU engine the stack is being rewritten into."),
    ("sovereign-core", "Sovereign core", "aien-dev/aien-sovereign-core", "Host runtime, inference ABI, and the pipeline that judges every change."),
    ("architecture", "Architecture", "aien-dev/aien-architecture", "Flows, boundaries, decision records, execution plan."),
    ("protocols", "Protocols", "aien-dev/aien-protocols", "Specifications, wire protocols, schemas."),
    ("physics", "Physics", "aien-dev/physics", "The FORGE realization layer and the boot artefacts for the GB10."),
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Merge {
    pub number: u64,
    pub title: String,
    pub merged_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Component {
    pub key: String,
    pub name: String,
    pub repo: String,
    pub role: String,
    pub url: String,
    pub pushed_at: String,
    pub open_prs: u64,
    pub last_merge: Option<Merge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Decision {
    pub id: String,
    pub title: String,
    pub url: String,
}

/// The "AIEN Verification" commit status on the latest commit of a repository's main branch:
/// the receipt that the Spark judge published for it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Receipt {
    pub repo: String,
    pub sha: String,
    pub commit_title: String,
    pub committed_at: String,
    /// success | failure | pending | none
    pub state: String,
    pub description: String,
    pub target_url: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Facts {
    pub generated_at: String,
    pub components: Vec<Component>,
    pub decisions: Vec<Decision>,
    pub receipt: Receipt,
}

impl Facts {
    pub fn load(path: &Path) -> Result<Facts, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e} (run `site-builder facts {}` first)", path.display(), path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{} is not a facts file: {e}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

fn gh(args: &[&str]) -> Result<serde_json::Value, String> {
    let out = Command::new("gh")
        .arg("api")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run gh: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "gh api {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("gh api {}: bad JSON: {e}", args.join(" ")))
}

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn now_utc() -> String {
    // RFC 3339 to the second, without pulling in a time crate.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil from days (Howard Hinnant's algorithm)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{sec:02}Z")
}

/// Gather everything through `gh api`. Read-only, public data; a plain `GH_TOKEN` is enough.
pub fn gather() -> Result<Facts, String> {
    let mut components = Vec::new();
    for (key, name, repo, role) in COMPONENTS {
        let meta = gh(&[&format!("repos/{repo}")])?;
        let full = s(&meta, "full_name");
        let full = if full.is_empty() { repo.to_string() } else { full };
        let open = gh(&[&format!("search/issues?q=repo:{full}+is:pr+is:open&per_page=1")])?;
        let open_prs = open.get("total_count").and_then(|x| x.as_u64()).unwrap_or(0);
        let pulls = gh(&[&format!("repos/{full}/pulls?state=closed&sort=updated&direction=desc&per_page=30")])?;
        let last_merge = pulls
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|p| p.get("merged_at").map(|m| !m.is_null()).unwrap_or(false))
                    .max_by(|a, b| s(a, "merged_at").cmp(&s(b, "merged_at")))
                    .map(|p| Merge {
                        number: p.get("number").and_then(|n| n.as_u64()).unwrap_or(0),
                        title: s(p, "title"),
                        merged_at: s(p, "merged_at"),
                        url: s(p, "html_url"),
                    })
            })
            .unwrap_or(None);
        components.push(Component {
            key: key.to_string(),
            name: name.to_string(),
            repo: repo.to_string(),
            role: role.to_string(),
            url: format!("https://github.com/{repo}"),
            pushed_at: s(&meta, "pushed_at"),
            open_prs,
            last_merge,
        });
    }

    // Decision records: the newest five by number, titles from the file names.
    let adr = gh(&["repos/aien-dev/aien-architecture/contents/docs/adr"])?;
    let mut decisions: Vec<Decision> = adr
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    let name = s(f, "name");
                    let stem = name.strip_suffix(".md")?;
                    let (id, slug) = stem.split_once('-')?;
                    if id.len() != 4 || !id.chars().all(|c| c.is_ascii_digit()) {
                        return None;
                    }
                    let mut title = slug.replace('-', " ");
                    if let Some(first) = title.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                    Some(Decision { id: id.to_string(), title, url: s(f, "html_url") })
                })
                .collect()
        })
        .unwrap_or_default();
    decisions.sort_by(|a, b| b.id.cmp(&a.id));
    decisions.truncate(5);
    // Prefer each record's own heading ("# ADR 0035: Persistent Cognitive Entity Boundary (ALLEN)").
    for d in &mut decisions {
        let raw = Command::new("gh")
            .args(["api", "-H", "Accept: application/vnd.github.raw"])
            .arg(format!("repos/aien-dev/aien-architecture/contents/docs/adr/{}", d.url.rsplit('/').next().unwrap_or("")))
            .output();
        if let Ok(out) = raw {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(h) = text.lines().find_map(|l| l.strip_prefix("# ")) {
                    let h = h.trim();
                    let h = h.split_once(':').filter(|(a, _)| a.trim().eq_ignore_ascii_case(&format!("ADR {}", d.id))).map(|(_, t)| t.trim()).unwrap_or(h);
                    if !h.is_empty() {
                        d.title = clip(h, 72);
                    }
                }
            }
        }
    }

    // The receipt: latest main commit of sovereign-core and its "AIEN Verification" status.
    let repo = "aien-dev/aien-sovereign-core";
    let commit = gh(&[&format!("repos/{repo}/commits/main")])?;
    let sha = s(&commit, "sha");
    let commit_title = commit
        .pointer("/commit/message")
        .and_then(|m| m.as_str())
        .and_then(|m| m.lines().next())
        .unwrap_or("")
        .to_string();
    let committed_at = commit.pointer("/commit/committer/date").and_then(|d| d.as_str()).unwrap_or("").to_string();
    // The judge's verdict is published as a check run named "AIEN Verification".
    let runs = gh(&[&format!("repos/{repo}/commits/{sha}/check-runs?per_page=100")])?;
    let verification = runs
        .get("check_runs")
        .and_then(|a| a.as_array())
        .and_then(|a| a.iter().filter(|r| s(r, "name") == "AIEN Verification").max_by(|a, b| s(a, "completed_at").cmp(&s(b, "completed_at"))));
    let receipt = match verification {
        Some(run) => {
            let (state, description) = match (s(run, "status").as_str(), s(run, "conclusion").as_str()) {
                ("completed", "success") => ("success", "Published Spark verification bundle matches this commit."),
                ("completed", "failure") | ("completed", "timed_out") => ("failure", "Verification failed for this commit."),
                ("completed", other) => ("pending", if other.is_empty() { "Verification finished without a verdict." } else { "Verification finished without a pass or fail verdict." }),
                _ => ("pending", "Verification is still running for this commit."),
            };
            Receipt {
                repo: repo.to_string(),
                sha,
                commit_title,
                committed_at,
                state: state.to_string(),
                description: description.to_string(),
                target_url: s(run, "html_url"),
                updated_at: s(run, "completed_at"),
            }
        }
        None => Receipt {
            repo: repo.to_string(),
            sha,
            commit_title,
            committed_at,
            state: "none".to_string(),
            description: "No verification status published for this commit yet.".to_string(),
            target_url: String::new(),
            updated_at: String::new(),
        },
    };

    Ok(Facts { generated_at: now_utc(), components, decisions, receipt })
}

/// Cut a long title at a word boundary so it reads as a title, not a sentence.
pub fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max).collect();
    let cut = head.rfind(' ').unwrap_or(head.len());
    format!("{}...", head[..cut].trim_end_matches([',', ':', ';']))
}

/// "2026-10-16T15:48:43Z" -> "16 Oct 2026". Anything else is returned unchanged.
pub fn short_date(ts: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let date = ts.split('T').next().unwrap_or(ts);
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return ts.to_string();
    }
    let month = parts[1].parse::<usize>().ok().filter(|m| (1..=12).contains(m));
    let day = parts[2].trim_start_matches('0');
    match month {
        Some(m) => format!("{day} {} {}", MONTHS[m - 1], parts[0]),
        None => ts.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_date_reads_rfc3339() {
        assert_eq!(short_date("2026-10-16T15:48:43Z"), "16 Oct 2026");
        assert_eq!(short_date("2026-01-05"), "5 Jan 2026");
        assert_eq!(short_date("not a date"), "not a date");
    }

    #[test]
    fn clip_cuts_at_a_word() {
        assert_eq!(clip("short", 72), "short");
        assert_eq!(clip("AIEN Verification: Check IR (manifest v2), exactness classes", 30), "AIEN Verification: Check IR...");
    }

    #[test]
    fn now_utc_is_rfc3339_shaped() {
        let t = now_utc();
        assert_eq!(t.len(), 20, "{t}");
        assert!(t.ends_with('Z'));
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "T");
    }

    #[test]
    fn facts_round_trip() {
        let f = Facts { generated_at: "2026-10-16T00:00:00Z".into(), ..Default::default() };
        let text = serde_json::to_string(&f).unwrap();
        let back: Facts = serde_json::from_str(&text).unwrap();
        assert_eq!(back.generated_at, f.generated_at);
    }
}
