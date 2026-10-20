//! /try/: run AienOS on a computer you already own, one step per system.
//!
//! Three boxes (Windows, macOS, Linux), each with the command, the sha256 of
//! the program, what happens next, what AienOS is not yet, and how to remove
//! it. Two sources feed it and nothing is typed by hand:
//!
//! * `public/data/try-release.json`: the latest release of aien-dev/aienos as
//!   the collector (`site-builder try-release`) read it. Until a release with
//!   an `aienos-run-SHA256SUMS` file exists the page says "not published yet"
//!   with the date it last looked, and offers no download link.
//! * `data/try.json`: the measured numbers. A null renders as NOT MEASURED; a
//!   number must carry a GitHub receipt (checked by the tests).
//!
//! The script behind the macOS and Linux command is `public/try.sh`, served at
//! /try.sh. It cannot be served at /try because this page is a folder there.

use std::fs;
use std::path::Path;
use std::process::Command;

use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::components::boxlink::{foot, source_href, Target};

pub const TITLE: &str = "Try AienOS | AIENOS";
pub const DESCRIPTION: &str = "Run AienOS in a window on Windows, macOS or Linux: one program, its sha256 beside it, what it does, what it does not do yet, and how to remove it.";
pub const URL: &str = "https://www.aienos.com/try/";

pub const RELEASE_PATH: &str = "public/data/try-release.json";
pub const RELEASE_REPO: &str = "aien-dev/aienos";
pub const SUMS_ASSET: &str = "aienos-run-SHA256SUMS";
pub const SCRIPT_URL: &str = "https://www.aienos.com/try.sh";
pub const COMMAND: &str = "curl -fsSL https://www.aienos.com/try.sh | sh";

const RAW: &str = include_str!("../../data/try.json");
const THIS_FILE: &str = "crates/site-builder/src/components/try_page.rs";
const DATA_FILE: &str = "crates/site-builder/data/try.json";
const SCRIPT_FILE: &str = "public/try.sh";

/// The phrase the build requires on the page.
pub const NOT_PUBLISHED: &str = "Not published yet";

// ------------------------------------------------------------------ data

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Artifact {
    pub asset: String,
    pub sha256: String,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Release {
    pub checked_utc: String,
    pub published: bool,
    pub repo: String,
    pub tag: Option<String>,
    pub published_at: Option<String>,
    pub release_url: Option<String>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
struct Measured {
    id: String,
    target: String,
    what: String,
    value: Option<String>,
    receipt: Option<String>,
}

#[derive(Deserialize)]
struct TryData {
    as_of: String,
    measured: Vec<Measured>,
}

fn data() -> TryData {
    serde_json::from_str(RAW).expect("data/try.json parses")
}

pub fn load() -> Release {
    let raw = fs::read_to_string(Path::new(RELEASE_PATH)).unwrap_or_else(|e| panic!("cannot read {RELEASE_PATH}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{RELEASE_PATH} does not parse: {e}"))
}

/// One runner: the file name a release publishes for it.
pub struct Runner {
    pub asset: &'static str,
    pub label: &'static str,
}

pub const WINDOWS: [Runner; 1] = [Runner { asset: "aienos-run-windows-x86_64.exe", label: "Windows 11, 64-bit (x86_64)" }];
pub const MACOS: [Runner; 1] = [Runner { asset: "aienos-run-macos-aarch64", label: "macOS on Apple Silicon (aarch64)" }];
pub const LINUX: [Runner; 2] = [
    Runner { asset: "aienos-run-linux-x86_64", label: "Linux x86_64" },
    Runner { asset: "aienos-run-linux-aarch64", label: "Linux aarch64" },
];

// ------------------------------------------------------------------ collector

/// Parse an `aienos-run-SHA256SUMS` file: `<64 hex>  <name>` per line (the
/// sha256sum format; `*name` marks binary mode). Any other line is an error,
/// so a malformed manifest is never half-read.
pub fn parse_sums(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (hash, name) = line.split_once(char::is_whitespace).ok_or_else(|| format!("line {}: no file name", n + 1))?;
        let name = name.trim().trim_start_matches('*');
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(format!("line {}: not a lowercase sha256: {hash:?}", n + 1));
        }
        if name.is_empty() || name.contains('/') {
            return Err(format!("line {}: bad file name {name:?}", n + 1));
        }
        out.push((hash.to_string(), name.to_string()));
    }
    Ok(out)
}

/// Build the release record from the GitHub release JSON and its sums file.
/// Only assets named in the sums file AND present on the release count, so a
/// hash never appears for a file that cannot be downloaded.
pub fn release_from(api: &serde_json::Value, sums: &str, checked_utc: &str) -> Result<Release, String> {
    let sums = parse_sums(sums)?;
    let assets = api["assets"].as_array().ok_or("release has no assets list")?;
    let mut artifacts = Vec::new();
    for (hash, name) in sums {
        let Some(a) = assets.iter().find(|a| a["name"].as_str() == Some(name.as_str())) else { continue };
        artifacts.push(Artifact {
            asset: name,
            sha256: hash,
            size: a["size"].as_u64(),
            url: a["browser_download_url"].as_str().ok_or("asset without a download url")?.to_string(),
        });
    }
    Ok(Release {
        checked_utc: checked_utc.to_string(),
        published: !artifacts.is_empty(),
        repo: RELEASE_REPO.to_string(),
        tag: api["tag_name"].as_str().map(str::to_string),
        published_at: api["published_at"].as_str().map(str::to_string),
        release_url: api["html_url"].as_str().map(str::to_string),
        artifacts,
    })
}

pub fn unpublished(checked_utc: &str) -> Release {
    Release { checked_utc: checked_utc.to_string(), published: false, repo: RELEASE_REPO.to_string(), ..Default::default() }
}

fn gh_api(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("gh").arg("api").args(args).output().map_err(|e| format!("cannot run gh: {e}"))?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    Err(format!("gh api {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
}

/// The newest release that is not a draft and carries the runner list. Other
/// releases of the repository (the PC disk images, for one) do not count, which
/// is why this does not use GitHub's "latest release".
pub fn pick_release(releases: &serde_json::Value) -> Option<&serde_json::Value> {
    releases.as_array()?.iter().find(|r| {
        r["draft"].as_bool() != Some(true)
            && r["assets"].as_array().map(|a| a.iter().any(|a| a["name"].as_str() == Some(SUMS_ASSET))).unwrap_or(false)
    })
}

/// Read the releases of aien-dev/aienos and write the record. No release with
/// a runner list is a normal answer and writes the unpublished record; a failed
/// read is an error so the last good file stays in place.
pub fn collect(out: &Path) -> Result<Release, String> {
    let now = crate::facts::now_utc();
    let body = gh_api(&[&format!("repos/{RELEASE_REPO}/releases?per_page=30")])?;
    let list: serde_json::Value = serde_json::from_slice(&body).map_err(|e| format!("release list JSON: {e}"))?;
    let release = match pick_release(&list) {
        None => unpublished(&now),
        Some(api) => {
            let sums_asset = api["assets"].as_array().and_then(|a| a.iter().find(|a| a["name"].as_str() == Some(SUMS_ASSET))).ok_or("sums asset vanished")?;
            let id = sums_asset["id"].as_u64().ok_or("sums asset without an id")?;
            let text = gh_api(&["-H", "Accept: application/octet-stream", &format!("repos/{RELEASE_REPO}/releases/assets/{id}")])
                ?;
            release_from(api, &String::from_utf8_lossy(&text), &now)?
        }
    };
    let json = serde_json::to_string_pretty(&release).map_err(|e| e.to_string())? + "\n";
    fs::write(out, json).map_err(|e| format!("write {}: {e}", out.display()))?;
    Ok(release)
}

// ------------------------------------------------------------------ rendering

fn day(utc: &str) -> &str {
    utc.get(..10).unwrap_or(utc)
}

fn artifact<'a>(r: &'a Release, asset: &str) -> Option<&'a Artifact> {
    r.artifacts.iter().find(|a| a.asset == asset)
}

fn size_text(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
}

/// The sha256 line for one runner, exactly as `sha256sum` prints it, or the
/// plain statement that there is none yet.
fn sha_line(r: &Release, t: &Runner) -> Markup {
    match artifact(r, t.asset) {
        Some(a) => html! {
            p class="try-sha-label" { (t.label) }
            pre class="try-code try-sha" { code { (a.sha256) "  " (a.asset) } }
        },
        None => html! {
            p class="try-sha-label" { (t.label) }
            p class="try-nm" { "sha256: " (NOT_PUBLISHED) " (checked " (day(&r.checked_utc)) ")" }
        },
    }
}

fn status_chip(r: &Release, targets: &[Runner]) -> Markup {
    let got: Vec<&Artifact> = targets.iter().filter_map(|t| artifact(r, t.asset)).collect();
    match (got.is_empty(), r.tag.as_deref(), r.published_at.as_deref()) {
        (false, Some(tag), Some(at)) => html! { span class="try-chip try-chip-on" { "Published: " (tag) ", " (day(at)) } },
        (false, _, _) => html! { span class="try-chip try-chip-on" { "Published" } },
        (true, _, _) => html! { span class="try-chip try-chip-off" { (NOT_PUBLISHED) ", checked " (day(&r.checked_utc)) } },
    }
}

fn measured_line(d: &TryData, target: &str) -> Markup {
    let m = d.measured.iter().find(|m| m.target == target);
    html! {
        p class="try-measured" data-id=(m.map(|m| m.id.as_str()).unwrap_or(target)) {
            span class="try-measured-k" { "Measured on this system (as of " (d.as_of) "): " }
            @match m {
                Some(Measured { value: Some(v), receipt: Some(link), what, .. }) => {
                    (what) ": " strong { (v) } " " a href=(link) target="_blank" rel="noopener noreferrer" { "(receipt)" }
                }
                Some(m) => { span class="try-nm" { (m.what) ": NOT MEASURED" } }
                None => { span class="try-nm" { "NOT MEASURED" } }
            }
        }
    }
}

fn steps(items: &[&str]) -> Markup {
    html! { ol class="try-list" { @for i in items { li { (i) } } } }
}

fn bullets(items: &[&str]) -> Markup {
    html! { ul class="try-list" { @for i in items { li { (i) } } } }
}

const NEXT_COMMON: [&str; 3] = [
    "The runner checks its own cache for the AienOS image. If the image is missing it downloads it and checks the sha256 against the image manifest; a mismatch stops it.",
    "It starts a virtual machine, a computer simulated inside a program, and boots AienOS in it.",
    "An AienOS console opens. You type into it; closing it shuts the machine down.",
];

const NOT_YET_COMMON: [&str; 4] = [
    "No GPU. The virtual machine is given no graphics card, so no GPU work runs in it.",
    "No language model is published in the image. Whether the image carries a small CPU model: NOT MEASURED.",
    "It is not yet joined to Interplane from inside the virtual machine: not built yet.",
    "Boot time on your machine is not measured here; the line above stays NOT MEASURED until a receipt exists.",
];

fn foot_for(r: &Release) -> Markup {
    match &r.release_url {
        Some(url) if r.published && url.starts_with("https://github.com/") => foot(url, Target::Evidence),
        _ => foot(&source_href(DATA_FILE), Target::Source),
    }
}

fn windows_box(r: &Release, d: &TryData) -> Markup {
    let t = &WINDOWS[0];
    let a = artifact(r, t.asset);
    html! {
        article class="try-card" id="try-windows" {
            div class="try-card-head" { h2 class="try-os" { "Windows" } (status_chip(r, &WINDOWS)) }
            p class="try-lead" { "Download one file and run it. No installer, no WSL." }
            h3 class="try-h3" { "Get it" }
            @if let Some(a) = a {
                p { a class="try-button" href=(a.url) { "Download " (a.asset) } @if let Some(s) = a.size { " " span class="try-size" { (size_text(s)) } } }
            } @else {
                p class="try-nm" { (t.asset) ": " (NOT_PUBLISHED) " (checked " (day(&r.checked_utc)) "). There is no download link until it exists." }
            }
            h3 class="try-h3" { "sha256" }
            (sha_line(r, t))
            p class="try-small" { "Check it before you run it, in PowerShell: " code { "Get-FileHash .\\" (t.asset) " -Algorithm SHA256" } }
            (measured_line(d, "windows"))
            h3 class="try-h3" { "What happens next" }
            (steps(&[
                "Windows may warn that the file is from an unknown publisher: it is not code-signed yet. You decide whether to continue.",
                NEXT_COMMON[0], NEXT_COMMON[1], NEXT_COMMON[2],
            ]))
            h3 class="try-h3" { "What it is not yet" }
            (bullets(&[
                NOT_YET_COMMON[0], NOT_YET_COMMON[1],
                "It needs the Windows Virtual Machine Platform feature switched on, which takes one reboot the first time. The Windows run has not been run on a Windows computer yet: NOT MEASURED.",
                NOT_YET_COMMON[2], NOT_YET_COMMON[3],
            ]))
            h3 class="try-h3" { "Remove it" }
            (bullets(&[
                "Start fresh but keep the program: run it with --reset. That deletes the virtual disk.",
                "Remove everything: delete the folder %LOCALAPPDATA%\\AienOS, then delete the .exe. The runner keeps its files in that folder, by design.",
            ]))
            (foot_for(r))
        }
    }
}

fn unix_box(r: &Release, d: &TryData, id: &str, name: &str, lead: &str, targets: &[Runner], cache: &str, not_yet: &[&str]) -> Markup {
    html! {
        article class="try-card" id=(format!("try-{id}")) {
            div class="try-card-head" { h2 class="try-os" { (name) } (status_chip(r, targets)) }
            p class="try-lead" { (lead) }
            h3 class="try-h3" { "Run this in a terminal" }
            pre class="try-code" { code { (COMMAND) } }
            p class="try-small" { "The script picks your system, downloads the matching runner, checks its sha256 and refuses on a mismatch. " a href="/try.sh" { "Read it first" } ": it is one short file." }
            h3 class="try-h3" { "sha256" }
            @for t in targets { (sha_line(r, t)) }
            p class="try-small" { "By hand: " code { "sha256sum <file>" } " on Linux, " code { "shasum -a 256 <file>" } " on macOS." }
            (measured_line(d, id))
            h3 class="try-h3" { "What happens next" }
            (steps(&[
                "The script reads the runner list of the newest release that carries one, and your line in it. If nothing is published it says so and stops; nothing is installed.",
                NEXT_COMMON[0], NEXT_COMMON[1], NEXT_COMMON[2],
            ]))
            h3 class="try-h3" { "What it is not yet" }
            (bullets(not_yet))
            h3 class="try-h3" { "Remove it" }
            (bullets(&[
                "Start fresh: run the command again with --reset on the end (sh -s -- --reset). That deletes the virtual disk.",
                cache,
            ]))
            (foot_for(r))
        }
    }
}

pub fn render_try_page(r: &Release) -> Markup {
    let d = data();
    let macos_not_yet = [
        NOT_YET_COMMON[0], NOT_YET_COMMON[1],
        "The runner is not code-signed or notarized by Apple yet. Intel Macs are not supported: there is no runner for them.",
        NOT_YET_COMMON[2], NOT_YET_COMMON[3],
    ];
    let linux_not_yet = [
        NOT_YET_COMMON[0], NOT_YET_COMMON[1],
        "The runner uses KVM, the virtual machine support in the Linux kernel. If /dev/kvm is missing or you cannot open it, it cannot start.",
        NOT_YET_COMMON[2], NOT_YET_COMMON[3],
    ];
    html! {
        div class="container try-wrap" {
            p class="hub-kicker" { "Try AienOS" }
            h1 class="try-title" { "Try AienOS in a window on your own computer." }
            p class="try-standfirst" {
                "AienOS is the operating system on the AIEN machines. aienos-run is one program that starts it inside a virtual machine on Windows, macOS or Linux, so you can look at it without giving up your computer. Everything here is either measured, with a receipt, or marked NOT MEASURED."
            }
            p class="try-standfirst try-standfirst-2" {
                "Last looked at the release page: " (day(&r.checked_utc)) ". "
                @if r.published { "A release exists." } @else { "No release exists yet, so there is no download below. This page reads the release every hour and fills in the files and their sha256 values when they appear." }
            }
            section class="try-cards" aria-label="One box per system" {
                (windows_box(r, &d))
                (unix_box(r, &d, "macos", "macOS", "One command in Terminal. Apple Silicon only.", &MACOS,
                    "Remove everything: delete the folder ~/Library/Caches/AienOS. The runner keeps its files in that folder, by design.", &macos_not_yet))
                (unix_box(r, &d, "linux", "Linux", "One command in a terminal. x86_64 or aarch64.", &LINUX,
                    "Remove everything: delete the folder ~/.cache/aienos (or $XDG_CACHE_HOME/aienos if you set it). The runner keeps its files in that folder, by design.", &linux_not_yet))
            }
            section class="try-more" aria-labelledby="try-check" {
                h2 id="try-check" class="hub-h2" { "Check it yourself" }
                ul class="try-list" {
                    li { "The script is " a href="/try.sh" { "/try.sh" } ". Read it with " code { "curl -fsSL " (SCRIPT_URL) " | less" } " before you pipe it to a shell. Its tests are in " a href=(source_href("scripts/test-try.sh")) target="_blank" rel="noopener noreferrer" { "scripts/test-try.sh" } " and include a tampered download that must be refused." }
                    li { "The release files and their sha256 values live at " a href="https://github.com/aien-dev/aienos/releases" target="_blank" rel="noopener noreferrer" { "github.com/aien-dev/aienos/releases" } ". The sha256 values on this page are copied from that release by the build, not typed." }
                    li { "The same data for programs: " a href="/data/try-release.json" { "/data/try-release.json" } "." }
                    li { "Not yet: the runner list is not signed. A checksum protects against a damaged download, not against someone replacing both the file and the list on GitHub." }
                }
                p class="try-small" { "Source of this page: " a href=(source_href(THIS_FILE)) target="_blank" rel="noopener noreferrer" { "try_page.rs" } ", " a href=(source_href(SCRIPT_FILE)) target="_blank" rel="noopener noreferrer" { "try.sh" } "." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::boxlink::check_boxes;

    const HASH_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const HASH_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    fn fake_api() -> serde_json::Value {
        serde_json::json!({
            "tag_name": "aienos-run-v0.1.0",
            "published_at": "2026-11-02T10:00:00Z",
            "html_url": "https://github.com/aien-dev/aienos/releases/tag/aienos-run-v0.1.0",
            "assets": [
                { "name": "aienos-run-linux-x86_64", "size": 1048576, "browser_download_url": "https://github.com/aien-dev/aienos/releases/download/aienos-run-v0.1.0/aienos-run-linux-x86_64" },
                { "name": "aienos-run-SHA256SUMS", "size": 200, "browser_download_url": "https://github.com/aien-dev/aienos/releases/download/aienos-run-v0.1.0/aienos-run-SHA256SUMS" }
            ]
        })
    }

    fn published() -> Release {
        let sums = format!("{HASH_A}  aienos-run-linux-x86_64\n{HASH_B}  aienos-run-windows-x86_64.exe\n");
        release_from(&fake_api(), &sums, "2026-11-02T11:00:00Z").unwrap()
    }

    #[test]
    fn committed_release_file_parses_and_says_not_published() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(RELEASE_PATH);
        let r: Release = serde_json::from_str(&fs::read_to_string(path).expect("read committed release file")).expect("parses");
        assert!(!r.published && r.artifacts.is_empty(), "the committed default must be the unpublished record");
        assert!(r.checked_utc.len() >= 10);
    }

    #[test]
    fn sums_parse_and_reject_garbage() {
        let ok = parse_sums(&format!("{HASH_A}  a\n\n{HASH_B} *b\n")).unwrap();
        assert_eq!(ok, vec![(HASH_A.into(), "a".into()), (HASH_B.into(), "b".into())]);
        assert!(parse_sums("nothex  a").is_err());
        assert!(parse_sums(&format!("{}  a", HASH_A.to_uppercase())).is_err(), "uppercase is not the sha256sum format");
        assert!(parse_sums(&format!("{HASH_A}")).is_err());
        assert!(parse_sums(&format!("{HASH_A}  ../x")).is_err());
        assert_eq!(parse_sums("").unwrap(), vec![]);
    }

    #[test]
    fn release_lists_only_files_that_can_be_downloaded() {
        let r = published();
        assert!(r.published);
        assert_eq!(r.artifacts.len(), 1, "the windows line has no asset on the release, so no hash is shown for it");
        assert_eq!(r.artifacts[0].asset, "aienos-run-linux-x86_64");
        assert_eq!(r.tag.as_deref(), Some("aienos-run-v0.1.0"));
        let none = release_from(&fake_api(), "", "2026-11-02T11:00:00Z").unwrap();
        assert!(!none.published, "a release with an empty runner list is not a publication");
    }

    #[test]
    fn unpublished_page_says_so_with_the_date_and_has_no_download_link() {
        let page = render_try_page(&unpublished("2026-10-20T09:00:00Z")).into_string();
        assert!(page.contains(NOT_PUBLISHED));
        assert!(page.contains("checked 2026-10-20"));
        assert!(!page.contains("try-button"), "no download button without a release");
        assert!(!page.contains("releases/download"), "no asset link without a release");
        assert_eq!(page.matches("NOT MEASURED").count() >= 3, true);
        assert!(page.contains(COMMAND));
    }

    #[test]
    fn published_page_shows_the_sha256_line_and_the_download() {
        let page = render_try_page(&published()).into_string();
        assert!(page.contains(&format!("{HASH_A}  aienos-run-linux-x86_64")), "the sha256 line is the sha256sum format");
        assert!(page.contains("releases/download/aienos-run-v0.1.0/aienos-run-linux-x86_64") || page.contains(HASH_A));
        assert!(page.contains("Published: aienos-run-v0.1.0, 2026-11-02"));
        assert!(!page.contains(HASH_B), "no hash is shown for a file that is not on the release");
        assert!(page.contains("aienos-run-windows-x86_64.exe: Not published yet"), "windows has no asset, so it says so");
        assert!(page.contains("https://github.com/aien-dev/aienos/releases/tag/aienos-run-v0.1.0"));
    }

    #[test]
    fn every_box_links_to_github_in_both_states() {
        for r in [unpublished("2026-10-20T00:00:00Z"), published()] {
            let page = render_try_page(&r).into_string();
            assert_eq!(check_boxes(&page), Ok(3), "three system boxes");
        }
    }

    #[test]
    fn page_names_each_system_and_the_three_removal_paths() {
        let page = render_try_page(&unpublished("2026-10-20T00:00:00Z")).into_string();
        for s in ["id=\"try-windows\"", "id=\"try-macos\"", "id=\"try-linux\"", "%LOCALAPPDATA%\\AienOS", "~/Library/Caches/AienOS", "~/.cache/aienos", "--reset"] {
            assert!(page.contains(s), "missing {s}");
        }
        assert_eq!(page.matches("What it is not yet").count(), 3);
        assert_eq!(page.matches("Remove it").count(), 3);
        assert_eq!(page.matches("sha256").count() >= 6, true);
    }

    #[test]
    fn measured_numbers_need_a_github_receipt() {
        let d = data();
        for m in &d.measured {
            match (&m.value, &m.receipt) {
                (None, None) => {}
                (Some(_), Some(link)) => assert!(link.starts_with("https://github.com/"), "{}: receipt must be on GitHub", m.id),
                _ => panic!("{}: a value needs a receipt and a receipt needs a value", m.id),
            }
        }
        assert_eq!(d.measured.len(), 3);
        assert_eq!(d.as_of.len(), 10);
    }

    #[test]
    fn a_measured_number_renders_with_its_receipt() {
        let d = TryData {
            as_of: "2026-11-02".into(),
            measured: vec![Measured { id: "boot-linux".into(), target: "linux".into(), what: "Boot".into(), value: Some("3.2 s".into()), receipt: Some("https://github.com/aien-dev/aienos/pull/1".into()) }],
        };
        let s = measured_line(&d, "linux").into_string();
        assert!(s.contains("3.2 s") && s.contains("https://github.com/aien-dev/aienos/pull/1"));
        assert!(measured_line(&d, "macos").into_string().contains("NOT MEASURED"));
    }

    #[test]
    fn no_dashes_no_comparison_no_marketing() {
        for r in [unpublished("2026-10-20T00:00:00Z"), published()] {
            let page = render_try_page(&r).into_string();
            assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'));
            let lower = page.to_ascii_lowercase();
            for w in crate::components::machines::FORBIDDEN {
                assert!(!lower.contains(w), "comparison with another system: {w:?}");
            }
            for w in ["seamless", "revolutionary", "blazing", "effortless", "powerful", "cutting-edge", "next-generation", "world-class", "best-in-class"] {
                assert!(!lower.contains(w), "marketing adjective: {w}");
            }
        }
    }

    #[test]
    fn try_sh_passes_its_shell_tests() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for shell in ["sh", "dash"] {
            let Ok(out) = Command::new(shell).arg(root.join("scripts/test-try.sh")).arg(root.join("public/try.sh")).output() else { continue };
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(out.status.success() && text.contains("ALL PASS"), "{shell}: {text}\n{}", String::from_utf8_lossy(&out.stderr));
        }
    }

    #[test]
    fn try_sh_has_no_dashes_and_runs_only_in_main() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let s = fs::read_to_string(root.join("public/try.sh")).unwrap();
        assert!(!s.contains('\u{2014}') && !s.contains('\u{2013}'));
        assert!(s.trim_end().ends_with("main \"$@\""), "the last line must be the one call, so a cut-short download runs nothing");
        assert_eq!(s.matches("main \"$@\"").count(), 1);
    }

    #[test]
    fn the_release_picker_skips_image_only_and_draft_releases() {
        let list = serde_json::json!([
            { "tag_name": "aienos-pc-v0.1.0", "assets": [{ "name": "SHA256SUMS" }] },
            { "tag_name": "draft-run", "draft": true, "assets": [{ "name": "aienos-run-SHA256SUMS" }] },
            { "tag_name": "aienos-run-v0.2.0", "assets": [{ "name": "aienos-run-SHA256SUMS" }] },
            { "tag_name": "aienos-run-v0.1.0", "assets": [{ "name": "aienos-run-SHA256SUMS" }] }
        ]);
        assert_eq!(pick_release(&list).unwrap()["tag_name"], "aienos-run-v0.2.0", "newest non-draft release with the runner list");
        assert!(pick_release(&serde_json::json!([{ "assets": [{ "name": "SHA256SUMS" }] }])).is_none());
        assert!(pick_release(&serde_json::json!([])).is_none());
        assert!(pick_release(&serde_json::json!({ "message": "Not Found" })).is_none());
    }
}
