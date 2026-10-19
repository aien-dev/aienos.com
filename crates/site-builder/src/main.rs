mod collector;
mod components;
mod facts;
mod layouts;

use std::fs;
use std::path::Path;
use std::time::Instant;

use layouts::base::{render_base_layout, render_page_layout};
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
        if entry.file_name() == ".crumb" || entry.file_name() == ".crumb.local" { continue; }
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
    let sitemap = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url><loc>https://aienos.com/</loc><priority>1.0</priority></url>\n  <url><loc>https://aienos.com/progress/</loc><priority>0.8</priority></url>\n  <url><loc>https://aienos.com/blog</loc><priority>0.8</priority></url>\n  <url><loc>https://aienos.com/blog/aien-v3-research-plan</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/post-llm-case/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/research/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/research/status/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/now/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/research/computing-machinery-and-understanding</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/research/computing-machinery-and-understanding-emergence</loc><priority>0.8</priority></url>\n  <url><loc>https://aienos.com/research/the-stapleton-doctrine</loc><priority>0.8</priority></url>\n  <url><loc>https://aienos.com/machine/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/turing/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/experiments/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/discovery/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/evidence/</loc><priority>0.9</priority></url>\n  <url><loc>https://aienos.com/philosophy/</loc><priority>0.8</priority></url>\n  <url><loc>https://aienos.com/licensing/</loc><priority>0.5</priority></url>\n</urlset>\n";
    let sitemap_path = dist.join("sitemap.xml");
    fs::write(&sitemap_path, sitemap).expect("Failed to write sitemap.xml");

    // Bot-friendly: the site is open to all agents and humans alike.
    // AI crawlers are explicitly allowed; there are no CAPTCHAs and no
    // bot blocking at the gate.
    let robots = "User-agent: *\nAllow: /\n\nUser-agent: GPTBot\nAllow: /\n\nUser-agent: ChatGPT-User\nAllow: /\n\nUser-agent: ClaudeBot\nAllow: /\n\nUser-agent: anthropic-ai\nAllow: /\n\nUser-agent: CCBot\nAllow: /\n\nUser-agent: Bytespider\nAllow: /\n\nSitemap: https://aienos.com/sitemap.xml\n";
    let robots_path = dist.join("robots.txt");
    fs::write(&robots_path, robots).expect("Failed to write robots.txt");

    println!("  [GEN] Emitted native sitemap.xml and robots.txt in: {}", dist.display());
}

fn build_pure_rust_site(dist: &Path, facts_path: &Path) {
    let facts = facts::Facts::load(facts_path)
        .unwrap_or_else(|e| panic!("Cannot build without facts: {e}. Run `site-builder facts {}` first.", facts_path.display()));
    println!("  [FACTS] Using facts gathered at {} from {}", facts.generated_at, facts_path.display());
    println!("  [BUILD] Compiling pure Rust Maud static site into: {}", dist.display());
    fs::create_dir_all(dist.join("assets")).expect("Failed to create assets dir");
    fs::create_dir_all(dist.join("js")).expect("Failed to create js dir");

    // Copy public assets
    let public_dir = Path::new("public");
    if public_dir.exists() {
        for entry in fs::read_dir(public_dir).expect("Failed to read public dir") {
            let entry = entry.expect("Valid entry");
            let path = entry.path();
            let name = entry.file_name();
            if name == ".crumb" || name == ".crumb.local" { continue; }
            let dest = dist.join(&name);
            if path.is_dir() {
                copy_dir_all(&path, &dest).expect("Failed to copy public subfolder");
            } else {
                fs::copy(&path, &dest).expect("Failed to copy public file");
            }
        }
        println!("  [ASSETS] Synced public/ assets into {}", dist.display());
    }

    // Canonical project state for machines: rendered from the same
    // source as the homepage status section (components::status), so
    // the page, the JSON, and llms.txt can never drift apart.
    fs::write(
        dist.join("status.json"),
        components::status::status_json(),
    )
    .expect("Failed to write status.json");
    fs::write(dist.join("llms.txt"), components::status::llms_txt())
        .expect("Failed to write llms.txt");
    // The machines and speeds data, published next to the page that renders it.
    fs::write(dist.join("machines.json"), components::machines::RAW)
        .expect("Failed to write machines.json");
    println!("  [STATE] Wrote status.json and llms.txt from the canonical layer states");

    // Render landing page in Maud
    let page_content = html! {
        (components::home_loop::render_home_loop())
        (components::story::render_story())
        (components::machines::render_machines_section())
        (components::machines::render_speeds_section())
        (components::evidence::render_evidence())
        (components::matrix::render_matrix())
        (components::research::render_research())
        (components::terminal::render_terminal())
        (components::status::render_status(&facts))
        (components::follow::render_follow())
    };

    let full_document = render_base_layout(page_content);
    let output_file = dist.join("index.html");
    fs::write(&output_file, full_document.into_string()).expect("Failed to write index.html");
    println!("  [PAGE] Emitted / (index.html)");
    fs::copy(facts_path, dist.join("facts.json")).expect("Failed to copy facts.json");
    println!("  [FACTS] Published /facts.json");

    // Public project measurements, collected before compilation in CI.
    let progress = render_page_layout(
        "Project progress | AIEN OS",
        "Explore the retained AIEN project history: codebase growth, revisions, test footprint and verification activity, refreshed hourly from public GitHub sources.",
        "https://aienos.com/progress/", "website",
        html! {
            link rel="stylesheet" href="/assets/progress.css";
            script defer src="/vendor/d3-7.9.0.min.js" {}
            script defer src="/js/project-progress.js" {}
        }, components::progress::render_progress_page(),
    );
    fs::create_dir_all(dist.join("progress")).expect("Create progress directory");
    fs::write(dist.join("progress/index.html"), progress.into_string()).expect("Write progress page");

    // Blog index page
    let blog_index = render_page_layout(
        "Blog | AIEN OS",
        "Research notes on building AIEN V2: what shipped, what the evidence says, and what remains an open question.",
        "https://aienos.com/blog",
        "website",
        html! {},
        components::blog::render_blog_index(),
    );
    let blog_index_dir = dist.join("blog");
    fs::create_dir_all(&blog_index_dir).expect("Failed to create blog dir");
    fs::write(blog_index_dir.join("index.html"), blog_index.into_string())
        .expect("Failed to write blog index.html");
    println!("  [PAGE] Emitted /blog (index.html)");

    // First post: the AIEN V3 research plan
    let blog_post = render_page_layout(
        &format!("{} | AIEN OS Blog", components::blog::POST_TITLE),
        components::blog::POST_DESCRIPTION,
        &format!("https://aienos.com/blog/{}", components::blog::POST_SLUG),
        "article",
        html! {},
        components::blog::render_blog_post(),
    );
    let post_dir = blog_index_dir.join(components::blog::POST_SLUG);
    fs::create_dir_all(&post_dir).expect("Failed to create post dir");
    fs::write(post_dir.join("index.html"), blog_post.into_string())
        .expect("Failed to write post index.html");
    println!("  [PAGE] Emitted /blog/{} (index.html)", components::blog::POST_SLUG);

    // Research status: ladder, experiment cards, and implementation list,
    // all rendered from data/research_status.json. The data file itself is
    // published next to it so machines read the same source.
    let research_status = render_page_layout(
        "Research status | AIEN OS",
        "Where the AIEN research stands: the qualification ladder, every experiment with its verdict, receipt and limits, and what is built versus still a hypothesis.",
        "https://aienos.com/research/status/",
        "website",
        html! {},
        components::research_status::render_research_status_page(),
    );
    let research_status_dir = dist.join("research/status");
    fs::create_dir_all(&research_status_dir).expect("Failed to create research/status dir");
    fs::write(research_status_dir.join("index.html"), research_status.into_string())
        .expect("Failed to write research/status/index.html");
    fs::write(dist.join("research-status.json"), components::research_status::RAW)
        .expect("Failed to write research-status.json");
    println!("  [PAGE] Emitted /research/status (index.html) and /research-status.json");

    // /now/: the live coordination ledger, laid into public/data/now.tsv by
    // the pages workflow from the data/now branch (components::now).
    let now = components::now::load();
    let now_page = render_page_layout(
        components::now::TITLE,
        components::now::DESCRIPTION,
        components::now::URL,
        "website",
        html! { link rel="stylesheet" href="/assets/now.css"; },
        components::now::render_now_page(&now),
    );
    fs::create_dir_all(dist.join("now")).expect("Failed to create now dir");
    fs::write(dist.join("now/index.html"), now_page.into_string()).expect("Failed to write now/index.html");
    fs::write(dist.join("now.json"), components::now::to_json(&now)).expect("Failed to write now.json");
    println!("  [PAGE] Emitted /now/ ({} ledger lines) and /now.json", now.items.len());
    // Research narrative pages (NARR-RESEARCH): The Machine and Scientific Discovery
    for (slug, title, description, url, body) in [
        (
            "machine",
            components::machine::TITLE,
            components::machine::DESCRIPTION,
            components::machine::URL,
            components::machine::render_machine(),
        ),
        (
            "discovery",
            components::discovery::TITLE,
            components::discovery::DESCRIPTION,
            components::discovery::URL,
            components::discovery::render_discovery(),
        ),
    ] {
        let page = render_page_layout(title, description, url, "article", html! {}, body);
        let page_dir = dist.join(slug);
        fs::create_dir_all(&page_dir).expect("Failed to create narrative page dir");
        fs::write(page_dir.join("index.html"), page.into_string())
            .expect("Failed to write narrative page index.html");
        println!("  [PAGE] Emitted /{}/ (index.html)", slug);
    }

    // Section hubs: /experiments/, /evidence/, /philosophy/ (NARR-NAV)
    components::hubs::emit_hub_pages(dist);

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
    assert!(content.contains("id=\"follow\""), "Missing follow section in index.html");
    assert!(content.contains("href=\"/now/\""), "Missing the Now link in the homepage follow section");
    assert!(content.contains("id=\"receipt\""), "Missing verification receipt in index.html");
    assert!(content.contains("id=\"live\""), "Missing live components block in index.html");
    assert!(!content.contains("fonts.googleapis.com"), "Outside font loaded in index.html");
    assert!(dist.join("facts.json").exists(), "Missing facts.json");

    // Homepage top (NARR-HOME): literal phrases, so a wording change in the
    // component cannot pass by accident.
    for phrase in [
        "AIENOS is an owned experimental machine for turning search into verified understanding.",
        "Weights suggest. Programs explain. Verification decides. Evidence teaches. The Turing keeps score.",
        "Did it learn anything?",
        "What did it cost to learn?",
        "Did what it learned make future discovery easier?",
    ] {
        assert!(content.contains(phrase), "Missing homepage phrase in index.html: {phrase}");
    }
    assert!(content.contains("<ol class=\"loop-stages\""), "Missing loop stage list in index.html");

    // The machines and the speeds (components::machines): five machines,
    // the 27B on Omega first, every number beside its receipt, and no
    // comparison with anyone else's system anywhere on the homepage.
    assert!(content.contains("id=\"machines\""), "Missing machines section in index.html");
    assert!(content.contains("id=\"speed\""), "Missing speed section in index.html");
    for id in ["machine-spark", "machine-imac", "machine-pi5", "machine-qemu", "machine-rig", "speed-27b-live"] {
        assert!(content.contains(&format!("id=\"{id}\"")), "Missing {id} in index.html");
    }
    assert!(content.contains("7.63 tok/s"), "Missing the live 27B speed in index.html");
    {
        let lower = content.to_ascii_lowercase();
        for w in components::machines::FORBIDDEN {
            assert!(!lower.contains(w), "Comparison with another system on the homepage: {w:?}");
        }
    }
    assert!(dist.join("assets/machines.css").exists(), "Missing assets/machines.css");
    assert!(dist.join("machines.json").exists(), "Missing machines.json");

    // Verify asset output
    assert!(dist.join("assets/style.css").exists(), "Missing assets/style.css");
    assert!(dist.join("js/terminal.js").exists(), "Missing js/terminal.js");

    // A stray or missing brace silently nests every later rule inside an
    // earlier one (this left the footer unstyled), so refuse unbalanced CSS.
    let css = fs::read_to_string(dist.join("assets/style.css")).expect("read assets/style.css");
    let mut depth: i64 = 0;
    for ch in css.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        assert!(depth >= 0, "assets/style.css has an unmatched closing brace");
    }
    assert_eq!(depth, 0, "assets/style.css has an unclosed rule; later rules would be nested");
    for class in ["aien-footer", "aien-footer-sections", "aien-footer-heading", "aien-footer-group", "aien-footer-legal", "aien-footer-mottos"] {
        assert!(css.contains(&format!(".{class}")), "assets/style.css missing footer rule .{class}");
    }

    // Verify Unslop standard
    assert!(!content.contains('\u{2014}'), "Forbidden em dash detected in index.html");
    assert!(!content.contains('\u{2013}'), "Forbidden en dash detected in index.html");

    assert!(dist.join("progress/index.html").exists(), "Missing progress page");
    assert!(dist.join("data/project-progress.json").exists(), "Missing progress dataset");
    assert!(!dist.join("data/.crumb.local").exists(), "Local coordination must not be published");

    // Verify sitemap and robots
    assert!(dist.join("sitemap.xml").exists(), "Missing sitemap.xml");
    assert!(dist.join("robots.txt").exists(), "Missing robots.txt");

    // Verify blog pages
    let blog_index = dist.join("blog/index.html");
    assert!(blog_index.exists(), "Missing blog/index.html");
    let blog_post = dist.join("blog/aien-v3-research-plan/index.html");
    assert!(blog_post.exists(), "Missing blog/aien-v3-research-plan/index.html");
    let research_status = dist.join("research/status/index.html");
    assert!(research_status.exists(), "Missing research/status/index.html");
    let now_page = dist.join("now/index.html");
    assert!(now_page.exists(), "Missing now/index.html");
    assert!(dist.join("now.json").exists(), "Missing now.json");
    assert!(dist.join("assets/now.css").exists(), "Missing assets/now.css");
    assert!(dist.join("research-status.json").exists(), "Missing research-status.json");
    for page in [&blog_index, &blog_post, &research_status, &now_page] {
        let page_content = fs::read_to_string(page)
            .unwrap_or_else(|_| panic!("Failed to read {}", page.display()));
        assert!(
            !page_content.contains('\u{2014}'),
            "Forbidden em dash detected in {}",
            page.display()
        );
        assert!(
            !page_content.contains('\u{2013}'),
            "Forbidden en dash detected in {}",
            page.display()
        );
        assert!(
            page_content.contains("og:title"),
            "Missing og:title in {}",
            page.display()
        );
    }

    // Section hubs exist, carry their required phrase and the six-section
    // nav, and contain no em or en dash.
    components::hubs::verify_hub_pages(dist);

    // Verify research narrative pages (NARR-RESEARCH)
    for slug in ["machine", "discovery"] {
        let page = dist.join(slug).join("index.html");
        assert!(page.exists(), "Missing {}", page.display());
        let page_content = fs::read_to_string(&page)
            .unwrap_or_else(|_| panic!("Failed to read {}", page.display()));
        assert!(!page_content.contains('\u{2014}'), "Forbidden em dash detected in {}", page.display());
        assert!(!page_content.contains('\u{2013}'), "Forbidden en dash detected in {}", page.display());
        assert!(page_content.contains("og:title"), "Missing og:title in {}", page.display());
        assert!(page_content.contains("rs-badge"), "Missing data-driven status badges in {}", page.display());
        assert!(!page_content.contains("status-badge"), "Old hand-kept status badge markup in {}", page.display());
    }
    let machine = fs::read_to_string(dist.join("machine/index.html")).expect("Failed to read machine page");
    assert!(machine.contains(components::narrative::SLOGAN), "Missing slogan in machine/index.html");
    assert!(machine.contains(components::narrative::TURING_FORMULA), "Missing Turing formula in machine/index.html");
    assert!(machine.contains("id=\"real-machines\""), "Missing real-machines section in machine/index.html");
    assert!(machine.find("id=\"real-machines\"") < machine.find("id=\"loop\""), "Real machines must come before the loop in machine/index.html");
    assert!(machine.contains("id=\"research-ladder\""), "Missing research-ladder section in machine/index.html");
    assert!(machine.contains("class=\"rs-ladder\""), "Missing research ladder markup in machine/index.html");
    assert!(machine.contains("class=\"rs-impl\""), "Missing implementation list in machine/index.html");

    // Every box on every generated page links to its evidence on GitHub
    // (components::boxlink). A page listed here with a floor below it must
    // carry at least that many boxes, so a component that stops rendering
    // its cards is caught as well as one that renders them without a link.
    let mut boxes_total = 0;
    for (page, floor) in GENERATED_PAGES {
        let path = dist.join(page);
        let html = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {}", path.display()));
        let n = components::boxlink::check_boxes(&html).unwrap_or_else(|e| panic!("/{page}: {e}"));
        assert!(n >= *floor, "/{page}: expected at least {floor} boxes, found {n}");
        boxes_total += n;
    }
    assert!(
        fs::read_to_string(dist.join("assets/turing-look.css")).expect("read turing-look.css").contains(".box-link::after"),
        "assets/turing-look.css must stretch .box-link over its box"
    );

    println!("------------------------------------------------------------");
    println!("  Verified index.html ({} bytes)", content.len());
    println!("  Every box links to GitHub: {boxes_total} boxes across {} pages", GENERATED_PAGES.len());
    println!("  Integrity & Unslop Invariants: PASS");
}

/// Every page this builder generates, with the least number of boxes it
/// must carry. /now/ has no floor because the ledger may be absent.
const GENERATED_PAGES: &[(&str, usize)] = &[
    ("index.html", 30),
    ("machine/index.html", 15),
    ("discovery/index.html", 0),
    ("experiments/index.html", 15),
    ("evidence/index.html", 7),
    ("philosophy/index.html", 3),
    ("research/status/index.html", 40),
    ("now/index.html", 0),
    ("progress/index.html", 0),
    ("blog/index.html", 2),
    ("blog/aien-v3-research-plan/index.html", 0),
];

fn main() {
    let start = Instant::now();
    let first_arg = std::env::args().nth(1).unwrap_or_else(|| "build".to_string());

    // `facts [facts.json]`: read the public GitHub record and write the facts file. Read-only,
    // needs `gh`. Run by the workflow before every build so the page never goes stale by hand.
    if first_arg == "facts" {
        let out = std::env::args().nth(2).unwrap_or_else(|| "facts.json".to_string());
        let gathered = facts::gather().unwrap_or_else(|e| {
            eprintln!("facts: {e}");
            std::process::exit(1);
        });
        gathered.save(Path::new(&out)).unwrap_or_else(|e| {
            eprintln!("facts: cannot write {out}: {e}");
            std::process::exit(1);
        });
        println!(
            "  [FACTS] {out}: {} components, {} decisions, receipt {} for {} ({:.2?})",
            gathered.components.len(),
            gathered.decisions.len(),
            gathered.receipt.state,
            &gathered.receipt.sha[..gathered.receipt.sha.len().min(12)],
            start.elapsed()
        );
        return;
    }

    // `progress [out.json] [--asof T] [--cache DIR] [--no-fetch]`: read the retained public
    // history of every public project repository and write the /progress/ dataset. Needs `gh`
    // and `git`. The existing file supplies the curated events and policy text; a snapshot that
    // does not reconcile is refused and the existing file is left as it was.
    if first_arg == "progress" {
        let args: Vec<String> = std::env::args().skip(2).collect();
        let mut out = None;
        let mut opts = collector::Options {
            output: "public/data/project-progress.json".into(),
            cache_dir: ".progress-cache".into(),
            asof: None,
            fetch: true,
        };
        let mut i = 0;
        while i < args.len() {
            let value = |i: usize| {
                args.get(i + 1).cloned().unwrap_or_else(|| {
                    eprintln!("progress: {} needs a value", args[i]);
                    std::process::exit(2);
                })
            };
            match args[i].as_str() {
                "--asof" => { opts.asof = Some(value(i)); i += 1; }
                "--cache" => { opts.cache_dir = value(i).into(); i += 1; }
                "--no-fetch" => opts.fetch = false,
                other if !other.starts_with("--") && out.is_none() => out = Some(other.to_string()),
                other => {
                    eprintln!("progress: unknown argument {other}");
                    std::process::exit(2);
                }
            }
            i += 1;
        }
        if let Some(out) = out {
            opts.output = out.into();
        }
        match collector::collect(&opts) {
            Ok(r) => println!(
                "  [PROGRESS] {}: {} public repositories: {} source lines, {} revisions, {} PRs, {} CI runs. Cutoff {} ({} GitHub API calls, {:.2?})",
                opts.output.display(), r.repos, r.loc, r.commits, r.prs, r.ci, r.asof, r.api_calls, start.elapsed()
            ),
            Err(e) => {
                eprintln!("Progress refresh failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let (mode, target_dir) = if first_arg == "build" || first_arg == "verify" {
        (first_arg, std::env::args().nth(2).unwrap_or_else(|| "dist-rust".to_string()))
    } else {
        ("verify".to_string(), first_arg)
    };
    let facts_file = std::env::args().nth(3).unwrap_or_else(|| "facts.json".to_string());

    let dist = Path::new(&target_dir);

    println!("============================================================");
    println!("  AIEN OS Native Static Site Builder & Integrity Verifier   ");
    println!("  Engine: Pure Rust & Maud | Target: {}", dist.display());
    println!("  Mode: {}", mode);
    println!("============================================================");

    if mode == "build" {
        build_pure_rust_site(dist, Path::new(&facts_file));
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
