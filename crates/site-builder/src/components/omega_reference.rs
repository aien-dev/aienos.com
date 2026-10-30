//! /omega/reference/: the agent-facing Omega language reference. Seeded
//! Omega writes it: om/omega-reference/page.om renders the vendored
//! omega-harness guide and every gated worked example (om/omega-reference/
//! check.sh), and this module only places the generated HTML in the layout.
use maud::{html, Markup, PreEscaped};

pub const TITLE: &str = "Omega language reference | AienOS";
pub const DESCRIPTION: &str = "The Omega language for agents: program shape, types, declarations, statements, words, OM-2, host words, every compiler error and trap exit code, common mistakes and every gated worked example with its expected output.";
pub const URL: &str = "https://www.aienos.com/omega/reference/";
pub const PAGE: &str = include_str!("../../../../om/omega-reference/main.html");
pub fn render_omega_reference_page() -> Markup { html! { (PreEscaped(PAGE)) } }

/// Vendored files (path relative to om/omega-reference/vendor) with their
/// recorded sha256, in manifest order.
pub fn manifest() -> Vec<(&'static str, &'static str)> {
    include_str!("../../../../om/omega-reference/vendor.sha256")
        .lines()
        .map(|l| {
            let (hash, path) = l.split_once("  ").expect("sha256sum line");
            (path, hash)
        })
        .collect()
}

/// Every vendored file must appear in llms-full.txt byte for byte as its
/// `File: PATH` label and fenced block, the form html_to_text gives the page.
pub fn verify_llms_full(dist: &std::path::Path) {
    let full = std::fs::read_to_string(dist.join("llms-full.txt")).expect("read llms-full.txt");
    let files = extract(&full);
    let vendor = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../om/omega-reference/vendor");
    let mut programs = 0;
    for (path, _) in manifest() {
        if path == "skills/writing-omega/SKILL.md" {
            continue;
        }
        let want = std::fs::read(vendor.join(path)).unwrap_or_else(|_| panic!("read vendored {path}"));
        let got = files.iter().find(|(p, _)| p == path).unwrap_or_else(|| panic!("llms-full.txt lacks File: {path}"));
        assert!(got.1 == want, "llms-full.txt changes the bytes of {path}");
        programs += usize::from(path.ends_with(".om"));
    }
    assert!(programs >= 30, "/omega/reference/ must carry at least 30 worked examples, found {programs}");
    println!("  [LLMS] llms-full.txt carries {} reference files byte for byte ({programs} programs)", files.len());
}

/// Files in llms-full.txt: each `File: PATH` line, with an optional
/// `(empty file)`, `(no final newline)` or `(hex bytes)` mark, and the
/// fenced block after it. A `File:` line in other copy, without a mark or
/// fence of this form, is not a file and is passed over.
pub fn extract(full: &str) -> Vec<(String, Vec<u8>)> {
    let lines: Vec<&str> = full.split('\n').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(label) = lines[i].strip_prefix("File: ") else { i += 1; continue };
        let (path, mark) = match label.split_once(' ') {
            Some((p, m)) => (p, m),
            None => (label, ""),
        };
        i += 1;
        if !matches!(mark, "" | "(empty file)" | "(no final newline)" | "(hex bytes)") {
            continue;
        }
        if mark == "(empty file)" {
            out.push((path.to_string(), Vec::new()));
            continue;
        }
        let mut f = i;
        while f < lines.len() && lines[f].is_empty() {
            f += 1;
        }
        if f >= lines.len() || lines[f] != "```" {
            continue;
        }
        let start = f + 1;
        let Some(len) = lines[start..].iter().position(|l| *l == "```") else { continue };
        let end = start + len;
        let mut body = String::new();
        for l in &lines[start..end] {
            body.push_str(l);
            body.push('\n');
        }
        let bytes = match mark {
            "(no final newline)" => body[..body.len() - 1].as_bytes().to_vec(),
            "(hex bytes)" => body.split_whitespace().map(|h| u8::from_str_radix(h, 16).expect("hex byte")).collect(),
            _ => body.into_bytes(),
        };
        out.push((path.to_string(), bytes));
        i = end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn reference_generated_artifacts_match_hashes() {
        let receipt = include_str!("../../../../om/omega-reference/main.html.sha256");
        assert_eq!(format!("{:x}", Sha256::digest(PAGE.as_bytes())), receipt.split_whitespace().next().expect("digest"));
        let vendor = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../om/omega-reference/vendor");
        let listed = manifest();
        for (path, hash) in &listed {
            let bytes = std::fs::read(vendor.join(path)).unwrap_or_else(|_| panic!("read {path}"));
            assert_eq!(&format!("{:x}", Sha256::digest(&bytes)), hash, "{path} differs from vendor.sha256");
        }
        fn count(dir: &std::path::Path) -> usize {
            std::fs::read_dir(dir).expect("read vendor dir").map(|e| {
                let p = e.expect("entry").path();
                if p.is_dir() { count(&p) } else { 1 }
            }).sum()
        }
        assert_eq!(count(&vendor), listed.len(), "vendor/ holds a file that vendor.sha256 does not list");
    }

    #[test]
    fn reference_page_covers_every_program_and_the_error_table() {
        let programs: Vec<_> = manifest().into_iter().filter(|(p, _)| p.ends_with(".om")).collect();
        assert!(programs.len() >= 30, "at least 30 worked examples, found {}", programs.len());
        for (path, _) in &programs {
            assert!(PAGE.contains(&format!("File: <code>{path}</code>")), "main.html lacks {path}");
        }
        for needle in ["id=\"s11\"", "id=\"s5\"", "id=\"s12\"", "id=\"examples\"", "id=\"gate\"", "File: <code>examples/gate.sh</code>", "never emitted at the pin", "<td>24</td>", "e5b32567a7aff3ce594e3254a622a846a6cd190b", "2c82480596bbdc50cda9626866209ef662d5834f278f839ff4ce0250c495029f"] {
            assert!(PAGE.contains(needle), "main.html lacks {needle}");
        }
        assert!(!PAGE.contains('\u{2014}') && !PAGE.contains('\u{2013}'), "no em or en dash");
        let info = crate::seo::PAGES.iter().find(|p| p.path == "/omega/reference/").expect("/omega/reference/ in seo::PAGES");
        let count = format!(" {} worked examples", programs.len());
        assert!(info.what.contains(&count), "seo::PAGES must give the example count{count}");
        assert!(PAGE.contains(&count), "main.html must give the example count{count}");
    }

    #[test]
    fn extract_reads_marks_and_fences() {
        let full = "File: notes on this, not a file\nFile: a.om\n\n```\nnote a\n\n  x\n```\nFile: b.stdin (no final newline)\n```\nab\n```\nFile: c.out (hex bytes)\n```\n00 ff 0a\n```\nFile: d.stdout (empty file)\n";
        let got = extract(full);
        assert_eq!(got[0], ("a.om".to_string(), b"note a\n\n  x\n".to_vec()));
        assert_eq!(got[1], ("b.stdin".to_string(), b"ab".to_vec()));
        assert_eq!(got[2], ("c.out".to_string(), vec![0, 255, 10]));
        assert_eq!(got[3], ("d.stdout".to_string(), Vec::new()));
        assert_eq!(got.len(), 4);
    }
}
