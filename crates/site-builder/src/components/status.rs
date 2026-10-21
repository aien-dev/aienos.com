use maud::{html, Markup};

use crate::facts::Facts;

use super::boxlink::{foot, source_href, Target};
use super::research_status as rs;

/// The file that holds a layer's words, for a layer with no receipt yet.
const THIS_FILE: &str = "crates/site-builder/src/components/status.rs";

/// The canonical public state of the project. This list is the single
/// source of truth: the homepage status section, /status.json, and
/// /llms.txt all render from it. Statuses are a small fixed set:
/// IMPLEMENTED, QUALIFIED, EXPERIMENTAL, BLOCKED, PLANNED.
/// Bump UPDATED when a state changes; states change when the evidence
/// changes, not when the copy does.
pub const UPDATED: &str = "2026-10-01";

pub struct LayerState {
    pub name: &'static str,
    pub status: &'static str,
    pub note: &'static str,
    pub receipt_label: Option<&'static str>,
    pub receipt_url: Option<&'static str>,
}

pub fn layer_states() -> Vec<LayerState> {
    vec![
        LayerState {
            name: "Omega foundation gates",
            status: "QUALIFIED",
            note: "Gates 1 through 14 closed with one combined admission receipt: 230 checks, 0 failures.",
            receipt_label: Some("aien-dev/omega · PR #111"),
            receipt_url: Some("https://github.com/aien-dev/omega/pull/111"),
        },
        LayerState {
            name: "Turing instrument",
            status: rs::turing_instrument_status(),
            note: rs::turing_instrument_note(),
            receipt_label: Some("aien-dev/omega · PR #88"),
            receipt_url: Some("https://github.com/aien-dev/omega/pull/88"),
        },
        LayerState {
            name: "Native C kernel",
            status: "IMPLEMENTED",
            note: "Freestanding C kernel on main. Boots via UEFI in emulation, with NVMe, ARGUS capability checks, and the sealed Store in the boot path.",
            receipt_label: Some("aien-dev/aienos · PR #194"),
            receipt_url: Some("https://github.com/aien-dev/aienos/pull/194"),
        },
        LayerState {
            name: "Native machine qualification (M5)",
            status: "BLOCKED",
            note: "First qualification receipt: NOT QUALIFIED, 10 blocked, 6 missing. Published, and being worked down in public.",
            receipt_label: Some("aien-dev/aienos · PR #187"),
            receipt_url: Some("https://github.com/aien-dev/aienos/pull/187"),
        },
        LayerState {
            name: "Turing experiments (EXP-001 / 002 / 003)",
            status: "EXPERIMENTAL",
            note: rs::experiments_note(),
            receipt_label: Some("aien-dev/omega · PR #119"),
            receipt_url: Some("https://github.com/aien-dev/omega/pull/119"),
        },
        LayerState {
            name: "AIEN, the persistent agent",
            status: "EXPERIMENTAL",
            note: "Runs in the lab on the DGX Spark. The learning loop, learning from verified experience, is the active build.",
            receipt_label: Some("aien-dev/aienos"),
            receipt_url: Some("https://github.com/aien-dev/aienos"),
        },
        LayerState {
            name: "Atlas, the awakening layer",
            status: "PLANNED",
            note: "The seed that wakes AIENOS on new hardware. Doctrine published in the architecture repository; implementation ahead.",
            receipt_label: Some("aien-dev/aien-architecture"),
            receipt_url: Some("https://github.com/aien-dev/aien-architecture"),
        },
        LayerState {
            name: "Public release",
            status: "PLANNED",
            note: "Pre-alpha. No supported release or installer yet. When there is one, it ships with versions, hashes, and signatures, inspectable before it runs.",
            receipt_label: None,
            receipt_url: None,
        },
    ]
}

fn chip_style(status: &str) -> &'static str {
    match status {
        "QUALIFIED" => "border: 1px solid rgba(0, 229, 153, 0.4); color: var(--accent-green); background: rgba(0, 229, 153, 0.08);",
        "IMPLEMENTED" => "border: 1px solid rgba(45, 127, 249, 0.4); color: var(--accent-blue); background: rgba(45, 127, 249, 0.08);",
        "EXPERIMENTAL" => "border: 1px solid rgba(245, 158, 11, 0.4); color: #f5a623; background: rgba(245, 158, 11, 0.08);",
        "BLOCKED" => "border: 1px solid rgba(248, 113, 113, 0.4); color: #f87171; background: rgba(248, 113, 113, 0.08);",
        _ => "border: 1px solid var(--border-subtle); color: var(--text-muted); background: var(--bg-surface);",
    }
}

pub fn render_status(facts: &Facts) -> Markup {
    let layers = layer_states();

    html! {
        section id="status" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="max-width: 760px; margin-bottom: 36px;" {
                    div class="badge badge-amber" style="margin-bottom: 12px;" {
                        "CANONICAL PROJECT STATE"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Where it honestly stands"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; line-height: 1.6;" {
                        "One state of the project, published once and consumed everywhere. This section, "
                        a href="/status.json" style="color: var(--accent-green); font-weight: 600;" { "status.json" }
                        ", and "
                        a href="/llms.txt" style="color: var(--accent-green); font-weight: 600;" { "llms.txt" }
                        " all render from the same source, so they cannot drift apart. Last updated " (UPDATED) "."
                    }
                }

                div style="display: flex; flex-direction: column;" {
                    @for layer in &layers {
                        div class="status-row" style="display: flex; gap: 20px; padding: 18px 0; border-top: 1px solid var(--border-subtle); align-items: flex-start; flex-wrap: wrap;" {
                            span class="badge" style=(format!("font-size: 10px; min-width: 118px; justify-content: center; {}", chip_style(layer.status))) {
                                (layer.status)
                            }
                            div style="flex: 1; min-width: 260px;" {
                                div style="font-weight: 700; color: var(--text-primary); font-size: 16px;" {
                                    (super::omega::link_first(layer.name))
                                }
                                p style="color: var(--text-secondary); font-size: 14.5px; line-height: 1.6; margin: 4px 0 0 0;" {
                                    (layer.note)
                                }
                                @if let (Some(label), Some(url)) = (layer.receipt_label, layer.receipt_url) {
                                    div style="margin-top: 6px; font-family: var(--font-mono); font-size: 12px; color: var(--text-muted);" {
                                        "RECEIPT " (label)
                                    }
                                    (foot(url, Target::Evidence))
                                } @else {
                                    div style="margin-top: 6px; font-family: var(--font-mono); font-size: 12px; color: var(--text-muted);" {
                                        "No receipt yet: there is nothing to show until a release exists."
                                    }
                                    (foot(&source_href(THIS_FILE), Target::Source))
                                }
                            }
                        }
                    }
                }

                (super::live::render_live(facts))

                div style="display: flex; gap: 12px; flex-wrap: wrap; margin-top: 32px;" {
                    a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="background: var(--accent-green); color: var(--bg-base); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 700;" {
                        "Follow the work on GitHub"
                    }
                    a href="#research" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                        "Read the research"
                    }
                    a href="#follow" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                        "Follow the work"
                    }
                }
            }
        }
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// The same canonical state as /status.json, for machines, badges,
/// external agents, and eventually AIEN itself.
pub fn status_json() -> String {
    let layers = layer_states();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"project\": \"AIENOS\",\n");
    out.push_str(&format!("  \"updated\": \"{}\",\n", UPDATED));
    out.push_str("  \"canonical\": \"https://www.aienos.com/#status\",\n");
    out.push_str("  \"statuses\": [\"IMPLEMENTED\", \"QUALIFIED\", \"EXPERIMENTAL\", \"BLOCKED\", \"PLANNED\"],\n");
    out.push_str("  \"layers\": [\n");
    for (i, layer) in layers.iter().enumerate() {
        let receipt = match layer.receipt_url {
            Some(url) => format!("\"{}\"", json_escape(url)),
            None => "null".to_string(),
        };
        let comma = if i + 1 < layers.len() { "," } else { "" };
        out.push_str(&format!(
            "    {{\"name\": \"{}\", \"status\": \"{}\", \"note\": \"{}\", \"receipt\": {}}}{}\n",
            json_escape(layer.name),
            layer.status,
            json_escape(layer.note),
            receipt,
            comma
        ));
    }
    out.push_str("  ]\n}\n");
    out
}

/// /llms.txt: the project in plain text for AI readers and crawlers, in
/// the llms.txt convention (see `seo::llms_txt`), with the layer states.
pub fn llms_txt() -> String {
    let lines: Vec<String> = layer_states().iter().map(|l| format!("{}: {}. {}", l.name, l.status, l.note)).collect();
    crate::seo::llms_txt(&lines, &rs::current_level_sentence())
}

#[cfg(test)]
mod wording_tests {
    use super::*;

    const STALE: [&str; 5] = [
        "in preparation",
        "profile holds",
        "yield log",
        "actually understood",
        "first unit written down",
    ];

    fn stale_hit(text: &str) -> Option<&'static str> {
        STALE.iter().copied().find(|s| text.contains(s))
    }

    #[test]
    fn experiments_note_follows_the_status_file() {
        let layers = layer_states();
        let l = layers
            .iter()
            .find(|l| l.name.starts_with("Turing experiments"))
            .expect("experiments layer present");
        for want in ["EXP-001 FAIL", "EXP-001R PASS", "EXP-002D INCOMPLETE", "EXP-003 BLOCKED", "Failures stay on the record"] {
            assert!(l.note.contains(want), "missing: {want}");
        }
    }

    #[test]
    fn every_layer_row_links_to_github() {
        let facts = Facts::default();
        let page = render_status(&facts).into_string();
        let rows = page.matches("class=\"status-row\"").count();
        assert_eq!(rows, layer_states().len());
        // The receipt aside and the live rows render too; count only up to the live block.
        let before_live = page.split("id=\"live\"").next().unwrap();
        assert_eq!(crate::components::boxlink::check_boxes(before_live), Ok(rows), "every layer row links to GitHub");
        assert_eq!(before_live.matches("box-link-source").count(), 1, "only the release row, which has no receipt, is a source link");
    }

    #[test]
    fn published_status_has_no_stale_wording() {
        assert_eq!(stale_hit(&(status_json() + &llms_txt())), None);
    }

    #[test]
    fn counterexample_stale_wording_is_caught() {
        assert_eq!(stale_hit("how much a machine actually understood"), Some("actually understood"));
        assert_eq!(stale_hit("Measurement profiles in preparation"), Some("in preparation"));
    }

    // Repo-wide stale scan. Mirrors the receipt grep (phrases below, skips
    // target, node_modules, .git, dist and *.tex). The only exclusion is the
    // named source file, by path. Content is never used to exclude a hit.
    const REPO_STALE: [&str; 2] = ["actually understood", "first unit written down"];

    // Every pattern on a line is reported, so two stale phrases on one line
    // count as two hits.
    fn line_is_stale(line: &str) -> Vec<String> {
        let mut out: Vec<String> = REPO_STALE
            .iter()
            .filter(|s| line.contains(**s))
            .map(|s| (*s).to_string())
            .collect();
        // The stale claim is the TY-2 gain figure presented as Turing yield.
        // Checked per sentence (split on ". "), in either order, so a sentence
        // that gives the gain and a later sentence saying yield is not yet
        // measured is honest, while "Turing yield of 2,559,679" and
        // "2,559,679.825 bits of Turing yield" are caught.
        if line
            .split(". ")
            .any(|s| s.contains("Turing yield") && s.contains("2,559,679"))
        {
            out.push("Turing yield.*2,559,679".to_string());
        }
        out
    }

    #[test]
    fn yield_claim_is_caught_per_sentence() {
        // Stale claims, either order, one sentence: caught.
        for bad in [
            "a Turing yield of 2,559,679 bits",
            "2,559,679.825 bits of Turing yield",
            "Turing yield: +2,559,679.825 T/J under profile V0.",
            "Gain recorded. The Turing yield was 2,559,679.825 T.",
        ] {
            assert_eq!(line_is_stale(bad), vec!["Turing yield.*2,559,679".to_string()], "{bad}");
        }
        // Honest wording from the research page (lines 95 and 218): gain
        // figure and the "yield not yet measured" note are separate sentences.
        for ok in [
            "<p>The empirical-results section records the first canonical measurement: TY-1+TY-2 PASS, +2,559,679.825 T under measurement profile V0 (omega PR #85, 2026-09-29). What it does not claim: Turing yield (T/J) is not yet measured; joining energy measurements to Turing-gain receipts is the next step.</p>",
            "Version 1.0 reports the first canonical Turing measurement: TY-1+TY-2 PASS, +2,559,679.825 T under measurement profile V0 (omega PR #85, 2026-09-29). Turing yield in T/J is not yet reported; joining energy measurements to Turing-gain receipts is the next step.",
        ] {
            assert!(line_is_stale(ok).is_empty(), "{ok}");
        }
    }

    // Exact-path match after resolving "..", so a root such as
    // "crates/site-builder/../.." still matches the skipped file.
    fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
        match (a.canonicalize(), b.canonicalize()) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }

    // Scans text files; returns Err on any directory or file read error
    // (non-UTF-8 binary files such as png are the only thing not read as text).
    // `scanned` records every text file actually read.
    fn scan_tree(
        dir: &std::path::Path,
        skip: &[std::path::PathBuf],
        hits: &mut Vec<String>,
        scanned: &mut Vec<std::path::PathBuf>,
    ) -> Result<(), String> {
        let rd = std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
        for e in rd {
            let e = e.map_err(|e| format!("entry in {}: {e}", dir.display()))?;
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if ["target", "node_modules", ".git", "dist"].contains(&name.as_str()) {
                    continue;
                }
                scan_tree(&p, skip, hits, scanned)?;
            } else if p.extension().map_or(false, |x| x == "tex") {
                continue;
            } else if skip.iter().any(|s| same_file(s, &p)) {
                continue;
            } else {
                match std::fs::read_to_string(&p) {
                    Ok(text) => {
                        scanned.push(p.clone());
                        for (i, line) in text.lines().enumerate() {
                            for w in line_is_stale(line) {
                                hits.push(format!("{}:{}: {}", p.display(), i + 1, w));
                            }
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {}
                    Err(err) => return Err(format!("read {}: {err}", p.display())),
                }
            }
        }
        Ok(())
    }

    #[test]
    fn repo_has_no_stale_wording_outside_this_source_file() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let me = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/components/status.rs");
        let mut hits = Vec::new();
        let mut scanned = Vec::new();
        scan_tree(&root, &[me], &mut hits, &mut scanned).expect("scan must read every directory and file");
        check_floor(&root, &scanned).expect("scan floor");
        assert!(hits.is_empty(), "stale wording: {hits:?}");
    }

    fn check_floor(root: &std::path::Path, scanned: &[std::path::PathBuf]) -> Result<(), String> {
        if scanned.is_empty() {
            return Err("scan read zero files".into());
        }
        for must in ["index.html", "public/turing/index.html", "public/research/index.html"] {
            let want = root.join(must).canonicalize().ok();
            if !scanned.iter().any(|p| p.canonicalize().ok() == want) {
                return Err(format!("required page not scanned: {must}"));
            }
        }
        Ok(())
    }

    #[test]
    fn counterexample_empty_or_missing_directory_fails() {
        let missing = std::env::temp_dir().join(format!("stalegrep-missing-{}", std::process::id()));
        let mut hits = Vec::new();
        let mut scanned = Vec::new();
        assert!(scan_tree(&missing, &[], &mut hits, &mut scanned).is_err());
        let empty = std::env::temp_dir().join(format!("stalegrep-empty-{}", std::process::id()));
        std::fs::create_dir_all(&empty).unwrap();
        let r = scan_tree(&empty, &[], &mut hits, &mut scanned);
        let _ = std::fs::remove_dir_all(&empty);
        assert!(r.is_ok());
        assert!(check_floor(&empty, &scanned).is_err(), "empty scan must fail the floor");
    }

    #[test]
    fn counterexample_quoted_phrase_in_public_page_is_caught() {
        let d = std::env::temp_dir().join(format!("stalegrep-{}", std::process::id()));
        let public = d.join("public");
        std::fs::create_dir_all(&public).unwrap();
        let page = public.join("x.html");
        let src = d.join("status.rs");
        std::fs::write(&page, "<p>\"Turing yield of 2,559,679\" and \"actually understood\"</p>\n").unwrap();
        std::fs::write(&src, "const A: &str = \"first unit written down\";\n").unwrap();
        let mut hits = Vec::new();
        let mut scanned = Vec::new();
        scan_tree(&d, &[src.clone()], &mut hits, &mut scanned).unwrap();
        // Same tree through a ".." root, as the repo test uses: the exclusion
        // must still hold and the page must still be caught.
        let mut hits2 = Vec::new();
        let mut scanned2 = Vec::new();
        scan_tree(&public.join(".."), &[src.clone()], &mut hits2, &mut scanned2).unwrap();
        // Without the exclusion the source file is caught as well.
        let mut hits3 = Vec::new();
        let mut scanned3 = Vec::new();
        scan_tree(&d, &[], &mut hits3, &mut scanned3).unwrap();
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert!(hits.iter().all(|h| h.contains("x.html")), "{hits:?}");
        assert_eq!(hits2.len(), 2, "{hits2:?}");
        assert!(hits2.iter().all(|h| h.contains("x.html")), "{hits2:?}");
        assert_eq!(hits3.len(), 3, "{hits3:?}");
    }
}
