mod collector;
mod components;
mod facts;
mod ledger_public;
mod layouts;
mod seo;

use std::fs;
use std::io::Write;
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

/// The zh-CN progress mirror is a generated translation of the old page,
/// which inlined the full dataset. The English builder now fetches the
/// dataset; the mirror still carries the dead copy in its source. Strip it
/// after copying so the deployed page has one data path and one copy.
fn strip_legacy_progress_dataset(dist: &Path) {
    let path = dist.join("zh-CN/progress/index.html");
    let Ok(html) = fs::read_to_string(&path) else { return; };
    let Some(start) = html.find("<script id=\"ag-data\"") else { return; };
    let Some(relative_end) = html[start..].find("</script>") else { return; };
    let end = start + relative_end + "</script>".len();
    let mut out = String::with_capacity(html.len());
    out.push_str(&html[..start]);
    out.push_str(&html[end..]);
    fs::write(&path, out).expect("strip legacy progress dataset");
    println!("  [ASSETS] Removed the legacy inline progress dataset from the zh-CN mirror");
}

fn build_pure_rust_site(dist: &Path, facts_path: &Path) {
    if let Ok(ledger) = fs::read_to_string("public/data/now.tsv") {
        ledger_public::check_public(&ledger).expect("refusing to build with an unprojected public ledger");
    } else if Path::new("public/data/now.tsv").exists() {
        panic!("refusing unreadable public ledger");
    }
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
        strip_legacy_progress_dataset(dist);
    }

    // Canonical project state for machines: rendered from the same
    // source as the homepage status section (components::status), so
    // the page, the JSON, and llms.txt can never drift apart.
    fs::write(
        dist.join("status.json"),
        components::status::status_json(),
    )
    .expect("Failed to write status.json");
    // The machines and speeds data, published next to the page that renders it.
    fs::write(dist.join("machines.json"), components::machines::RAW)
        .expect("Failed to write machines.json");
    println!("  [STATE] Wrote status.json from the canonical layer states");

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

    // /start/: the newcomer route through the project. It carries no status
    // figures of its own; it links to the canonical implementation list,
    // evidence record and release record, and names the cross-project
    // boundaries.
    let start_page = render_page_layout(
        components::start::TITLE,
        components::start::DESCRIPTION,
        components::start::URL,
        "website",
        html! {},
        components::start::render_start_page(),
    );
    fs::create_dir_all(dist.join("start")).expect("Failed to create start dir");
    fs::write(dist.join("start/index.html"), start_page.into_string())
        .expect("Failed to write start/index.html");
    println!("  [PAGE] Emitted /start/ (index.html)");

    // Public project measurements, collected before compilation in CI.
    let progress = render_page_layout(
        "Project progress | AienOS",
        "Explore the retained AIEN project history: codebase growth, revisions, test footprint and verification activity, refreshed hourly from public GitHub sources.",
        "https://www.aienos.com/progress/", "website",
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
        "Blog | AienOS",
        "Research notes on building AIEN V2: what shipped, what the evidence says, and what remains an open question.",
        "https://www.aienos.com/blog/",
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
        &format!("{} | AienOS Blog", components::blog::POST_TITLE),
        components::blog::POST_DESCRIPTION,
        &format!("https://www.aienos.com/blog/{}/", components::blog::POST_SLUG),
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
        "Research status | AienOS",
        "Where the AIEN research stands: the qualification ladder, every experiment with its verdict, receipt and limits, and what is built versus still a hypothesis.",
        "https://www.aienos.com/research/status/",
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
    // /dash/: the one-screen dashboard, from accounts.tsv and routing.tsv on data/now.
    let dash = components::dash::load();
    let dash_page = render_page_layout(
        components::dash::TITLE,
        components::dash::DESCRIPTION,
        components::dash::URL,
        "website",
        html! { link rel="stylesheet" href="/assets/dash.css"; },
        components::dash::render_dash_page(&dash, &now),
    );
    fs::create_dir_all(dist.join("dash")).expect("Failed to create dash dir");
    fs::write(dist.join("dash/index.html"), dash_page.into_string()).expect("Failed to write dash/index.html");
    println!("  [PAGE] Emitted /dash/ ({} quota accounts)", dash.quota.accounts.len());
    let hive_page = render_page_layout(
        components::hive::TITLE,
        components::hive::DESCRIPTION,
        components::hive::URL,
        "website",
        html! {
            link rel="stylesheet" href="/assets/hive.css";
            script defer src="/js/hive.js" {}
        },
        components::hive::render_hive_page(),
    );
    fs::create_dir_all(dist.join("hive")).expect("Create Hive directory");
    fs::write(dist.join("hive/index.html"), hive_page.into_string()).expect("Write Hive page");
    let learn_page = render_page_layout(
        components::learn::TITLE,
        components::learn::DESCRIPTION,
        components::learn::URL,
        "website",
        html! { link rel="stylesheet" href="/assets/learn.css"; },
        components::learn::render_learn_page(),
    );
    fs::create_dir_all(dist.join("learn")).expect("Create learning directory");
    fs::write(dist.join("learn/index.html"), learn_page.into_string()).expect("Write learning page");
    // /engineer/: About the engineer, drakestapleton.com merged into this site.
    components::engineer::emit(dist, &facts);
    // /agents/: every public repository with its agent instructions, from
    // public/data/agent-repos.json (refreshed by `site-builder agents`).
    let agent_repos = components::agents::Repos::load(Path::new(components::agents::DATA_PATH))
        .unwrap_or_else(|e| panic!("agents: {e}"));
    let agents_page = render_page_layout(
        components::agents::TITLE,
        components::agents::DESCRIPTION,
        components::agents::URL,
        "website",
        html! {},
        components::agents::render_agents_page(&agent_repos),
    );
    fs::create_dir_all(dist.join("agents")).expect("Failed to create agents dir");
    fs::write(dist.join("agents/index.html"), agents_page.into_string()).expect("Failed to write agents/index.html");
    println!("  [PAGE] Emitted /agents/ ({} repositories)", agent_repos.listed().len());
    // /omega/: what Omega is, from data/omega.json (the omega repository's
    // docs/OMEGA.md), published next to it as /omega.json.
    let omega_page = render_page_layout(
        components::omega::TITLE,
        components::omega::DESCRIPTION,
        components::omega::URL,
        "website",
        html! { link rel="stylesheet" href="/assets/omega.css"; },
        components::omega::render_omega_page(),
    );
    fs::create_dir_all(dist.join("omega")).expect("Failed to create omega dir");
    fs::write(dist.join("omega/index.html"), omega_page.into_string()).expect("Failed to write omega/index.html");
    fs::write(dist.join("omega.json"), components::omega::RAW).expect("Failed to write omega.json");
    println!("  [PAGE] Emitted /omega/ and /omega.json");

    // /cudaplusplus/: CUDA++, the GPU half of Omega, from
    // data/cudaplusplus.json, published next to it as /cudaplusplus.json.
    let cudapp_page = render_page_layout(
        components::cudaplusplus::TITLE,
        components::cudaplusplus::DESCRIPTION,
        components::cudaplusplus::URL,
        "website",
        html! { link rel="stylesheet" href="/assets/omega.css"; link rel="stylesheet" href="/assets/cudaplusplus.css"; },
        components::cudaplusplus::render_cudaplusplus_page(),
    );
    fs::create_dir_all(dist.join("cudaplusplus")).expect("Failed to create cudaplusplus dir");
    fs::write(dist.join("cudaplusplus/index.html"), cudapp_page.into_string()).expect("Failed to write cudaplusplus/index.html");
    fs::write(dist.join("cudaplusplus.json"), components::cudaplusplus::RAW).expect("Failed to write cudaplusplus.json");
    println!("  [PAGE] Emitted /cudaplusplus/ and /cudaplusplus.json");

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
        (
            "owned",
            components::owned::TITLE,
            components::owned::DESCRIPTION,
            components::owned::URL,
            components::owned::render_owned(),
        ),
    ] {
        let page = render_page_layout(title, description, url, "article", html! {}, body);
        let page_dir = dist.join(slug);
        fs::create_dir_all(&page_dir).expect("Failed to create narrative page dir");
        fs::write(page_dir.join("index.html"), page.into_string())
            .expect("Failed to write narrative page index.html");
        println!("  [PAGE] Emitted /{}/ (index.html)", slug);
    }

    // Glossary: the terms of the project, each linked to where it is used.
    let glossary = render_page_layout(
        components::glossary::TITLE,
        components::glossary::DESCRIPTION,
        components::glossary::URL,
        "website",
        html! {},
        components::glossary::render_glossary(),
    );
    fs::create_dir_all(dist.join("glossary")).expect("Failed to create glossary dir");
    fs::write(dist.join("glossary/index.html"), glossary.into_string())
        .expect("Failed to write glossary/index.html");
    println!("  [PAGE] Emitted /glossary/ (index.html)");

    let privacy = render_page_layout(
        "Privacy & saying hello to Allen | AienOS",
        "Read without tracking. Saying hello to Allen consents to X pixel rfrlz; No refuses it.",
        "https://www.aienos.com/privacy/",
        "website",
        html! {},
        components::allen_intro::render_privacy(),
    );
    fs::create_dir_all(dist.join("privacy")).expect("Create privacy directory");
    fs::write(dist.join("privacy/index.html"), privacy.into_string()).expect("Write privacy page");

    // Where I am and how to reach me: X, email, Allen and GitHub.
    let contact = render_page_layout(
        components::contact::TITLE,
        components::contact::DESCRIPTION,
        components::contact::URL,
        "website",
        html! {},
        components::contact::render_contact(),
    );
    fs::create_dir_all(dist.join("contact")).expect("Create contact directory");
    fs::write(dist.join("contact/index.html"), contact.into_string()).expect("Write contact page");
    println!("  [PAGE] Emitted /contact/ (index.html)");

    // Section hubs: /experiments/, /evidence/, /philosophy/ (NARR-NAV)
    components::hubs::emit_hub_pages(dist);

    // The 404 page, served by GitHub Pages for any address that has no file.
    let not_found = render_page_layout(
        "Page not found | AienOS",
        "There is nothing at this address. Every page of AienOS is listed in llms.txt and the sitemap; the front page, the machine and the research are one link away.",
        "",
        "website",
        seo::head_404(),
        seo::render_404(),
    );
    fs::write(dist.join("404.html"), components::allen_intro::enrich(not_found.into_string())).expect("Failed to write 404.html");

    // Every page written above (generated or copied from public/) gets the same
    // head: www canonical, cards, hreflang, feeds, JSON-LD. Then the machine
    // surface is written from the pages that actually exist.
    let ctx = seo::BuildCtx::gather(&now, &facts.generated_at);
    let pages = seo::enrich(dist, &ctx);
    seo::write_machine_surface(dist, &pages, &now, &ctx, &components::status::llms_txt());
}

fn verify_site(dist: &Path) {
    let index_file = dist.join("index.html");
    let ledger_path = dist.join("data/now.tsv");
    if ledger_path.exists() {
        let ledger = fs::read_to_string(&ledger_path).expect("unreadable published ledger");
        ledger_public::check_public(&ledger).expect("refusing unprojected published ledger");
    }
    assert!(index_file.exists(), "Required index.html missing in {}", dist.display());

    let content = fs::read_to_string(&index_file).expect("Failed to read index.html");

    let zh_progress = fs::read_to_string(dist.join("zh-CN/progress/index.html"))
        .expect("Failed to read the zh-CN progress page");
    assert!(!zh_progress.contains("id=\"ag-data\""), "zh-CN progress page must not inline the dataset");

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
        "AienOS is an owned experimental machine for turning search into verified understanding.",
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
    // The whole machine surface: canonical host, JSON-LD, feeds, llms files, data index.
    seo::verify(dist);

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
    // /omega/ (components::omega): says what Omega is first, carries the
    // CUDA map, the live receipt and the bar, and is linked from the nav
    // and from the front page wherever Omega is named.
    let omega_page = dist.join("omega/index.html");
    assert!(omega_page.exists(), "Missing omega/index.html");
    assert!(dist.join("omega.json").exists(), "Missing omega.json");
    assert!(dist.join("assets/omega.css").exists(), "Missing assets/omega.css");
    {
        let o = fs::read_to_string(&omega_page).expect("read omega page");
        for phrase in [
            "Omega is AIEN&#39;s language for all computing, GPU to CPU.",
            "On the GPU, that makes Omega the replacement for CUDA.",
            "id=\"cuda-map\"",
            "id=\"today\"",
            "id=\"not-yet\"",
            "id=\"bar\"",
            "evidence/WHAT-OMEGA-IS-20261021/live-serve-no-cuda.txt",
        ] {
            assert!(o.contains(phrase) || o.contains(&phrase.replace("&#39;", "'")), "Missing on /omega/: {phrase}");
        }
        assert!(content.contains("href=\"/omega/\""), "The front page must link /omega/ where it names Omega");
    }
    // /cudaplusplus/ (components::cudaplusplus): CUDA++ with the plus
    // signs, its four sections, linked from the nav, the front page and /omega/.
    let cudapp_page = dist.join("cudaplusplus/index.html");
    assert!(cudapp_page.exists(), "Missing cudaplusplus/index.html");
    assert!(dist.join("cudaplusplus.json").exists(), "Missing cudaplusplus.json");
    assert!(dist.join("assets/cudaplusplus.css").exists(), "Missing assets/cudaplusplus.css");
    {
        let c = fs::read_to_string(&cudapp_page).expect("read cudaplusplus page");
        for phrase in [
            "CUDA++ is the GPU half of Omega",
            "Less computation, out of the big data centers.",
            "id=\"path\"",
            "id=\"how\"",
            "id=\"pain-points\"",
            "id=\"swap\"",
            "https://github.com/aien-dev/CUDApp",
        ] {
            assert!(c.contains(phrase), "Missing on /cudaplusplus/: {phrase}");
        }
        assert!(content.contains("href=\"/cudaplusplus/\""), "The front page must link /cudaplusplus/");
        let o = fs::read_to_string(&omega_page).expect("read omega page");
        assert!(o.contains("href=\"/cudaplusplus/\""), "/omega/ must link /cudaplusplus/");
    }
    for page in [&blog_index, &blog_post, &research_status, &now_page, &omega_page, &cudapp_page] {
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
    // /owned/ is a position page: no status badges, the thesis and signature present, no dashes.
    let owned = fs::read_to_string(dist.join("owned/index.html")).expect("Missing owned/index.html");
    assert!(owned.contains(components::owned::THESIS), "Missing thesis in owned/index.html");
    assert!(owned.contains(components::owned::SIGNATURE_LINE), "Missing signature in owned/index.html");
    assert!(owned.contains("og:title"), "Missing og:title in owned/index.html");
    assert!(!owned.contains('\u{2014}') && !owned.contains('\u{2013}'), "Forbidden dash in owned/index.html");
    assert!(!owned.contains("rs-badge"), "Status badge on the position page owned/index.html");
    let machine = fs::read_to_string(dist.join("machine/index.html")).expect("Failed to read machine page");
    assert!(machine.contains(components::narrative::SLOGAN), "Missing slogan in machine/index.html");
    assert!(machine.contains(components::narrative::TURING_FORMULA), "Missing Turing formula in machine/index.html");
    assert!(machine.contains("id=\"real-machines\""), "Missing real-machines section in machine/index.html");
    assert!(machine.find("id=\"real-machines\"") < machine.find("id=\"loop\""), "Real machines must come before the loop in machine/index.html");
    assert!(machine.contains("id=\"research-ladder\""), "Missing research-ladder section in machine/index.html");
    assert!(machine.contains("class=\"rs-ladder\""), "Missing research ladder markup in machine/index.html");
    assert!(machine.contains("class=\"rs-impl\""), "Missing implementation list in machine/index.html");

    // The newcomer route must reach the canonical status, evidence,
    // reproduction and cross-project surfaces.
    let start = fs::read_to_string(dist.join("start/index.html")).expect("Failed to read start/index.html");
    for needle in [
        "/machine/#today",
        "/research/status/#implementation",
        "/evidence/",
        "/AGENTS.md",
        "/engineer/",
        "https://interplane.io/",
    ] {
        assert!(start.contains(needle), "/start/ is missing {needle}");
    }

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
    // About the engineer: every page written, filled and self-contained.
    components::engineer::verify(dist);
    // Drake writes alone: no we, us, our, ours or ourselves in rendered copy.
    components::voice::verify_singular_voice(dist);
    // Every page shows Follow me on X in its header and the contact line in its footer.
    components::contact::verify(dist);

    println!("------------------------------------------------------------");
    println!("  Verified index.html ({} bytes)", content.len());
    println!("  Every box links to GitHub: {boxes_total} boxes across {} pages", GENERATED_PAGES.len());
    println!("  Integrity & Unslop Invariants: PASS");
}

/// Every page this builder generates, with the least number of boxes it
/// must carry. /now/ has no floor because the ledger may be absent.
const GENERATED_PAGES: &[(&str, usize)] = &[
    ("index.html", 30),
    ("start/index.html", 0),
    ("privacy/index.html", 0),
    ("contact/index.html", 0),
    ("machine/index.html", 15),
    ("discovery/index.html", 0),
    ("experiments/index.html", 15),
    ("evidence/index.html", 7),
    ("philosophy/index.html", 4),
    ("owned/index.html", 7),
    ("research/status/index.html", 40),
    ("now/index.html", 0),
    ("dash/index.html", 0),
    ("hive/index.html", 0),
    ("learn/index.html", 0),
    ("omega/index.html", 30),
    ("cudaplusplus/index.html", 12),
    ("progress/index.html", 0),
    ("agents/index.html", 10),
    ("blog/index.html", 2),
    ("blog/aien-v3-research-plan/index.html", 0),
];

fn main() {
    let start = Instant::now();
    let first_arg = std::env::args().nth(1).unwrap_or_else(|| "build".to_string());
    if first_arg == "ledger-project" || first_arg == "ledger-check" {
        let path = std::env::args().nth(2).filter(|_| std::env::args().nth(3).is_none())
            .unwrap_or_else(|| { eprintln!("{first_arg}: expected one TSV path"); std::process::exit(2) });
        let input = fs::read_to_string(&path).unwrap_or_else(|_| {
            eprintln!("{first_arg}: unreadable ledger");
            std::process::exit(1);
        });
        let result = if first_arg == "ledger-project" {
            ledger_public::project(&input)
        } else {
            ledger_public::check_public(&input).map(|()| String::new())
        };
        let output = result.unwrap_or_else(|e| {
            eprintln!("{first_arg}: {e}");
            std::process::exit(1);
        });
        if first_arg == "ledger-project" {
            std::io::stdout().write_all(output.as_bytes()).expect("cannot write projected ledger");
        }
        return;
    }

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

    // `agents [out.json]`: read every public repository of the organization and
    // the agent brief it carries (AGENTS.md or CLAUDE.md). Read-only, needs `gh`.
    if first_arg == "agents" {
        let out = std::env::args().nth(2).unwrap_or_else(|| components::agents::DATA_PATH.to_string());
        let gathered = components::agents::gather().unwrap_or_else(|e| {
            eprintln!("agents: {e}");
            std::process::exit(1);
        });
        gathered.save(Path::new(&out)).unwrap_or_else(|e| {
            eprintln!("agents: cannot write {out}: {e}");
            std::process::exit(1);
        });
        println!(
            "  [AGENTS] {out}: {} public repositories, {} with a brief ({:.2?})",
            gathered.repos.len(),
            gathered.repos.iter().filter(|r| r.brief.is_some()).count(),
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
    println!("  AienOS Native Static Site Builder & Integrity Verifier   ");
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
        let sample = "AienOS: The GPU-Native Neural Operating Environment.";
        assert!(!sample.contains('\u{2014}'), "Must not contain em dash");
        assert!(!sample.contains('\u{2013}'), "Must not contain en dash");
    }

    #[test]
    fn turing_sliders_have_accessible_names() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for rel in ["public/turing/index.html", "public/zh-CN/turing/index.html"] {
            let page = fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("cannot read {rel}"));
            for id in ["r-gain", "r-cost", "r-base"] {
                assert!(page.contains(&format!("for=\"{id}\"")), "{rel}: no label for {id}");
                let pos = page.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("{rel}: missing id {id}"));
                let tag_start = page[..pos].rfind("<input").unwrap_or_else(|| panic!("{rel}: {id} is not an input"));
                let tag_end = page[tag_start..].find('>').unwrap_or(0) + tag_start;
                let tag = &page[tag_start..=tag_end];
                assert!(tag.contains("aria-label="), "{rel}: input {id} has no accessible name");
            }
            assert!(!page.contains("<output"), "{rel}: <output> inside a label is invalid");
        }
    }

    #[test]
    fn test_layout_rendering() {
        let markup = render_base_layout(html! { p { "Test" } });
        let s = markup.into_string();
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains("<div id=\"root\""));
        assert!(s.contains("AienOS"));
    }
}
