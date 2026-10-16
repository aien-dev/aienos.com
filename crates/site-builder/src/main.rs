//! aienos.com: one static page, built from Rust, with its changing facts read from a JSON file
//! that the `facts` subcommand gathers from the public GitHub record.
//!
//!   site-builder facts [facts.json]        gather live facts (needs `gh`, read-only)
//!   site-builder build [dist] [facts.json] render the page from the facts file
//!   site-builder verify [dist]             check an existing output directory

mod components;
mod facts;
mod layouts;

use std::fs;
use std::path::Path;
use std::process::exit;
use std::time::Instant;

use facts::Facts;
use layouts::base::{render_base_layout, SITE_URL};
use maud::html;

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn write_sitemap_and_robots(dist: &Path) -> std::io::Result<()> {
    let sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url><loc>{SITE_URL}</loc><priority>1.0</priority></url>\n</urlset>\n"
    );
    fs::write(dist.join("sitemap.xml"), sitemap)?;
    fs::write(dist.join("robots.txt"), format!("User-agent: *\nAllow: /\nSitemap: {SITE_URL}sitemap.xml\n"))
}

pub fn render_page(facts: &Facts) -> String {
    let body = html! {
        (components::hero::render_hero(facts))
        (components::what::render_what())
        (components::status::render_status(facts))
        (components::how::render_how(facts))
        (components::principles::render_principles())
        (components::install::render_install())
    };
    render_base_layout(body).into_string()
}

fn build(dist: &Path, facts_path: &Path) -> Result<(), String> {
    let facts = Facts::load(facts_path)?;
    fs::create_dir_all(dist).map_err(|e| e.to_string())?;
    copy_dir_all(Path::new("public"), dist).map_err(|e| format!("copying public/: {e}"))?;
    fs::write(dist.join("index.html"), render_page(&facts)).map_err(|e| e.to_string())?;
    fs::copy(facts_path, dist.join("facts.json")).map_err(|e| format!("copying facts: {e}"))?;
    write_sitemap_and_robots(dist).map_err(|e| e.to_string())?;
    println!("  built {} from {} ({} components, receipt {})", dist.display(), facts_path.display(), facts.components.len(), facts.receipt.state);
    Ok(())
}

/// The page's invariants. A build that breaks one of these does not deploy.
pub fn verify_html(content: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let mut need = |cond: bool, what: &str| {
        if !cond {
            problems.push(what.to_string());
        }
    };
    need(content.contains("<main id=\"main-content\""), "main container missing");
    need(content.contains("id=\"receipt\""), "verification receipt missing");
    need(content.contains("id=\"components\""), "components table missing");
    need(content.contains("id=\"verification\""), "verification section missing");
    need(content.contains("AGPL-3.0-or-later"), "licence line missing");
    need(!content.contains("<script"), "page must not carry scripts");
    need(!content.contains("fonts.googleapis.com") && !content.contains("fonts.gstatic.com"), "page must not load outside fonts");
    need(!content.contains("waitlist"), "stale waitlist copy");
    need(!content.contains('\u{2014}') && !content.contains('\u{2013}'), "em or en dash in page");
    // Nothing on the page compares AIEN with somebody else's system.
    for word in ["FastAPI", "vLLM", "llama.cpp", "faster than", "x faster"] {
        need(!content.contains(word), &format!("outside comparison or forbidden word: {word}"));
    }
    problems
}

fn verify(dist: &Path) -> Result<(), String> {
    let index = dist.join("index.html");
    let content = fs::read_to_string(&index).map_err(|e| format!("cannot read {}: {e}", index.display()))?;
    let mut problems = verify_html(&content);
    for f in ["assets/style.css", "facts.json", "sitemap.xml", "robots.txt", "favicon.svg", "install.sh", "CNAME"] {
        if !dist.join(f).exists() {
            problems.push(format!("missing {f}"));
        }
    }
    if problems.is_empty() {
        println!("  verified {} ({} bytes): all invariants hold", index.display(), content.len());
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

fn main() {
    let start = Instant::now();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(String::as_str).unwrap_or("build");
    let result = match mode {
        "facts" => {
            let out = Path::new(args.get(1).map(String::as_str).unwrap_or("facts.json"));
            facts::gather().and_then(|f| {
                let n = f.components.len();
                f.save(out).map(|_| println!("  wrote {} ({n} components, receipt {} for {})", out.display(), f.receipt.state, &f.receipt.sha[..f.receipt.sha.len().min(12)]))
            })
        }
        "build" => {
            let dist = Path::new(args.get(1).map(String::as_str).unwrap_or("dist"));
            let facts_path = Path::new(args.get(2).map(String::as_str).unwrap_or("facts.json"));
            build(dist, facts_path).and_then(|_| verify(dist))
        }
        "verify" => verify(Path::new(args.get(1).map(String::as_str).unwrap_or("dist"))),
        other => Err(format!("unknown mode {other}; use facts | build | verify")),
    };
    match result {
        Ok(()) => println!("  done in {:.2?}", start.elapsed()),
        Err(e) => {
            eprintln!("error: {e}");
            exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_facts() -> Facts {
        let text = include_str!("../../../facts.json");
        serde_json::from_str(text).expect("facts.json in the repository root parses")
    }

    #[test]
    fn page_renders_and_holds_invariants() {
        let page = render_page(&sample_facts());
        assert!(page.starts_with("<!DOCTYPE html>"));
        let problems = verify_html(&page);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn page_carries_every_component_and_the_receipt_commit() {
        let facts = sample_facts();
        let page = render_page(&facts);
        for c in &facts.components {
            assert!(page.contains(&c.url), "missing link to {}", c.repo);
        }
        assert!(page.contains(&facts.receipt.sha[..12]));
        assert!(page.contains(&facts.generated_at));
    }

    #[test]
    fn verify_rejects_scripts_fonts_and_comparisons() {
        assert!(verify_html("<main id=\"main-content\"><script></script>").iter().any(|p| p.contains("scripts")));
        assert!(verify_html("3x faster than FastAPI").iter().any(|p| p.contains("comparison")));
        assert!(verify_html("fonts.googleapis.com").iter().any(|p| p.contains("fonts")));
    }
}
