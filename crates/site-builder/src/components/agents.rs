//! /agents/: every public repository of the organization with the
//! instructions a coding agent needs to work on it.
//!
//! Two files make up an agent's brief: the shared rules every AIEN agent
//! follows (`public/agents/RULES.md`, published at /agents/RULES.md) and the
//! repository's own `AGENTS.md`. The repository list is not typed by hand:
//! `site-builder agents` reads it from GitHub into
//! `public/data/agent-repos.json`, the pages workflow refreshes that file
//! before every build, and the build renders the page from it.

use std::path::Path;
use std::process::Command;

use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::components::boxlink::{foot_labelled, Target};
use crate::facts::{gh, now_utc, retired_name};

pub const TITLE: &str = "Agents | AIENOS";
pub const DESCRIPTION: &str = "Point your own coding agent at any public AIEN repository: the rules our agents work by, and each repository's own AGENTS.md, refreshed hourly from GitHub.";
pub const URL: &str = "https://www.aienos.com/agents/";

/// The organization whose public repositories are listed.
pub const ORG: &str = "aien-dev";
/// Where `site-builder agents` writes the list and the build reads it.
pub const DATA_PATH: &str = "public/data/agent-repos.json";
/// The shared rules, compiled in so the page and the file never disagree.
pub const RULES: &str = include_str!("../../../../public/agents/RULES.md");
pub const RULES_PATH: &str = "/agents/RULES.md";
pub const RULES_URL: &str = "https://www.aienos.com/agents/RULES.md";
/// The source of the rules on GitHub, for the rules box.
pub const RULES_SOURCE: &str = "https://github.com/aien-dev/aienos.com/blob/main/public/agents/RULES.md";
/// Instruction files looked for in each repository, in order of preference.
pub const BRIEF_FILES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Repo {
    pub name: String,
    pub description: String,
    pub url: String,
    pub default_branch: String,
    pub fork: bool,
    pub pushed_at: String,
    /// The first of BRIEF_FILES present on the default branch, if any.
    pub brief: Option<String>,
}

impl Repo {
    /// The brief on GitHub, or None when the repository has none yet.
    pub fn brief_url(&self) -> Option<String> {
        self.brief.as_ref().map(|f| format!("{}/blob/{}/{f}", self.url, self.default_branch))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Repos {
    pub generated_at: String,
    pub org: String,
    pub repos: Vec<Repo>,
}

impl Repos {
    pub fn load(path: &Path) -> Result<Repos, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e} (run `site-builder agents {}` first)", path.display(), path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{} is not an agent repository list: {e}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// The repositories the page shows: retired names are never rendered.
    pub fn listed(&self) -> Vec<&Repo> {
        self.repos.iter().filter(|r| !retired_name(&r.name)).collect()
    }
}

/// Whether `path` exists on GitHub. A 404 is a plain no; any other failure
/// (rate limit, network) is an error, so a missing answer never reads as
/// a missing file.
fn exists(path: &str) -> Result<bool, String> {
    let out = Command::new("gh")
        .args(["api", "--silent", path])
        .output()
        .map_err(|e| format!("cannot run gh: {e}"))?;
    if out.status.success() {
        return Ok(true);
    }
    let err = String::from_utf8_lossy(&out.stderr);
    if err.contains("HTTP 404") {
        Ok(false)
    } else {
        Err(format!("gh api {path} failed: {}", err.trim()))
    }
}

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// Read every public, unarchived repository of ORG and the brief file each
/// one carries. Read-only public data; a plain `GH_TOKEN` is enough.
pub fn gather() -> Result<Repos, String> {
    let list = gh(&[&format!("orgs/{ORG}/repos?type=public&per_page=100")])?;
    let list = list.as_array().ok_or("org repository list is not an array")?;
    if list.len() >= 100 {
        return Err("100 or more public repositories: the collector must paginate".into());
    }
    let mut repos = Vec::new();
    for v in list {
        if v.get("archived").and_then(|x| x.as_bool()).unwrap_or(false)
            || v.get("private").and_then(|x| x.as_bool()).unwrap_or(false)
        {
            continue;
        }
        let name = s(v, "name");
        let default_branch = s(v, "default_branch");
        let mut brief = None;
        for f in BRIEF_FILES {
            if exists(&format!("repos/{ORG}/{name}/contents/{f}?ref={default_branch}"))? {
                brief = Some(f.to_string());
                break;
            }
        }
        repos.push(Repo {
            description: s(v, "description"),
            url: s(v, "html_url"),
            fork: v.get("fork").and_then(|x| x.as_bool()).unwrap_or(false),
            pushed_at: s(v, "pushed_at"),
            name,
            default_branch,
            brief,
        });
    }
    // Most recently pushed first: where the work is happening leads.
    repos.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at).then(a.name.cmp(&b.name)));
    Ok(Repos { generated_at: now_utc(), org: ORG.to_string(), repos })
}

/// The numbered section titles of the shared rules ("1. The loop", ...).
pub fn rule_headings(md: &str) -> Vec<&str> {
    md.lines()
        .filter_map(|l| l.strip_prefix("## "))
        .filter(|h| h.starts_with(|c: char| c.is_ascii_digit()))
        .collect()
}

/// The prompt a visitor pastes into their agent for one repository.
pub fn prompt(repo_url: &str) -> String {
    format!(
        "Read {RULES_URL} and then the AGENTS.md of {repo_url}, and follow both. Pick an open issue there, comment to claim it, work on a branch, and open a pull request that leads with the evidence: the commands you ran and their output."
    )
}

fn repo_card(r: &Repo) -> Markup {
    html! {
        li class="hub-card agents-card" id=(format!("repo-{}", r.name)) {
            a href=(r.url) target="_blank" rel="noopener noreferrer" { (r.name) }
            @if r.fork {
                p class="agents-tag" { "Fork" }
            }
            @if !r.description.is_empty() {
                p { (r.description) }
            }
            @match r.brief_url() {
                Some(href) => {
                    (foot_labelled(&href, &format!("{} on GitHub", r.brief.as_deref().unwrap_or("AGENTS.md")), Target::Evidence))
                }
                None => {
                    p class="agents-none" { "No AGENTS.md yet: the shared rules apply, then the README." }
                    (foot_labelled(&r.url, "README on GitHub", Target::Source))
                }
            }
        }
    }
}

pub fn render_agents_page(repos: &Repos) -> Markup {
    let listed = repos.listed();
    let with_brief = listed.iter().filter(|r| r.brief.is_some()).count();
    html! {
        div class="container hub-wrap" {
            p class="hub-kicker" { "For agents" }
            h1 class="hub-title" { "Point your agent at any AIEN repository" }
            p class="hub-standfirst" {
                "Every public AIEN repository is open to your own coding agent, working under the same rules ours follow. Give it two files: the shared rules, and the AGENTS.md of the repository you want it to work on. Your own subscription pays for the work (Claude, Codex, Gemini, Grok or any other agent), and nothing of yours is sent to the project."
            }

            section class="hub-section" id="start" aria-labelledby="agents-start" {
                h2 id="agents-start" class="hub-h2" { "Start an agent" }
                ol class="agents-steps" {
                    li { "Pick a repository below and open its AGENTS.md: it says what the repository is, how to build and test it, and what needs approval." }
                    li { "Paste this into your agent, with the repository's address in place of the example:" }
                }
                pre class="agents-prompt" { code { (prompt("https://github.com/aien-dev/omega-mv")) } }
                p class="hub-note" {
                    "Your agent works through ordinary pull requests. It needs no access to the project's machines, and only reviewed, merged work reaches the record on "
                    a href="/now/" { "Now" }
                    "."
                }
            }

            section class="hub-section" id="rules" aria-labelledby="agents-rules" {
                h2 id="agents-rules" class="hub-h2" { "The shared rules" }
                p {
                    "These are the working rules of every AIEN agent, published word for word. Our own agents also read a few rules that only work on the project's own machine (its local services, hardware reservations and how to report to the operator). Those are left out because an outside agent cannot follow them."
                }
                ul class="hub-cards" {
                    li class="hub-card agents-card" {
                        a href=(RULES_PATH) { "RULES.md" }
                        ol class="agents-rule-list" {
                            @for h in rule_headings(RULES) {
                                li { (h.split_once(". ").map(|(_, t)| t).unwrap_or(h)) }
                            }
                        }
                        (foot_labelled(RULES_SOURCE, "Source on GitHub", Target::Source))
                    }
                }
            }

            section class="hub-section" id="repositories" aria-labelledby="agents-repos" {
                h2 id="agents-repos" class="hub-h2" { "Every repository" }
                p class="hub-note" {
                    (listed.len()) " public repositories, " (with_brief) " with their own instructions, most recently active first. Read from GitHub at " (repos.generated_at) "; refreshed hourly. The same list as data: "
                    a href="/data/agent-repos.json" { "/data/agent-repos.json" }
                    "."
                }
                ul class="hub-cards" {
                    @for r in &listed {
                        (repo_card(r))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str, brief: Option<&str>, pushed: &str) -> Repo {
        Repo {
            name: name.into(),
            description: format!("{name} does a thing"),
            url: format!("https://github.com/aien-dev/{name}"),
            default_branch: "main".into(),
            fork: false,
            pushed_at: pushed.into(),
            brief: brief.map(String::from),
        }
    }

    fn sample() -> Repos {
        Repos {
            generated_at: "2026-10-25T19:00:00Z".into(),
            org: ORG.into(),
            repos: vec![
                repo("omega-mv", Some("AGENTS.md"), "2026-10-25T18:00:00Z"),
                repo("translate", Some("CLAUDE.md"), "2026-10-24T18:00:00Z"),
                repo("aien-edge", None, "2026-10-22T18:00:00Z"),
                repo("aienix", Some("AGENTS.md"), "2026-10-23T18:00:00Z"),
            ],
        }
    }

    #[test]
    fn every_repository_links_its_own_brief_or_says_it_has_none() {
        let page = render_agents_page(&sample()).into_string();
        assert!(page.contains("href=\"https://github.com/aien-dev/omega-mv/blob/main/AGENTS.md\""));
        assert!(page.contains("href=\"https://github.com/aien-dev/translate/blob/main/CLAUDE.md\""), "a CLAUDE.md is linked under its own name");
        assert!(page.contains(">CLAUDE.md on GitHub"));
        assert!(!page.contains("aien-edge/blob/main/AGENTS.md"), "a brief that does not exist is never linked");
        assert!(page.contains("No AGENTS.md yet"));
        assert_eq!(crate::components::boxlink::check_boxes(&page), Ok(4), "the rules box and three listed repositories, each with a GitHub link");
    }

    #[test]
    fn retired_name_is_not_rendered() {
        let page = render_agents_page(&sample()).into_string();
        assert!(!page.to_ascii_lowercase().contains("aienix"));
        assert!(page.contains("3 public repositories, 2 with their own instructions"));
    }

    #[test]
    fn rules_headings_come_from_the_published_file() {
        let h = rule_headings(RULES);
        assert!(h.len() >= 5, "RULES.md lost its numbered sections: {h:?}");
        let page = render_agents_page(&sample()).into_string();
        for title in h {
            let t = title.split_once(". ").map(|(_, t)| t).unwrap_or(title);
            assert!(page.contains(&maud::html! { (t) }.into_string()), "rules box is missing {t}");
        }
    }

    #[test]
    fn rules_hold_the_public_invariants() {
        assert!(!RULES.contains('\u{2014}') && !RULES.contains('\u{2013}'), "no em or en dash");
        for private in ["127.0.0.1", "/home/", "~/", "18091", "100.64.", "atlas-vault", "CHIP-RESERVED"] {
            assert!(!RULES.contains(private), "RULES.md leaks a machine-only detail: {private}");
        }
        assert!(RULES.contains("https://www.aienos.com/agents/"), "RULES.md must point back at the repository list");
    }

    #[test]
    fn prompt_names_both_files() {
        let p = prompt("https://github.com/aien-dev/x");
        assert!(p.contains(RULES_URL) && p.contains("AGENTS.md of https://github.com/aien-dev/x"));
    }
}
