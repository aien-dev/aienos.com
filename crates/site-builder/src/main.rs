mod components;
mod layouts;

use std::fs;
use std::path::Path;
use std::time::Instant;

use layouts::base::render_base_layout;
use maud::html;

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn generate_sitemap_and_robots(dist: &Path) {
    let sitemap = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url><loc>https://aienos.com/</loc><priority>1.0</priority></url>\n</urlset>\n";
    let sitemap_path = dist.join("sitemap.xml");
    fs::write(&sitemap_path, sitemap).expect("Failed to write sitemap.xml");

    let robots = "User-agent: *\nAllow: /\nSitemap: https://aienos.com/sitemap.xml\n";
    let robots_path = dist.join("robots.txt");
    fs::write(&robots_path, robots).expect("Failed to write robots.txt");

    println!("  [GEN] Emitted native sitemap.xml and robots.txt in: {}", dist.display());
}

fn build_pure_rust_site(dist: &Path) {
    println!("  [BUILD] Compiling pure Rust Maud static site into: {}", dist.display());
    fs::create_dir_all(dist.join("assets")).expect("Failed to create assets dir");
    fs::create_dir_all(dist.join("js")).expect("Failed to create js dir");

    // Copy stylesheet
    let css_src = Path::new("src/index.css");
    if css_src.exists() {
        fs::copy(css_src, dist.join("assets/style.css")).expect("Failed to copy style.css");
        println!("  [CSS] Copied src/index.css -> assets/style.css");
    }

    // Copy public assets
    let public_dir = Path::new("public");
    if public_dir.exists() {
        for entry in fs::read_dir(public_dir).expect("Failed to read public dir") {
            let entry = entry.expect("Valid entry");
            let path = entry.path();
            let name = entry.file_name();
            let dest = dist.join(&name);
            if path.is_dir() {
                copy_dir_all(&path, &dest).expect("Failed to copy public subfolder");
            } else {
                fs::copy(&path, &dest).expect("Failed to copy public file");
            }
        }
        println!("  [ASSETS] Synced public/ assets into {}", dist.display());
    }

    // Render landing page in Maud
    let page_content = html! {
        (components::hero::render_hero())
        (components::terminal::render_terminal())
        (components::benchmarks::render_benchmarks())
        (components::matrix::render_matrix())
        (components::ecosystem::render_ecosystem())
        (components::install::render_install())
    };

    let full_document = render_base_layout(page_content);
    let output_file = dist.join("index.html");
    fs::write(&output_file, full_document.into_string()).expect("Failed to write index.html");
    println!("  [PAGE] Emitted / (index.html)");

    generate_sitemap_and_robots(dist);
}

fn verify_site(dist: &Path) {
    let index_file = dist.join("index.html");
    assert!(index_file.exists(), "Required index.html missing in {}", dist.display());

    let content = fs::read_to_string(&index_file).expect("Failed to read index.html");

    // Verify DOM structure
    assert!(content.contains("<div id=\"root\""), "Missing #root element in index.html");
    assert!(content.contains("<main id=\"main-content\""), "Missing main container in index.html");
    assert!(content.contains("id=\"terminal-output\""), "Missing interactive terminal output in index.html");

    // Verify Unslop standard
    assert!(!content.contains('\u{2014}'), "Forbidden em dash detected in index.html");
    assert!(!content.contains('\u{2013}'), "Forbidden en dash detected in index.html");

    // Verify sitemap and robots
    assert!(dist.join("sitemap.xml").exists(), "Missing sitemap.xml");
    assert!(dist.join("robots.txt").exists(), "Missing robots.txt");

    println!("------------------------------------------------------------");
    println!("  Verified index.html ({} bytes)", content.len());
    println!("  Integrity & Unslop Invariants: PASS");
}

fn main() {
    let start = Instant::now();
    let first_arg = std::env::args().nth(1).unwrap_or_else(|| "build".to_string());
    let (mode, target_dir) = if first_arg == "build" || first_arg == "verify" {
        (first_arg, std::env::args().nth(2).unwrap_or_else(|| "dist-rust".to_string()))
    } else {
        ("verify".to_string(), first_arg)
    };

    let dist = Path::new(&target_dir);

    println!("============================================================");
    println!("  AIEN OS Native Static Site Builder & Integrity Verifier   ");
    println!("  Engine: Pure Rust & Maud | Target: {}", dist.display());
    println!("  Mode: {}", mode);
    println!("============================================================");

    if mode == "build" {
        build_pure_rust_site(dist);
        verify_site(dist);
    } else {
        if !dist.exists() {
            eprintln!("Error: Target directory '{}' does not exist.", dist.display());
            std::process::exit(1);
        }
        verify_site(dist);
    }

    let elapsed = start.elapsed();
    println!("  Build and Verification Completed in: {:.2?}", elapsed);
    println!("============================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unslop_punctuation_invariants() {
        let sample = "AIEN OS: The GPU-Native Neural Operating Environment.";
        assert!(!sample.contains('\u{2014}'), "Must not contain em dash");
        assert!(!sample.contains('\u{2013}'), "Must not contain en dash");
    }

    #[test]
    fn test_layout_rendering() {
        let markup = render_base_layout(html! { p { "Test" } });
        let s = markup.into_string();
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains("<div id=\"root\""));
        assert!(s.contains("AIEN OS"));
    }
}
