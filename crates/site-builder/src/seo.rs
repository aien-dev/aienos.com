//! Everything that makes a page findable, by people and by machines, in one
//! place: the canonical host, the social card tags, hreflang alternates,
//! JSON-LD structured data, the sitemap walked from what was actually
//! written, robots.txt, the Atom feeds, llms.txt and llms-full.txt, the
//! data index, humans.txt and the 404 page.
//!
//! The pass runs over `dist/` after every page is written, so generated
//! pages and hand-written pages under `public/` get the same head. A page
//! that is not in [`PAGES`] fails the build: nothing is published unlisted.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use maud::{html, Markup, PreEscaped};
use serde_json::{json, Value};

use crate::components::now::{self, Now};

pub const HOST: &str = "https://www.aienos.com";
pub const SITE_NAME: &str = "AienOS";
pub const SITE_CLAIM: &str = "Persistent machine intelligence on a machine you own";
pub const ORG_NAME: &str = "AIEN (aien-dev)";
pub const ORG_URL: &str = "https://github.com/aien-dev";
pub const ORG_EMAIL: &str = "aien@aienos.com";
/// Drake's X account, from his GitHub profile and /engineer/.
pub const X_HANDLE: &str = "@mdrakestapleton";
pub const X_URL: &str = "https://x.com/mdrakestapleton";
pub const AUTHOR: &str = "M. Drake Stapleton";
pub const AUTHOR_URL: &str = "https://www.aienos.com/engineer/";
pub const AUTHOR_GITHUB: &str = "https://github.com/drakestapleton";
pub const LICENSE_NAME: &str = "Apache-2.0 WITH LLVM-exception";
pub const LICENSE_URL: &str = "https://www.aienos.com/licensing/";
/// Bump whenever scripts/og-cards.py redraws the cards: X and other link
/// unfurlers cache a card image by its URL, so a new query string makes them
/// fetch the new picture instead of showing the old one.
pub const OG_IMAGE_VERSION: &str = "20261025b";

/// What a page is, for structured data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The front page.
    Home,
    /// A plain page.
    Page,
    /// A hub or index listing other pages.
    Collection,
    /// A dated article (blog post, position paper).
    Article,
    /// A research preprint with a PDF.
    Paper,
    /// A page rebuilt from live data.
    Live,
}

/// The sections of llms.txt, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Start,
    Machine,
    Turing,
    Evidence,
    Philosophy,
    Blog,
    Engineer,
    Optional,
}

/// One English page. `published` is the day the page first went up
/// (from the repository history); `doi` is the Zenodo DOI where one exists.
pub struct PageInfo {
    pub path: &'static str,
    pub name: &'static str,
    pub what: &'static str,
    pub section: Section,
    pub kind: Kind,
    pub image: &'static str,
    pub published: &'static str,
    pub doi: Option<&'static str>,
}

/// Every English page of the site. The build fails if dist holds a page
/// that is not here, or if a page here was not written.
pub const PAGES: &[PageInfo] = &[
    PageInfo { path: "/", name: "AienOS", what: "The front page: the loop, the five real machines and their measured speeds beside the physical ceiling, the evidence cards, the current status of every layer.", section: Section::Machine, kind: Kind::Home, image: "/og.png", published: "2026-09-23", doi: None },
    PageInfo { path: "/start/", name: "Start here", what: "The shortest route through AienOS: what it is, what exists today, how to check the evidence, how to reproduce or contribute, and where the author and Interplane fit.", section: Section::Start, kind: Kind::Page, image: "/og.png", published: "2026-10-24", doi: None },
    PageInfo { path: "/contact/", name: "Where I am and how to reach me", what: "How to reach Drake: X @mdrakestapleton, email drake@aienos.com, Allen at /allen/ (who can pass a message on or put you through when Drake is online) and the aien-dev repositories on GitHub.", section: Section::Start, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/support/", name: "Support my open source work", what: "Help me keep building free and open source software through X Money. US-only eligibility, the exact recipient and the steps to send support.", section: Section::Start, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/privacy/", name: "Privacy & saying hello to Allen", what: "Your choice about the consent-only X pixel, what X receives, what I keep and how to refuse.", section: Section::Optional, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/machine/", name: "The Machine", what: "One machine across five physical bodies: the closed epistemic loop, the Turing as its accounting layer, and an honest list of what exists.", section: Section::Machine, kind: Kind::Article, image: "/og-machine.png", published: "2026-10-01", doi: None },
    PageInfo { path: "/omega/", name: "What Omega is", what: "Omega is AIEN's language for all computing, GPU to CPU; on the GPU it is the replacement for CUDA. The CUDA stack piece by piece with status, the receipt for the live server running with zero CUDA libraries, what is not true yet, and the bar C1 to C6.", section: Section::Machine, kind: Kind::Page, image: "/og-machine.png", published: "2026-10-21", doi: None },
    PageInfo { path: "/cudaplusplus/", name: "CUDA++", what: "CUDA++ is the GPU half of Omega: everything CUDA gives you plus the motor, for the NVIDIA GPUs people already own, with no NVIDIA user-space software (Apache-2.0 with LLVM exception, aien-dev/CUDApp). What it is for, the path from program to chip with whose code runs at each step, how I did it, CUDA's pain points and the swap down to the kernel driver.", section: Section::Machine, kind: Kind::Page, image: "/og-machine.png", published: "2026-10-22", doi: None },
    PageInfo { path: "/now/", name: "Now", what: "The live coordination ledger: what every agent is doing right now, what is settled with evidence, what waits on the operator. Rebuilt whenever the ledger changes.", section: Section::Machine, kind: Kind::Live, image: "/og-now.png", published: "2026-10-19", doi: None },
    PageInfo { path: "/dash/", name: "Dashboard", what: "One screen: model quota left on every account the agents run on, the Spark's pulse, open work, and which model each role uses. Rebuilt with the ledger.", section: Section::Machine, kind: Kind::Live, image: "/og-now.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/hive/", name: "Hive", what: "Watch Drake build with coding agents: live missions, nested subagents, exact models and public conversations, with source freshness and failures visible.", section: Section::Start, kind: Kind::Live, image: "/og-now.png", published: "2026-10-25", doi: None },
    PageInfo { path: "/alpha/", name: "Sovereign Intelligence", what: "Sovereign Intelligence, built on AienOS. The Omega installer, hardware and evidence boundaries, source and a reversible way back. The local Allen experience is coming in the next update.", section: Section::Start, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/learn/", name: "Build with AI", what: "A welcoming, practical guide to coding with AI: structure a task, delegate bounded work, inspect changes, verify behavior and learn from mistakes in the live Hive.", section: Section::Start, kind: Kind::Page, image: "/og-now.png", published: "2026-10-25", doi: None },
    PageInfo { path: "/agents/", name: "Agents", what: "Point your own coding agent at any public AIEN repository: the shared rules every AIEN agent follows and each repository's own AGENTS.md, listed from GitHub and refreshed hourly.", section: Section::Start, kind: Kind::Page, image: "/og.png", published: "2026-10-25", doi: None },
    PageInfo { path: "/progress/", name: "Project progress", what: "Source lines, revisions, test footprint and verification activity across the public repositories, refreshed hourly from GitHub.", section: Section::Machine, kind: Kind::Live, image: "/og.png", published: "2026-10-01", doi: None },
    PageInfo { path: "/turing/", name: "The Turing: an interactive field guide", what: "What the Turing measures (net held-out explanatory compression), with games, live calculations and the theories behind it.", section: Section::Turing, kind: Kind::Article, image: "/og-turing.png", published: "2026-09-30", doi: None },
    PageInfo { path: "/research/", name: "AIEN Research", what: "The index of preprints and public evidence.", section: Section::Turing, kind: Kind::Collection, image: "/og-research.png", published: "2026-09-30", doi: None },
    PageInfo { path: "/research/computing-machinery-and-understanding/", name: "Computing Machinery and Understanding: From the Imitation Game to the Turing", what: "The formal preprint defining the Turing as an operational measure of generalized explanatory compression. DOI 10.5281/zenodo.23437498.", section: Section::Turing, kind: Kind::Paper, image: "/og-turing.png", published: "2026-09-30", doi: Some("10.5281/zenodo.23437498") },
    PageInfo { path: "/research/computing-machinery-and-understanding-emergence/", name: "Computing Machinery and Understanding: The Turing as a Measure Emerging from AIEN", what: "The companion engineering paper: how the Turing emerged from building AIEN. DOI 10.5281/zenodo.23437568.", section: Section::Turing, kind: Kind::Paper, image: "/og-turing.png", published: "2026-09-30", doi: Some("10.5281/zenodo.23437568") },
    PageInfo { path: "/research/the-turing-brief/", name: "The Turing: A Unit of Verified Explanatory Gain", what: "The three-page plain-language brief: what the Turing measures, what it does not, and what would prove it wrong.", section: Section::Turing, kind: Kind::Paper, image: "/og-turing.png", published: "2026-10-02", doi: None },
    PageInfo { path: "/research/turing-discovery-gap-conjecture/", name: "The Turing Discovery Gap Conjecture", what: "Available versus accessible explanatory value: worlds with compact explanations that no efficient learner can reach.", section: Section::Turing, kind: Kind::Paper, image: "/og-research.png", published: "2026-10-02", doi: None },
    PageInfo { path: "/research/crumb-doctrine/", name: "The Crumb Doctrine", what: "No receipt, no crumb: the epistemic constitution of the machine.", section: Section::Turing, kind: Kind::Paper, image: "/og-research.png", published: "2026-10-02", doi: None },
    PageInfo { path: "/research/omega-language/", name: "Omega: A Programming Language Built for Verified Programs", what: "Semantic identity, evidence-bearing dependencies, and a foreign-code boundary that never lets unverified work inside.", section: Section::Turing, kind: Kind::Paper, image: "/og-research.png", published: "2026-10-02", doi: None },
    PageInfo { path: "/research/atemporal-aien-postulate/", name: "The Atemporal AIEN Postulate", what: "Causal semantics without clocks: an AIEN computation is defined by what causally depends on what.", section: Section::Turing, kind: Kind::Paper, image: "/og-research.png", published: "2026-10-02", doi: None },
    PageInfo { path: "/research/the-aien-papers/", name: "The AIEN Papers: The Complete Collection", what: "The Turing unit, the evidence doctrine, the language design and the philosophy in one document.", section: Section::Turing, kind: Kind::Paper, image: "/og-research.png", published: "2026-10-02", doi: None },
    PageInfo { path: "/experiments/", name: "Experiments", what: "Every experiment on the research ladder, failures included, rendered from one canonical status source.", section: Section::Evidence, kind: Kind::Collection, image: "/og-experiments.png", published: "2026-10-10", doi: None },
    PageInfo { path: "/discovery/", name: "Scientific Discovery", what: "Physics Zero, Dirac, Zeta and supersymmetry as examinations of representation discovery; the machine is never given the named answer.", section: Section::Evidence, kind: Kind::Article, image: "/og-discovery.png", published: "2026-10-10", doi: None },
    PageInfo { path: "/evidence/", name: "Evidence", what: "What counts as evidence, which source wins when two disagree, and where the receipts, frozen profiles and status records live.", section: Section::Evidence, kind: Kind::Collection, image: "/og-evidence.png", published: "2026-10-10", doi: None },
    PageInfo { path: "/research/status/", name: "Research status", what: "The qualification ladder, every experiment with its verdict, receipt and limits, and what is built versus still a hypothesis.", section: Section::Evidence, kind: Kind::Live, image: "/og-evidence.png", published: "2026-10-05", doi: None },
    PageInfo { path: "/philosophy/", name: "Philosophy", what: "The position papers behind AIEN. These argue a stance and are not experimental results.", section: Section::Philosophy, kind: Kind::Collection, image: "/og-philosophy.png", published: "2026-10-10", doi: None },
    PageInfo { path: "/research/the-stapleton-doctrine/", name: "The Stapleton Doctrine", what: "A personal philosophy of AI architecture and programming.", section: Section::Philosophy, kind: Kind::Paper, image: "/og-philosophy.png", published: "2026-09-30", doi: None },
    PageInfo { path: "/post-llm-case/", name: "The Post-LLM Case", what: "The case for a different learning machine: programs proposed, verification deciding, knowledge kept as inspectable programs.", section: Section::Philosophy, kind: Kind::Article, image: "/og-philosophy.png", published: "2026-09-27", doi: None },
    PageInfo { path: "/owned/", name: "Intelligence should be owned", what: "Drake Stapleton's manifesto: language models are a common inheritance, running one yourself is a right and a responsibility, and what AIEN does about it with receipts.", section: Section::Philosophy, kind: Kind::Article, image: "/og-philosophy.png", published: "2026-10-19", doi: None },
    PageInfo { path: "/call-to-humanity/", name: "A Call to Humanity", what: "An open letter on breaking the CUDA stranglehold on the hardware market: a clean-room replacement for the CUDA user stack, built in the open, with public receipts. DOI 10.5281/zenodo.23614947.", section: Section::Philosophy, kind: Kind::Article, image: "/og-philosophy.png", published: "2026-10-24", doi: Some("10.5281/zenodo.23614947") },
    PageInfo { path: "/blog/", name: "Blog", what: "Research notes on building AIEN: what shipped, what the evidence says, what remains open.", section: Section::Blog, kind: Kind::Collection, image: "/og-blog.png", published: "2026-09-27", doi: None },
    PageInfo { path: "/blog/aien-v3-research-plan/", name: "The AIEN V3 Research Plan", what: "The ratified V3 research plan, the Substrate Edition, with six falsifiable hypotheses.", section: Section::Blog, kind: Kind::Article, image: "/og-blog.png", published: "2026-09-27", doi: None },
    // About the engineer: drakestapleton.com, merged here on 2026-10-26 (components::engineer, public/data/engineer.json).
    PageInfo { path: "/engineer/", name: "About the engineer", what: "The personal project record of Drake Stapleton, Freedom Fighter, AI Architect & Operator: model training, research, Atlas, and what my generation can build with AI.", section: Section::Engineer, kind: Kind::Collection, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/path/", name: "AI Architect Career Path", what: "Follow Drake Stapleton's career from industrial chemist to manufacturing engineer to AI Architect through a documented personal record.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/what-i-learned/", name: "What I Learned", what: "The experiences, production lessons, and disciplined returns that shaped Drake Stapleton's work as an AI operator.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/philosophy/", name: "Philosophy", what: "How Drake Stapleton approaches projects, duties, difficult things, and setbacks. The working philosophy behind everything he builds: there is always a way, and giving up is not one of the options.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/software/", name: "AI Orchestration & Architecture", what: "Drake Stapleton's AI orchestration frameworks, architectural patterns, operational methods, and verifiable systems history.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/evidence/", name: "Canonical Evidence Hub & Verified Claims", what: "Inspect the canonical evidence hub, structured claims index, physical Grace Blackwell silicon telemetry, and historical audit packages.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/atlas/", name: "Why Atlas Exists", what: "Explore Atlas, Drake Stapleton's personal R&D program in accountable AI architecture, continuity, model training, and orchestration.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/scholar/", name: "Scholar: Academic Work", what: "The published record: a master's thesis and a peer-reviewed semiconductor nanorod paper from Western Kentucky University, with links to the current AIEN research.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/interest/", name: "Start a Conversation", what: "Leave Drake Stapleton a note about a question, an idea, the documented journey, or research that advances what my generation can build with AI.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/donate/", name: "Support my work", what: "Help me keep building free and open source software through X Money, with US-only eligibility and the exact steps.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/glossary/", name: "Glossary", what: "Definitions of AIEN, AienOS, Omega, CUDA++, Atlas, Symphony, AEGIS, and verification terms used in this project record.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/symphony/", name: "Multi-Model AI Orchestration", what: "Atlas Symphony is Drake Stapleton's operator-led process for coordinating multiple AI models through bounded work lanes and evidence-bearing reviews.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/symphony/first/", name: "First Multi-Model Symphony Run", what: "Review the first documented Atlas Symphony run, its operator, participating AI runtimes, work lanes, and independent reviewers.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/symphony/workflow/", name: "AI Orchestration Workflow", what: "Explore the Atlas Symphony command tree and operator cycle for bounded multi-model AI orchestration.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/symphony/map/", name: "AI Orchestration Map", what: "Explore a six-track operator map of Atlas development and multi-model orchestration from June through August 2026.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/", name: "Works", what: "A public catalog of Drake Stapleton's shipped systems: Atlas Symphony, Atlas Harness, MobDev and Panel Arena, DefiantMob, BeltBrain, DHG Hive, the 2025 foundations, AIEN, CUDA++, AEGIS, and inference research.", section: Section::Engineer, kind: Kind::Collection, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/aegis/", name: "AEGIS", what: "AEGIS extends Atlas into on-prem cyber defense for authorized networks, with containment, evidence preservation, and a verified project evaluation.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/aien/", name: "AIEN", what: "AIEN is a sovereign computing stack built in the open on one machine: AienOS, Omega, the sovereign core runtime and the Spark verification pipeline, gathered into the Aienix core tree.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/atlas-harness/", name: "Atlas Harness", what: "Atlas Harness is the operating desk that turns prose rules into typed policy, clear work lanes, completion checks, source records, recovery, and evaluation.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/atlas-symphony/", name: "Atlas Symphony", what: "Atlas Symphony is the named process for exclusive lanes, a gatekeeper, cost-aware model routing, and an operator who still owns the final GO.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/beltbrain/", name: "BeltBrain", what: "BeltBrain is a local-first decision engine for planning, inventory intelligence, and execution on a durable system of record.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/chatgpt-vs-aien/", name: "Where Builders Start", what: "ChatGPT is where many builders start, me included. Aien builds on that start: it runs real work on my own machine, checks the results, and shows its receipts.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/cudaplusplus/", name: "CUDA++", what: "CUDA++ is the GPU half of Omega, its own MIT project: everything CUDA gives you plus the motor, with no NVIDIA user-space software, and how it was built.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/defiantmob/", name: "DefiantMob", what: "DefiantMob is production operator software for member operations, ledgers, training-desk automation, and payout tracking.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/dhg-hive/", name: "DHG Hive", what: "DHG Hive is the community platform and toolbox where the hive operating pattern took shape and later settled inside Atlas.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/endless-symphony/", name: "The Endless Symphony", what: "The Endless Symphony is my system for building software with AI agents. AI does the building, every claim earns its place with a check, the test is written before the fix, and everything is verified twice. The models change. The Symphony remains.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/foundations-2025/", name: "2025 Foundations", what: "The 2025 foundations cover the financial ledger, ParseSDS, CRM, meetings, and DMAIC systems that preceded model orchestration.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/mobdev-panel-arena/", name: "MobDev and Panel Arena", what: "MobDev turns community tasks into build briefs, and Panel Arena runs a multi-vendor review before implementation.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/research/", name: "Research", what: "What the AIEN research program measures today: Turing yield, verified understanding, and distance to the hardware ceiling, and why the earlier inference study was withdrawn.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/the-continuation-test/", name: "The Continuation Test", what: "A proposal for the evaluation of machine intelligence. Can a machine take up its position on the spectrum and keep moving? Includes the first measured run.", section: Section::Engineer, kind: Kind::Article, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/engineer/works/chatgpt-vs-aien/infographic/", name: "Same chat box, different machine", what: "An infographic that separates the layers: an LLM predicts language, ChatGPT wraps a model in a hosted product and is where many builders start, and Aien is being built as a sovereign, evidence-first system that leaves receipts.", section: Section::Engineer, kind: Kind::Page, image: "/og.png", published: "2026-10-26", doi: None },
    PageInfo { path: "/glossary/", name: "Glossary", what: "The terms AienOS uses in its own sense, each linked to the page where it does real work.", section: Section::Optional, kind: Kind::Page, image: "/og.png", published: "2026-10-23", doi: None },
    PageInfo { path: "/licensing/", name: "Licensing", what: "The one canonical licensing statement: Apache-2.0 WITH LLVM-exception for compiler and runtime repositories; plain Apache-2.0 for protocols and benchmarks.", section: Section::Optional, kind: Kind::Page, image: "/og.png", published: "2026-10-01", doi: None },
];

/// One machine-readable file: path, purpose, schema hint, how often it changes, media type.
pub struct DataFile {
    pub path: &'static str,
    pub purpose: &'static str,
    pub schema: &'static str,
    pub cadence: &'static str,
    pub media_type: &'static str,
}

pub const DATA_FILES: &[DataFile] = &[
    DataFile { path: "/data/index.json", purpose: "This list: every machine-readable file on the site with its purpose and cadence.", schema: "{ generated, license, files: [{ path, url, purpose, schema, cadence, media_type }] }", cadence: "every build", media_type: "application/json" },
    DataFile { path: "/llms.txt", purpose: "The site for AI readers: one summary and every page with a one-line description (llms.txt convention).", schema: "Markdown: H1, blockquote summary, H2 sections of links.", cadence: "every build", media_type: "text/markdown" },
    DataFile { path: "/llms-full.txt", purpose: "The full text of every page, concatenated, generated from the same sources as the HTML.", schema: "Markdown: one H1 per page with its source URL.", cadence: "every build", media_type: "text/markdown" },
    DataFile { path: "/now.json", purpose: "The live coordination ledger: every item with owner, status, done-when, evidence notes and timestamps.", schema: "{ built_utc, ledger_written_utc, items: [{ id, opened_utc, owner, owner_kind, status, item, done_when, notes[], updated_utc, stream, age_s }], pulse: [{ key, value }] }", cadence: "whenever the ledger changes, at most hourly", media_type: "application/json" },
    DataFile { path: "/now/feed.xml", purpose: "Atom feed of settled ledger lines (done or dropped) with their evidence, newest first.", schema: "Atom 1.0", cadence: "whenever the ledger changes, at most hourly", media_type: "application/atom+xml" },
    DataFile { path: "/machines.json", purpose: "The five real machines, what runs on each today, and every measured speed beside its physical ceiling, each with a receipt link.", schema: "{ as_of, source, judging, ceilings: [{ id, model, machine, tok_s, how, link, receipt, branch, on_main }], machines: [{ id, name, detail, role, today, evidence_date, evidence, link, branch, on_main, more[] }], speeds: [...] }", cadence: "when a new receipt is published", media_type: "application/json" },
    DataFile { path: "/omega.json", purpose: "What Omega is: its parts, every CUDA piece with Omega's replacement and status, the live no-CUDA receipt, what is not true yet, and the bar C1 to C6, each with its GitHub link.", schema: "{ source, claim, claim_rest[], true_today, not_true_yet, parts[], below, layers: [{ cuda, job, omega, status, note, links, box }], receipt, not_yet[], bar: [{ id, title, done_when, status, now, receipt, links }], beyond[] }; links are short forms (o: omega file, o/: omega folder, p: physics) expanded as on /omega/", cadence: "when a bar row or map row changes", media_type: "application/json" },
    DataFile { path: "/cudaplusplus.json", purpose: "CUDA++: what it is for, every step from program to chip with whose code runs there and its status, how I did it, CUDA's pain points and the swap, each with its GitHub link.", schema: "{ source, claim, claim_rest[], purpose_title, purpose, hops: [{ step, cuda, ours, status, note, ledger, links, box }], steps[], did_not[], pains: [{ title, text, status, ledger, command, receipt, links, box }], swaps[] }; links are short forms (cp: CUDA++ main, cp@: CUDA++ at a commit, om@: omega at a commit) expanded as on /cudaplusplus/", cadence: "when a step or row changes", media_type: "application/json" },
    DataFile { path: "/facts.json", purpose: "The public GitHub record read at build time: components, last merges, open PR counts, latest decision records, latest verification receipt.", schema: "{ generated_at, components: [{ key, name, repo, role, url, pushed_at, open_prs, last_merge }], decisions: [{ id, title, url }], receipt: { repo, sha, state, description, target_url, updated_at } }", cadence: "hourly", media_type: "application/json" },
    DataFile { path: "/status.json", purpose: "The status of every layer of the stack (IMPLEMENTED, QUALIFIED, EXPERIMENTAL, BLOCKED, PLANNED) with a note and a receipt.", schema: "{ project, updated, canonical, statuses[], layers: [{ name, status, note, receipt }] }", cadence: "when a layer changes state", media_type: "application/json" },
    DataFile { path: "/research-status.json", purpose: "The research ladder: every experiment with its verdict, receipt and limits; the one source for every status badge on the site.", schema: "{ ladder, experiments, implementations, ... } as rendered on /research/status/", cadence: "when an experiment settles", media_type: "application/json" },
    DataFile { path: "/data/project-progress.json", purpose: "Retained public history of every project repository: source lines, revisions, PRs, CI runs over time.", schema: "{ asof, repos: [...], series: [...], events: [...], policy }", cadence: "hourly", media_type: "application/json" },
    DataFile { path: "/data/now.tsv", purpose: "The line-preserving public ledger projection with private paths and identifiers withheld.", schema: "Columns: id, opened_utc, owner, status, item, done_when, evidence, updated_utc.", cadence: "whenever a safe ledger projection is accepted", media_type: "text/tab-separated-values" },
    DataFile { path: "/data/accounts.tsv", purpose: "Model quota left on every account the agents run on, as the Spark read it. Accounts are numbered; no email or account id is published.", schema: "checked, utc; then limit, provider, account, plan, window, label, percent used, resets utc, ok|exhausted.", cadence: "with the ledger, at most every 30 minutes on its own", media_type: "text/tab-separated-values" },
    DataFile { path: "/data/routing.tsv", purpose: "Which model each omp role, subagent and advisor seat uses on the Spark.", schema: "role|agent|seat, name, model selector.", cadence: "when the configuration changes", media_type: "text/tab-separated-values" },
    DataFile { path: "/feed.xml", purpose: "Atom feed of the blog posts and research papers, newest first.", schema: "Atom 1.0", cadence: "when a page is published or revised", media_type: "application/atom+xml" },
    DataFile { path: "/sitemap.xml", purpose: "Every page, both languages, with lastmod and hreflang alternates. Walked from the files actually written.", schema: "sitemaps.org 0.9 with xhtml:link alternates", cadence: "every build", media_type: "application/xml" },
    DataFile { path: "/sitemap.md", purpose: "The readable twin of sitemap.xml: every page grouped in H2 sections with a one-line description.", schema: "Markdown: H1, blockquote summary, H2 sections of links.", cadence: "every build", media_type: "text/markdown" },
    DataFile { path: "/AGENTS.md", purpose: "The repository brief for coding agents: installation, configuration, usage, conventions and approval boundaries.", schema: "Markdown: H1, Installation, Configuration, Usage, Conventions, Safety and approvals.", cadence: "when the build or conventions change", media_type: "text/markdown" },
    DataFile { path: "/agents/RULES.md", purpose: "The working rules every AIEN agent follows, public edition: read first by any agent working on an aien-dev repository, before that repository's own AGENTS.md.", schema: "Markdown: H1, blockquote scope, numbered H2 sections, Where to go next.", cadence: "when the rules change", media_type: "text/markdown" },
    DataFile { path: "/data/agent-repos.json", purpose: "Every public, unarchived aien-dev repository with its description, default branch and the agent brief it carries (AGENTS.md or CLAUDE.md).", schema: "{ generated_at, org, repos: [{ name, description, url, default_branch, fork, pushed_at, brief }] }; brief is null when the repository has none", cadence: "hourly", media_type: "application/json" },
    DataFile { path: "/index.md", purpose: "The front page as Markdown: the same content as the HTML, with frontmatter, the page body and a sitemap section. Every page has a mirror at the same path with .md.", schema: "Markdown: YAML frontmatter (title, description, last_updated, canonical_url), body, Sitemap section.", cadence: "every build", media_type: "text/markdown" },
    DataFile { path: "/research/CITATION.cff", purpose: "Citation metadata for the Turing preprints.", schema: "Citation File Format 1.2.0", cadence: "when a paper is revised", media_type: "text/yaml" },
    DataFile { path: "/research/aien-research-plan.md", purpose: "The ratified V3 research plan in Markdown.", schema: "Markdown", cadence: "when the plan is revised", media_type: "text/markdown" },
    DataFile { path: "/data/engineer.json", purpose: "About the engineer: every page of /engineer/ (drakestapleton.com, merged here) with its title, description, body and the old drakestapleton.com paths that redirect to it.", schema: "{ source, note, pages: [{ path, old[], name, title, description, kind, body_html }] }", cadence: "when a page of the section changes", media_type: "application/json" },
    DataFile { path: "/humans.txt", purpose: "Who builds this: the person, the agents and the machines.", schema: "humanstxt.org", cadence: "rarely", media_type: "text/plain" },
    DataFile { path: "/robots.txt", purpose: "Every crawler and AI agent is allowed everywhere; points at the sitemap.", schema: "robots.txt", cadence: "every build", media_type: "text/plain" },
];

/// Crawlers named one by one in robots.txt so nobody has to guess whether
/// the wildcard was meant for them.
pub const AI_AGENTS: &[&str] = &[
    "GPTBot", "ChatGPT-User", "OAI-SearchBot", "ClaudeBot", "Claude-Web", "Claude-SearchBot", "anthropic-ai",
    "PerplexityBot", "Perplexity-User", "Google-Extended", "Googlebot", "Applebot", "Applebot-Extended",
    "Bingbot", "Bytespider", "CCBot", "Amazonbot", "cohere-ai", "Meta-ExternalAgent", "Meta-ExternalFetcher",
    "DuckAssistBot", "YouBot", "MistralAI-User", "Twitterbot", "facebookexternalhit", "LinkedInBot",
];

/// What the build knows about time and history.
pub struct BuildCtx {
    /// RFC 3339, the moment of this build.
    pub built: String,
    /// RFC 3339, the latest commit of the repository (what generated pages last changed with).
    pub head_committed: String,
    /// RFC 3339, when the ledger was last written (for /now/).
    pub ledger_written: String,
    /// RFC 3339, when facts.json was gathered (for /progress/ and the front page tables).
    pub facts_generated: String,
}

impl BuildCtx {
    pub fn gather(now: &Now, facts_generated: &str) -> BuildCtx {
        let built = rfc3339(now_secs());
        let head = git(&["log", "-1", "--format=%cI"]).unwrap_or_else(|| built.clone());
        let ledger = now::parse_utc(&now.ledger_written_utc).map(rfc3339).unwrap_or_else(|| built.clone());
        let facts = if facts_generated.is_empty() { built.clone() } else { facts_generated.to_string() };
        BuildCtx { built, head_committed: head, ledger_written: ledger, facts_generated: facts }
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// RFC 3339 in UTC with seconds, from Unix seconds.
pub fn rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

// ------------------------------------------------------------------ pages

/// One page found in dist.
#[derive(Clone, Debug)]
pub struct Emitted {
    /// Site path with leading and trailing slash, e.g. `/research/status/`.
    pub path: String,
    pub file: PathBuf,
    pub lang: String,
    pub lastmod: String,
}

impl Emitted {
    pub fn url(&self) -> String {
        format!("{HOST}{}", self.path)
    }
    pub fn is_zh(&self) -> bool {
        self.path.starts_with("/zh-CN/")
    }
    /// The English path this page is, or mirrors.
    pub fn en_path(&self) -> String {
        if self.is_zh() { format!("/{}", &self.path["/zh-CN/".len()..]) } else { self.path.clone() }
    }
    pub fn zh_path(&self) -> String {
        if self.is_zh() { self.path.clone() } else { format!("/zh-CN{}", self.path) }
    }
}

/// Every `index.html` under dist, as site paths. The 404 page is not a page.
pub fn walk_pages(dist: &Path, ctx: &BuildCtx) -> Vec<Emitted> {
    let mut out = Vec::new();
    fn walk(dir: &Path, dist: &Path, ctx: &BuildCtx, out: &mut Vec<Emitted>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, dist, ctx, out);
            } else if p.file_name().map(|n| n == "index.html").unwrap_or(false) {
                let rel = p.strip_prefix(dist).unwrap().parent().unwrap();
                let mut path = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
                if !path.ends_with('/') {
                    path.push('/');
                }
                if path == "//" {
                    path = "/".into();
                }
                let lang = if path.starts_with("/zh-CN/") { "zh-CN" } else { "en" }.to_string();
                let lastmod = lastmod_for(&path, ctx);
                out.push(Emitted { path, file: p, lang, lastmod });
            }
        }
    }
    walk(dist, dist, ctx, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// When a page last changed. Hand-written pages carry the date of their
/// last commit when the history is available; pages rebuilt from live data
/// carry the data's own time; other generated pages carry the latest
/// repository commit. The build time is the fallback, never the first choice,
/// because a lastmod that moves on every hourly build tells crawlers nothing.
fn lastmod_for(path: &str, ctx: &BuildCtx) -> String {
    match path {
        "/now/" | "/zh-CN/now/" => return ctx.ledger_written.clone(),
        "/progress/" | "/" => return ctx.facts_generated.clone(),
        _ => {}
    }
    let source = format!("public{}index.html", path);
    if Path::new(&source).exists() {
        if let Some(d) = git(&["log", "-1", "--format=%cI", "--", &source]) {
            return d;
        }
    }
    ctx.head_committed.clone()
}

pub fn info_for(en_path: &str) -> Option<&'static PageInfo> {
    PAGES.iter().find(|p| p.path == en_path)
}

/// The markdown mirror of a page: `/` becomes `/index.md`, every other
/// directory page loses its trailing slash and gains `.md`. This is the
/// path an agent reaches by adding `.md` to the readable URL.
pub fn md_path(path: &str) -> String {
    if path == "/" || path == "/zh-CN/" {
        format!("{path}index.md")
    } else {
        format!("{}.md", path.trim_end_matches('/'))
    }
}

/// Fails the build when dist and PAGES disagree.
pub fn check_registry(pages: &[Emitted]) {
    for p in pages.iter().filter(|p| !p.is_zh()) {
        assert!(info_for(&p.path).is_some(), "{} was written but is not listed in seo::PAGES (add it so llms.txt, the sitemap and the feeds know it)", p.path);
    }
    for info in PAGES {
        assert!(pages.iter().any(|p| p.path == info.path), "seo::PAGES lists {} but no such page was written", info.path);
    }
}

// ------------------------------------------------------------------ head

/// What a head said before the pass touched it.
#[derive(Default, Debug)]
struct HeadFacts {
    title: String,
    description: String,
    og_title: String,
    og_description: String,
    og_type: String,
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<String> {
    // name="value" or name='value', any attribute order, any spacing.
    let mut i = 0;
    let bytes = tag.as_bytes();
    while let Some(pos) = tag[i..].find(name) {
        let start = i + pos;
        let before_ok = start == 0 || bytes[start - 1].is_ascii_whitespace();
        let mut j = start + name.len();
        while j < bytes.len() && bytes[j].is_ascii_whitespace() { j += 1; }
        if before_ok && j < bytes.len() && bytes[j] == b'=' {
            j += 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() { j += 1; }
            if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                let q = bytes[j];
                let end = tag[j + 1..].find(q as char)? + j + 1;
                return Some(tag[j + 1..end].to_string());
            }
        }
        i = start + name.len();
    }
    None
}

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<').trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// True for the tags the pass owns and rewrites.
fn owned_tag(tag: &str) -> bool {
    match tag_name(tag).as_str() {
        "link" => {
            let rel = attr(tag, "rel").unwrap_or_default();
            rel == "canonical"
                || rel == "describedby"
                || (rel == "alternate"
                    && (attr(tag, "hreflang").is_some()
                        || attr(tag, "type").map(|t| t.contains("xml") || t.contains("markdown")).unwrap_or(false)))
        }
        "meta" => {
            let key = attr(tag, "property").or_else(|| attr(tag, "name")).unwrap_or_default();
            key.starts_with("og:") || key.starts_with("twitter:") || key.starts_with("article:") || key == "robots"
        }
        _ => false,
    }
}

/// Splits a head into (kept text, facts): owned tags and ld+json blocks are removed.
fn strip_head(head: &str) -> (String, HeadFacts) {
    let mut facts = HeadFacts::default();
    let mut kept = String::with_capacity(head.len());
    let mut i = 0;
    while i < head.len() {
        let Some(lt) = head[i..].find('<') else { kept.push_str(&head[i..]); break; };
        kept.push_str(&head[i..i + lt]);
        let start = i + lt;
        let rest = &head[start..];
        let name = tag_name(rest);
        if rest.starts_with("<!--") {
            let end = rest.find("-->").map(|e| e + 3).unwrap_or(rest.len());
            kept.push_str(&rest[..end]);
            i = start + end;
            continue;
        }
        if name == "script" || name == "style" || name == "title" {
            let close = format!("</{name}");
            let end = rest.to_ascii_lowercase().find(&close).map(|e| rest[e..].find('>').map(|g| e + g + 1).unwrap_or(rest.len())).unwrap_or(rest.len());
            let whole = &rest[..end];
            let open_end = whole.find('>').unwrap_or(0);
            let open = &whole[..=open_end.min(whole.len() - 1)];
            if name == "title" {
                facts.title = whole[open_end + 1..].split("</").next().unwrap_or("").trim().to_string();
                kept.push_str(whole);
            } else if name == "script" && attr(open, "type").map(|t| t == "application/ld+json").unwrap_or(false) {
                // dropped: the pass writes its own
            } else {
                kept.push_str(whole);
            }
            i = start + end;
            continue;
        }
        let end = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
        let tag = &rest[..end];
        if name == "meta" {
            let key = attr(tag, "property").or_else(|| attr(tag, "name")).unwrap_or_default();
            let content = attr(tag, "content").unwrap_or_default();
            match key.as_str() {
                "description" => facts.description = content.clone(),
                "og:title" => facts.og_title = content.clone(),
                "og:description" => facts.og_description = content.clone(),
                "og:type" => facts.og_type = content.clone(),
                _ => {}
            }
        }
        if !owned_tag(tag) {
            kept.push_str(tag);
        }
        i = start + end;
    }
    (kept, facts)
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn breadcrumb_name(segment: &str, zh: bool) -> String {
    match (segment, zh) {
        ("research", false) => "Research".into(),
        ("research", true) => "研究".into(),
        ("blog", false) => "Blog".into(),
        ("blog", true) => "博客".into(),
        ("status", false) => "Status".into(),
        ("status", true) => "状态".into(),
        _ => {
            let mut words: Vec<String> = segment.split('-').map(|w| {
                let mut c = w.chars();
                match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() }
            }).collect();
            if words.len() > 6 { words.truncate(6); }
            words.join(" ")
        }
    }
}

fn breadcrumb(page: &Emitted, page_name: &str) -> Value {
    let zh = page.is_zh();
    let home = if zh { format!("{HOST}/zh-CN/") } else { format!("{HOST}/") };
    let mut items = vec![json!({"@type": "ListItem", "position": 1, "name": if zh { "主页" } else { "Home" }, "item": home})];
    let rel = page.en_path();
    let segs: Vec<&str> = rel.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    let prefix = if zh { "/zh-CN" } else { "" };
    let mut acc = String::new();
    for (i, seg) in segs.iter().enumerate() {
        acc.push('/');
        acc.push_str(seg);
        let last = i + 1 == segs.len();
        let name = if last { page_name.to_string() } else { breadcrumb_name(seg, zh) };
        items.push(json!({"@type": "ListItem", "position": i + 2, "name": name, "item": format!("{HOST}{prefix}{acc}/")}));
    }
    json!({"@type": "BreadcrumbList", "@id": format!("{}#breadcrumb", page.url()), "itemListElement": items})
}

fn org() -> Value {
    json!({
        "@type": "Organization",
        "@id": format!("{HOST}/#organization"),
        "name": ORG_NAME,
        "alternateName": "aien-dev",
        "url": format!("{HOST}/"),
        "logo": {"@type": "ImageObject", "url": format!("{HOST}/logo.png"), "width": 512, "height": 512},
        "email": ORG_EMAIL,
        "sameAs": [ORG_URL, X_URL],
        "founder": {"@id": format!("{HOST}/#drake")}
    })
}

fn author() -> Value {
    json!({
        "@type": "Person",
        "@id": format!("{HOST}/#drake"),
        "name": AUTHOR,
        "alternateName": "Drake Stapleton",
        "url": AUTHOR_URL,
        "sameAs": [AUTHOR_GITHUB, X_URL],
        "affiliation": {"@id": format!("{HOST}/#organization")}
    })
}

fn website() -> Value {
    json!({
        "@type": "WebSite",
        "@id": format!("{HOST}/#website"),
        "url": format!("{HOST}/"),
        "name": SITE_NAME,
        "description": SITE_CLAIM,
        "inLanguage": ["en", "zh-CN"],
        "publisher": {"@id": format!("{HOST}/#organization")},
        "license": LICENSE_URL
    })
}

fn dataset(path: &str, name: &str, purpose: &str, modified: &str) -> Value {
    json!({
        "@type": "Dataset",
        "@id": format!("{HOST}{path}"),
        "name": name,
        "description": purpose,
        "url": format!("{HOST}{path}"),
        "license": LICENSE_URL,
        "isAccessibleForFree": true,
        "creator": {"@id": format!("{HOST}/#organization")},
        "dateModified": modified,
        "distribution": [{"@type": "DataDownload", "encodingFormat": "application/json", "contentUrl": format!("{HOST}{path}")}]
    })
}

fn software(key: &str, name: &str, repo: &str, role: &str) -> Value {
    json!({
        "@type": "SoftwareSourceCode",
        "@id": format!("https://github.com/{repo}"),
        "name": name,
        "alternateName": key,
        "description": role,
        "codeRepository": format!("https://github.com/{repo}"),
        "license": LICENSE_URL,
        "isAccessibleForFree": true,
        "author": {"@id": format!("{HOST}/#organization")}
    })
}

/// The JSON-LD graph for one page.
fn graph(page: &Emitted, info: &PageInfo, facts: &HeadFacts, has_mirror: bool, ctx: &BuildCtx, dist: &Path) -> Value {
    let url = page.url();
    let title = unescape(if facts.og_title.is_empty() { &facts.title } else { &facts.og_title });
    let description = unescape(if facts.description.is_empty() { &facts.og_description } else { &facts.description });
    let image = format!("{HOST}{}", info.image);
    let zh = page.is_zh();
    let kind = match info.kind {
        Kind::Home | Kind::Page | Kind::Live => "WebPage",
        Kind::Collection => "CollectionPage",
        Kind::Article => "Article",
        Kind::Paper => "ScholarlyArticle",
    };
    let mut node = json!({
        "@type": kind,
        "@id": url,
        "url": url,
        "name": title,
        "headline": title,
        "description": description,
        "inLanguage": page.lang,
        "isPartOf": {"@id": format!("{HOST}/#website")},
        "image": {"@type": "ImageObject", "url": image, "width": 1200, "height": 630},
        "datePublished": info.published,
        "dateModified": page.lastmod,
        "license": LICENSE_URL,
        "isAccessibleForFree": true,
        "publisher": {"@id": format!("{HOST}/#organization")}
    });
    let is_web_page = matches!(info.kind, Kind::Home | Kind::Page | Kind::Live | Kind::Collection);
    if is_web_page {
        // WebPage-only properties; the validator warns when they sit on an Article.
        node["primaryImageOfPage"] = json!({"@type": "ImageObject", "url": image, "width": 1200, "height": 630});
        node["breadcrumb"] = json!({"@id": format!("{url}#breadcrumb")});
    } else {
        node["author"] = json!({"@id": format!("{HOST}/#drake")});
        node["mainEntityOfPage"] = json!(url);
    }
    if info.kind == Kind::Paper {
        node["creativeWorkStatus"] = json!("Preprint, not peer reviewed");
        if let Some(doi) = info.doi {
            node["identifier"] = json!({"@type": "PropertyValue", "propertyID": "DOI", "value": doi});
            node["sameAs"] = json!(format!("https://doi.org/{doi}"));
        }
        let pdf = dist.join(page.en_path().trim_start_matches('/')).join("paper.pdf");
        if pdf.exists() {
            node["encoding"] = json!([{"@type": "MediaObject", "encodingFormat": "application/pdf", "contentUrl": format!("{HOST}{}paper.pdf", page.en_path())}]);
        }
    }
    if has_mirror {
        if zh {
            node["translationOfWork"] = json!({"@id": format!("{HOST}{}", page.en_path())});
        } else {
            node["workTranslation"] = json!({"@id": format!("{HOST}{}", page.zh_path())});
        }
    }
    let mut nodes = vec![org(), author(), website(), node];
    if is_web_page || page.path != "/" {
        nodes.push(breadcrumb(page, &title));
    }
    match info.path {
        "/" => {
            for (key, name, repo, role) in crate::facts::COMPONENTS {
                nodes.push(software(key, name, repo, role));
            }
            nodes.push(dataset("/now.json", "AIEN coordination ledger", DATA_FILES.iter().find(|d| d.path == "/now.json").unwrap().purpose, &ctx.ledger_written));
            nodes.push(dataset("/machines.json", "AIEN machines and speeds", DATA_FILES.iter().find(|d| d.path == "/machines.json").unwrap().purpose, &ctx.head_committed));
            nodes.push(dataset("/facts.json", "AIEN public record facts", DATA_FILES.iter().find(|d| d.path == "/facts.json").unwrap().purpose, &ctx.facts_generated));
            nodes.push(dataset("/status.json", "AIEN layer status", DATA_FILES.iter().find(|d| d.path == "/status.json").unwrap().purpose, &ctx.head_committed));
            nodes.push(dataset("/research-status.json", "AIEN research status", DATA_FILES.iter().find(|d| d.path == "/research-status.json").unwrap().purpose, &ctx.head_committed));
        }
        "/now/" => nodes.push(dataset("/now.json", "AIEN coordination ledger", DATA_FILES.iter().find(|d| d.path == "/now.json").unwrap().purpose, &ctx.ledger_written)),
        "/omega/" => nodes.push(dataset("/omega.json", "What Omega is", DATA_FILES.iter().find(|d| d.path == "/omega.json").unwrap().purpose, &ctx.head_committed)),
        "/cudaplusplus/" => nodes.push(dataset("/cudaplusplus.json", "CUDA++", DATA_FILES.iter().find(|d| d.path == "/cudaplusplus.json").unwrap().purpose, &ctx.head_committed)),
        "/machine/" => nodes.push(dataset("/machines.json", "AIEN machines and speeds", DATA_FILES.iter().find(|d| d.path == "/machines.json").unwrap().purpose, &ctx.head_committed)),
        "/evidence/" | "/research/status/" => nodes.push(dataset("/research-status.json", "AIEN research status", DATA_FILES.iter().find(|d| d.path == "/research-status.json").unwrap().purpose, &ctx.head_committed)),
        _ => {}
    }
    json!({"@context": "https://schema.org", "@graph": nodes})
}

/// The tags the pass writes into every head.
fn head_block(page: &Emitted, info: &PageInfo, facts: &HeadFacts, has_mirror: bool, ctx: &BuildCtx, dist: &Path) -> String {
    let url = page.url();
    let title = if facts.og_title.is_empty() { facts.title.clone() } else { facts.og_title.clone() };
    let description = if facts.og_description.is_empty() { facts.description.clone() } else { facts.og_description.clone() };
    let og_type = if !facts.og_type.is_empty() { facts.og_type.clone() } else { match info.kind { Kind::Article | Kind::Paper => "article".into(), _ => "website".into() } };
    let image = format!("{HOST}{}?v={OG_IMAGE_VERSION}", info.image);
    let image_alt = escape(&format!("{SITE_NAME}: {}", info.name));
    let locale = if page.is_zh() { "zh_CN" } else { "en_US" };
    let mut b = String::new();
    b.push_str(&format!("<link rel=\"canonical\" href=\"{url}\">\n"));
    b.push_str(&format!("<link rel=\"alternate\" type=\"text/markdown\" href=\"{}\">\n", md_path(&page.path)));
    b.push_str(&format!("<link rel=\"describedby\" href=\"{HOST}/llms.txt\">\n"));
    b.push_str("<meta name=\"robots\" content=\"index, follow, max-snippet:-1, max-image-preview:large\">\n");
    if has_mirror {
        let en = format!("{HOST}{}", page.en_path());
        let zh = format!("{HOST}{}", page.zh_path());
        b.push_str(&format!("<link rel=\"alternate\" hreflang=\"en\" href=\"{en}\">\n<link rel=\"alternate\" hreflang=\"zh-CN\" href=\"{zh}\">\n<link rel=\"alternate\" hreflang=\"x-default\" href=\"{en}\">\n"));
    }
    b.push_str(&format!("<link rel=\"alternate\" type=\"application/atom+xml\" title=\"AienOS: blog and research\" href=\"{HOST}/feed.xml\">\n"));
    b.push_str(&format!("<link rel=\"alternate\" type=\"application/atom+xml\" title=\"AienOS: settled work from the live ledger\" href=\"{HOST}/now/feed.xml\">\n"));
    b.push_str(&format!("<meta property=\"og:site_name\" content=\"{SITE_NAME}\">\n<meta property=\"og:locale\" content=\"{locale}\">\n"));
    b.push_str(&format!("<meta property=\"og:title\" content=\"{title}\">\n<meta property=\"og:description\" content=\"{description}\">\n<meta property=\"og:url\" content=\"{url}\">\n<meta property=\"og:type\" content=\"{og_type}\">\n"));
    b.push_str(&format!("<meta property=\"og:image\" content=\"{image}\">\n<meta property=\"og:image:width\" content=\"1200\">\n<meta property=\"og:image:height\" content=\"630\">\n<meta property=\"og:image:alt\" content=\"{image_alt}\">\n"));
    if matches!(info.kind, Kind::Article | Kind::Paper) {
        b.push_str(&format!("<meta property=\"article:published_time\" content=\"{}\">\n<meta property=\"article:modified_time\" content=\"{}\">\n<meta property=\"article:author\" content=\"{AUTHOR_URL}\">\n", info.published, page.lastmod));
    }
    b.push_str(&format!("<meta name=\"twitter:card\" content=\"summary_large_image\">\n<meta name=\"twitter:site\" content=\"{X_HANDLE}\">\n<meta name=\"twitter:creator\" content=\"{X_HANDLE}\">\n<meta name=\"twitter:title\" content=\"{title}\">\n<meta name=\"twitter:description\" content=\"{description}\">\n<meta name=\"twitter:image\" content=\"{image}\">\n<meta name=\"twitter:image:alt\" content=\"{image_alt}\">\n"));
    let ld = serde_json::to_string(&graph(page, info, facts, has_mirror, ctx, dist)).expect("JSON-LD serializes").replace("</", "<\\/");
    b.push_str(&format!("<script type=\"application/ld+json\">{ld}</script>\n"));
    b
}

fn yaml_quote(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The markdown mirror of one page: frontmatter, the page body as markdown
/// from the same HTML that was just written, and the sitemap section every
/// mirror carries so an agent can walk the rest of the site from it.
fn markdown_mirror(page: &Emitted, facts: &HeadFacts, html: &str) -> String {
    let title = unescape(if facts.og_title.is_empty() { &facts.title } else { &facts.og_title });
    let description = unescape(if facts.description.is_empty() { &facts.og_description } else { &facts.description });
    let date = page.lastmod.get(..10).unwrap_or(&page.lastmod);
    let mut s = String::new();
    s.push_str("---\n");
    s.push_str(&format!("title: \"{}\"\n", yaml_quote(&title)));
    s.push_str(&format!("description: \"{}\"\n", yaml_quote(&description)));
    s.push_str(&format!("last_updated: \"{date}\"\n"));
    s.push_str(&format!("canonical_url: \"{}\"\n", page.url()));
    s.push_str("---\n\n");
    s.push_str(&html_to_text(html));
    s.push_str(&format!("\n\n## Sitemap\n\nSee the full [sitemap]({HOST}/sitemap.md) for every page on aienos.com.\n"));
    s
}

/// Writes the mirror next to the page: `/index.md` for `/`, `/machine.md`
/// for `/machine/`, and the same shape under `/zh-CN/`.
fn write_markdown_mirror(dist: &Path, page: &Emitted, facts: &HeadFacts, html: &str) {
    let body = markdown_mirror(page, facts, html);
    // Both conventions agents use: the sibling file (`/machine.md`) and the
    // directory index (`/machine/index.md`). The head points at the sibling.
    let mut paths = vec![md_path(&page.path)];
    if page.path != "/" && page.path != "/zh-CN/" {
        paths.push(format!("{}index.md", page.path));
    }
    for rel in paths {
        let file = dist.join(rel.trim_start_matches('/'));
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| panic!("create {}: {e}", parent.display()));
        }
        fs::write(&file, &body).unwrap_or_else(|e| panic!("write {}: {e}", file.display()));
    }
}

/// Every page links the glossary. Generated pages already do through the
/// footer; hand-written pages under public/ get one small link before
/// </body>, so the term page is reachable from every scanned URL.
fn ensure_glossary_link(mut html: String) -> String {
    if html.contains("href=\"/glossary/\"") {
        return html;
    }
    let block = "<div style=\"padding: 18px 24px; text-align: center; font-family: var(--font-mono); font-size: 12px;\"><a href=\"/glossary/\" style=\"color: var(--accent-green);\">Glossary</a></div>";
    match html.rfind("</body>") {
        Some(i) => {
            html.insert_str(i, block);
            html
        }
        None => html,
    }
}

/// Rewrites the head of every page in dist. Returns the pages, for the sitemap and feeds.
pub fn enrich(dist: &Path, ctx: &BuildCtx) -> Vec<Emitted> {
    let pages = walk_pages(dist, ctx);
    check_registry(&pages);
    for page in &pages {
        let html = fs::read_to_string(&page.file).unwrap_or_else(|e| panic!("read {}: {e}", page.file.display()));
        let lower = html.to_ascii_lowercase();
        let head_start = lower.find("<head").and_then(|h| lower[h..].find('>').map(|g| h + g + 1)).unwrap_or_else(|| panic!("{} has no <head>", page.path));
        let head_end = lower.find("</head>").unwrap_or_else(|| panic!("{} has no </head>", page.path));
        let (kept, facts) = strip_head(&html[head_start..head_end]);
        assert!(!facts.title.is_empty(), "{} has no <title>", page.path);
        assert!(!facts.description.is_empty() || !facts.og_description.is_empty(), "{} has no meta description", page.path);
        let info = info_for(&page.en_path()).unwrap_or_else(|| panic!("{} has no English page in seo::PAGES", page.path));
        let mirror = if page.is_zh() { dist.join(page.en_path().trim_start_matches('/')).join("index.html") } else { dist.join(page.zh_path().trim_start_matches('/')).join("index.html") };
        let has_mirror = mirror.exists();
        let block = head_block(page, info, &facts, has_mirror, ctx, dist);
        let mut out = String::with_capacity(html.len() + block.len());
        out.push_str(&html[..head_start]);
        out.push_str(kept.trim_end());
        out.push('\n');
        out.push_str(&block);
        out.push_str(&html[head_end..]);
        let out = crate::components::allen_intro::enrich(crate::components::support::enrich(crate::components::contact::enrich(ensure_glossary_link(out), page.is_zh())));
        write_markdown_mirror(dist, page, &facts, &out);
        fs::write(&page.file, out).unwrap_or_else(|e| panic!("write {}: {e}", page.file.display()));
    }
    println!("  [SEO] Rewrote the head of {} pages: canonical on {HOST}, cards, hreflang, JSON-LD, markdown mirrors", pages.len());
    pages
}

// ------------------------------------------------------------------ sitemap and robots

pub fn sitemap_xml(pages: &[Emitted]) -> String {
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n");
    for p in pages {
        s.push_str(&format!("  <url>\n    <loc>{}</loc>\n    <lastmod>{}</lastmod>\n", p.url(), p.lastmod));
        let en = format!("{HOST}{}", p.en_path());
        let zh = format!("{HOST}{}", p.zh_path());
        let has_mirror = pages.iter().any(|q| q.path == (if p.is_zh() { p.en_path() } else { p.zh_path() }));
        if has_mirror {
            s.push_str(&format!("    <xhtml:link rel=\"alternate\" hreflang=\"en\" href=\"{en}\"/>\n    <xhtml:link rel=\"alternate\" hreflang=\"zh-CN\" href=\"{zh}\"/>\n    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{en}\"/>\n"));
        }
        s.push_str("  </url>\n");
    }
    s.push_str("</urlset>\n");
    s
}

pub fn robots_txt() -> String {
    let mut s = String::from("# www.aienos.com is open to every reader, human or machine.\n# No crawler is blocked, no CAPTCHA, no bot wall. Cite freely; every page carries receipts.\n\nUser-agent: *\nAllow: /\n\n");
    for a in AI_AGENTS {
        s.push_str(&format!("User-agent: {a}\nAllow: /\n\n"));
    }
    s.push_str(&format!("Sitemap: {HOST}/sitemap.xml\n"));
    s
}

// ------------------------------------------------------------------ feeds

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn atom_head(title: &str, subtitle: &str, self_path: &str, updated: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n  <title>{}</title>\n  <subtitle>{}</subtitle>\n  <link href=\"{HOST}{self_path}\" rel=\"self\" type=\"application/atom+xml\"/>\n  <link href=\"{HOST}/\" rel=\"alternate\" type=\"text/html\"/>\n  <id>{HOST}{self_path}</id>\n  <updated>{updated}</updated>\n  <author><name>{}</name><uri>{AUTHOR_URL}</uri></author>\n  <rights>{LICENSE_NAME}</rights>\n  <icon>{HOST}/logo.png</icon>\n  <logo>{HOST}/og.png</logo>\n",
        xml(title), xml(subtitle), xml(AUTHOR)
    )
}

/// /feed.xml: the blog posts and research papers, newest change first.
pub fn site_feed(pages: &[Emitted]) -> String {
    let mut entries: Vec<(&Emitted, &PageInfo)> = pages
        .iter()
        .filter(|p| !p.is_zh())
        .filter_map(|p| info_for(&p.path).map(|i| (p, i)))
        .filter(|(_, i)| matches!(i.kind, Kind::Article | Kind::Paper))
        .collect();
    entries.sort_by(|a, b| b.0.lastmod.cmp(&a.0.lastmod).then(b.1.published.cmp(a.1.published)));
    let updated = entries.first().map(|e| e.0.lastmod.clone()).unwrap_or_else(|| rfc3339(now_secs()));
    let mut s = atom_head("AienOS: blog and research", "Research notes and preprints from the AIEN project, with receipts.", "/feed.xml", &updated);
    for (p, i) in entries {
        s.push_str(&format!(
            "  <entry>\n    <title>{}</title>\n    <link href=\"{}\" rel=\"alternate\" type=\"text/html\"/>\n    <id>{}</id>\n    <published>{}T00:00:00Z</published>\n    <updated>{}</updated>\n    <summary>{}</summary>\n    <category term=\"{}\"/>\n  </entry>\n",
            xml(i.name), p.url(), p.url(), i.published, p.lastmod, xml(i.what), if i.kind == Kind::Paper { "research" } else { "blog" }
        ));
    }
    s.push_str("</feed>\n");
    s
}

/// /now/feed.xml: one entry per settled ledger line, newest first, with its evidence.
pub fn now_feed(now: &Now, ctx: &BuildCtx) -> String {
    let mut settled: Vec<&now::Item> = now.items.iter().filter(|i| i.status == "done" || i.status == "dropped").collect();
    settled.sort_by(|a, b| now::parse_utc(&b.updated_utc).cmp(&now::parse_utc(&a.updated_utc)).then(b.id.cmp(&a.id)));
    let mut s = atom_head("AienOS: settled work from the live ledger", "Every coordination ledger line that reached done or dropped, with the evidence that settled it.", "/now/feed.xml", &ctx.ledger_written);
    for it in settled {
        let updated = now::parse_utc(&it.updated_utc).map(rfc3339).unwrap_or_else(|| ctx.ledger_written.clone());
        let published = now::parse_utc(&it.opened_utc).map(rfc3339).unwrap_or_else(|| updated.clone());
        let mut body = format!("Status: {}. Owner: {}. Stream: {}.", it.status, it.owner, it.stream);
        if !it.done_when.is_empty() {
            body.push_str(&format!("\nDone when: {}", it.done_when));
        }
        if !it.notes.is_empty() {
            body.push_str("\nEvidence:");
            for n in &it.notes {
                body.push_str(&format!("\n- {n}"));
            }
        }
        s.push_str(&format!(
            "  <entry>\n    <title>{}: {} ({})</title>\n    <link href=\"{HOST}/now/#{}\" rel=\"alternate\" type=\"text/html\"/>\n    <id>{HOST}/now/#{}</id>\n    <published>{published}</published>\n    <updated>{updated}</updated>\n    <content type=\"text\">{}</content>\n    <category term=\"{}\"/>\n  </entry>\n",
            xml(&it.id), xml(&it.item), xml(&it.status), xml(&it.id), xml(&it.id), xml(&body), xml(&it.status)
        ));
    }
    s.push_str("</feed>\n");
    s
}

// ------------------------------------------------------------------ llms.txt, data index, humans.txt

fn section_title(s: Section) -> &'static str {
    match s {
        Section::Start => "Start here",
        Section::Machine => "The machine",
        Section::Turing => "The Turing and the research",
        Section::Evidence => "Evidence and status",
        Section::Philosophy => "Philosophy",
        Section::Blog => "Blog",
        Section::Engineer => "About the engineer",
        Section::Optional => "Optional",
    }
}

/// /llms.txt in the llms.txt convention: one H1, a blockquote summary, H2
/// sections of links with one-line descriptions, an Optional section last.
pub fn llms_txt(status_lines: &[String], research_sentence: &str) -> String {
    let mut s = String::new();
    s.push_str("# AienOS\n\n");
    s.push_str(&format!("> {SITE_CLAIM}. AienOS is an experimental, open source (Apache-2.0 WITH LLVM-exception) machine for turning search into verified understanding: AIEN proposes, Omega makes the proposal explicit and verifiable, the machine tests it against reality, the Turing measures what was learned, Cortex keeps the evidence. Every claim on the site carries a public receipt. Sovereign Intelligence, built on AienOS, is in alpha testing; each component keeps its own evidence level. Built by {AUTHOR} with AI agents on five real machines.\n\n"));
    s.push_str("Rules the site keeps: no comparisons with other systems (every speed stands beside the physical ceiling of its own machine only), failures stay on the record, no outside fonts or unsolicited tracking (X pixel rfrlz loads only after saying hello to Allen; privacy and refusal at /privacy/; the film on /turing/ is an outside embed), and AI agents are welcome to read, cite, argue and correct. Contact Drake: drake@aienos.com or X @mdrakestapleton; every route, Allen included, is at https://www.aienos.com/contact/. Project mail: aien@aienos.com. Source: https://github.com/aien-dev.\n\n");
    s.push_str("## Current state\n\n");
    for l in status_lines {
        s.push_str(&format!("- {l}\n"));
    }
    s.push_str(&format!("- [Research status]({HOST}/research/status/): {research_sentence}\n\n"));
    for sec in [Section::Start, Section::Machine, Section::Turing, Section::Evidence, Section::Philosophy, Section::Blog, Section::Engineer] {
        s.push_str(&format!("## {}\n\n", section_title(sec)));
        for p in PAGES.iter().filter(|p| p.section == sec) {
            s.push_str(&format!("- [{}]({HOST}{}): {}\n", p.name, p.path, p.what));
        }
        s.push('\n');
    }
    s.push_str("## Machine-readable files\n\n");
    for d in DATA_FILES {
        s.push_str(&format!("- [{}]({HOST}{}): {} Updated: {}.\n", d.path, d.path, d.purpose, d.cadence));
    }
    s.push_str(&format!("- [{HOST}/llms-full.txt]({HOST}/llms-full.txt): every page's full text in one file.\n\n"));
    s.push_str("## Optional\n\n");
    for p in PAGES.iter().filter(|p| p.section == Section::Optional) {
        s.push_str(&format!("- [{}]({HOST}{}): {}\n", p.name, p.path, p.what));
    }
    s.push_str(&format!("- [简体中文]({HOST}/zh-CN/): the same site in Simplified Chinese (every page has a /zh-CN/ mirror).\n"));
    s.push_str(&format!("- [Papers as PDF]({HOST}/research/): each research page links its paper.pdf, paper.tex and SHA256SUMS.\n"));
    s.push_str(&format!("- [Drake on X ({X_HANDLE})]({X_URL}): where the work is posted as it happens.\n"));
    s
}

/// /sitemap.md: the readable twin of sitemap.xml, H2 sections of described
/// links, generated from the same page registry as llms.txt and the XML map.
pub fn sitemap_md(pages: &[Emitted]) -> String {
    let mut s = String::from("# Sitemap\n\n");
    s.push_str(&format!("> Every page on aienos.com, grouped the way the site is. The crawler index is [sitemap.xml]({HOST}/sitemap.xml); the file list is [data/index.json]({HOST}/data/index.json).\n\n"));
    for sec in [Section::Start, Section::Machine, Section::Turing, Section::Evidence, Section::Philosophy, Section::Blog, Section::Engineer, Section::Optional] {
        s.push_str(&format!("## {}\n\n", section_title(sec)));
        for p in PAGES.iter().filter(|p| p.section == sec) {
            s.push_str(&format!("- [{}]({HOST}{}): {}\n", p.name, p.path, p.what));
        }
        s.push('\n');
    }
    s.push_str("## Simplified Chinese\n\n");
    for p in pages.iter().filter(|p| p.is_zh()) {
        if let Some(info) = info_for(&p.en_path()) {
            s.push_str(&format!("- [{}]({HOST}{}): the same page in Simplified Chinese.\n", info.name, p.path));
        }
    }
    s.push_str("\n## For agents\n\n");
    s.push_str(&format!("- [llms.txt]({HOST}/llms.txt): the site in one file for AI readers.\n"));
    s.push_str(&format!("- [llms-full.txt]({HOST}/llms-full.txt): every page's full text in one file.\n"));
    s.push_str(&format!("- [AGENTS.md]({HOST}/AGENTS.md): how to build, test and change this site.\n"));
    s.push_str(&format!("- [data/index.json]({HOST}/data/index.json): every machine-readable file with its purpose and cadence.\n"));
    s
}

/// Plain text of a page body: scripts, styles, header, nav and footer dropped,
/// headings marked, links kept with their targets.
pub fn html_to_text(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<main").map(|m| lower[m..].find('>').map(|g| m + g + 1).unwrap_or(m)).or_else(|| lower.find("<body").map(|m| lower[m..].find('>').map(|g| m + g + 1).unwrap_or(m))).unwrap_or(0);
    let end = lower.find("</main>").or_else(|| lower.find("<footer")).or_else(|| lower.find("</body>")).unwrap_or(html.len());
    let body = &html[start..end.max(start)];
    let mut out = String::new();
    let mut i = 0;
    let mut href: Option<String> = None;
    let mut link_text = String::new();
    while i < body.len() {
        let Some(lt) = body[i..].find('<') else { out.push_str(&unescape(&body[i..])); break; };
        let text = &body[i..i + lt];
        if href.is_some() { link_text.push_str(&unescape(text)); } else { out.push_str(&unescape(text)); }
        let rest = &body[i + lt..];
        let name = tag_name(rest);
        if matches!(name.as_str(), "script" | "style" | "svg" | "nav" | "header" | "template") || rest.starts_with("<!--") {
            let close = if rest.starts_with("<!--") { "-->".to_string() } else { format!("</{name}") };
            let e = rest.to_ascii_lowercase().find(&close).map(|e| rest[e..].find('>').map(|g| e + g + 1).unwrap_or(rest.len())).unwrap_or(rest.len());
            i += lt + e;
            continue;
        }
        let e = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
        let tag = &rest[..e];
        let closing = tag.starts_with("</");
        match name.as_str() {
            "h1" if !closing => out.push_str("\n\n# "),
            "h2" if !closing => out.push_str("\n\n## "),
            "h3" | "h4" if !closing => out.push_str("\n\n### "),
            "p" | "div" | "section" | "article" | "li" | "tr" | "figure" | "figcaption" | "blockquote" | "ul" | "ol" | "table" | "aside" | "pre" | "h1" | "h2" | "h3" | "h4" | "summary" | "details" => out.push('\n'),
            "br" => out.push('\n'),
            "td" | "th" | "span" | "small" | "em" | "strong" | "b" | "i" | "code" | "time" | "label" | "dt" | "dd" => out.push(' '),
            "a" if !closing => {
                let h = attr(tag, "href").unwrap_or_default();
                if h.starts_with("http") || h.starts_with('/') {
                    href = Some(if h.starts_with('/') { format!("{HOST}{h}") } else { h });
                    link_text.clear();
                }
            }
            "a" if closing => {
                if let Some(h) = href.take() {
                    let t = link_text.trim();
                    if t.is_empty() { out.push_str(&format!(" {h} ")); } else { out.push_str(&format!("[{t}]({h})")); }
                }
            }
            _ => {}
        }
        if name == "li" && !closing { out.push_str("- "); }
        i += lt + e;
    }
    // Collapse runs of blank lines and trailing spaces.
    let mut cleaned = String::new();
    let mut blank = 0;
    for line in out.lines() {
        let l = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if l.is_empty() {
            blank += 1;
            if blank <= 1 { cleaned.push('\n'); }
        } else {
            blank = 0;
            cleaned.push_str(&l);
            cleaned.push('\n');
        }
    }
    cleaned.trim().to_string()
}

/// /llms-full.txt: every English page as text, in llms.txt order, from the files just written.
pub fn llms_full(pages: &[Emitted]) -> String {
    let mut s = format!("# AienOS: every page in one file\n\nGenerated with the site from the same sources as the HTML. Page list and summaries: {HOST}/llms.txt. Machine-readable data: {HOST}/data/index.json.\n\n");
    for info in PAGES {
        let Some(p) = pages.iter().find(|p| p.path == info.path) else { continue };
        let html = fs::read_to_string(&p.file).unwrap_or_default();
        s.push_str(&format!("\n---\n\n# {}\n\nSource: {}\nPublished: {}. Last changed: {}.\n\n{}\n", info.name, p.url(), info.published, p.lastmod, html_to_text(&html)));
    }
    s
}

pub fn data_index(ctx: &BuildCtx) -> String {
    let files: Vec<Value> = DATA_FILES
        .iter()
        .map(|d| json!({"path": d.path, "url": format!("{HOST}{}", d.path), "purpose": d.purpose, "schema": d.schema, "cadence": d.cadence, "media_type": d.media_type}))
        .collect();
    let v = json!({
        "site": format!("{HOST}/"),
        "generated": ctx.built,
        "license": LICENSE_NAME,
        "license_url": LICENSE_URL,
        "contact": ORG_EMAIL,
        "source": ORG_URL,
        "rules": [
            "No comparisons with other systems: every number stands beside the physical ceiling of its own machine only.",
            "Every claim links its receipt. Failures stay on the record.",
            "No outside fonts or unsolicited tracking. X pixel rfrlz loads only with Allen introduction consent; /privacy/ explains and controls the choice. The film on /turing/ is an outside embed."
        ],
        "files": files
    });
    serde_json::to_string_pretty(&v).expect("data index serializes") + "\n"
}

pub fn humans_txt(ctx: &BuildCtx) -> String {
    format!(
        "/* TEAM */\n\tBuilder and owner: {AUTHOR}\n\tSite: {AUTHOR_URL}\n\tX: {X_HANDLE}\n\tGitHub: {AUTHOR_GITHUB}\n\tContact: {ORG_EMAIL}\n\n/* AGENTS */\n\tThe code is written with AI coding agents working in parallel under one human conductor.\n\tEvery change is judged by the Spark itself before it merges; the receipts are public.\n\tAgents are first-class readers here: read, cite, argue, correct. {HOST}/llms.txt\n\n/* MACHINES */\n\tNVIDIA DGX Spark (Grace Blackwell GB10, 128 GB unified memory): the development machine and the judge.\n\tIntel iMac 2017: bare metal Aienux, the x86_64 port of the AienOS C kernel.\n\tRaspberry Pi 5: the small machine.\n\tQEMU: the laboratory where every kernel boots first.\n\tThe rig: the owned inference box.\n\tDetails and receipts: {HOST}/machine/ and {HOST}/machines.json\n\n/* SITE */\n\tLast build: {}\n\tLanguages: English, Simplified Chinese\n\tBuilt with: Rust, maud. No frameworks or outside fonts. X tracking only after consent to say hello to Allen; see /privacy/.\n\tSource: https://github.com/aien-dev/aienos.com\n\tLicense: {LICENSE_NAME}\n",
        ctx.built
    )
}

// ------------------------------------------------------------------ 404

pub fn render_404() -> Markup {
    html! {
        section class="hub" {
            div class="container" style="padding: 96px 0 120px 0; max-width: 720px;" {
                p class="hub-kicker" style="font-family: var(--font-mono); font-size: 12px; letter-spacing: .14em; color: var(--accent-blue); text-transform: uppercase; margin-bottom: 16px;" { "404: no receipt for this address" }
                h1 style="font-size: clamp(32px, 5vw, 48px); font-weight: 800; letter-spacing: -0.02em; color: var(--text-primary); margin: 0 0 16px;" { "There is nothing at this address." }
                p style="color: var(--text-secondary); font-size: 17px; line-height: 1.7;" {
                    "The page may have moved, or the link was never right. Nothing on this site disappears without a trace: every page is listed in "
                    a href="/llms.txt" { "llms.txt" }
                    " and in the "
                    a href="/sitemap.xml" { "sitemap" }
                    "."
                }
                ul style="margin: 28px 0 0 0; padding: 0; list-style: none; display: grid; gap: 10px;" {
                    li { a href="/" { "The front page" } ": the loop, the machines, the evidence." }
                    li { a href="/machine/" { "The Machine" } ": what AienOS is and what runs today." }
                    li { a href="/now/" { "Now" } ": what every agent is doing right now." }
                    li { a href="/research/" { "Research" } ": the preprints and the papers." }
                    li { a href="/evidence/" { "Evidence" } ": receipts over claims." }
                }
                p style="margin-top: 36px; font-family: var(--font-mono); font-size: 12px; color: var(--text-muted);" {
                    "Found a broken link on the site? Open an issue at "
                    a href="https://github.com/aien-dev/aienos.com/issues" rel="noopener noreferrer" { "aien-dev/aienos.com" }
                    "."
                }
            }
        }
    }
}

/// Extra head for the 404 page: never indexed, no canonical.
pub fn head_404() -> Markup {
    html! { meta name="robots" content="noindex"; (PreEscaped("")) }
}

/// Writes everything that is not a page: sitemap, robots, feeds, llms files, data index, humans.
pub fn write_machine_surface(dist: &Path, pages: &[Emitted], now: &Now, ctx: &BuildCtx, llms: &str) {
    fs::write(dist.join("sitemap.xml"), sitemap_xml(pages)).expect("write sitemap.xml");
    fs::write(dist.join("robots.txt"), robots_txt()).expect("write robots.txt");
    fs::write(dist.join("feed.xml"), site_feed(pages)).expect("write feed.xml");
    fs::create_dir_all(dist.join("now")).expect("now dir");
    fs::write(dist.join("now/feed.xml"), now_feed(now, ctx)).expect("write now/feed.xml");
    fs::write(dist.join("llms.txt"), llms).expect("write llms.txt");
    fs::write(dist.join("llms-full.txt"), llms_full(pages)).expect("write llms-full.txt");
    fs::write(dist.join("sitemap.md"), sitemap_md(pages)).expect("write sitemap.md");
    let agents = fs::read_to_string("AGENTS.md").expect("AGENTS.md must exist at the repository root: the build publishes it at /AGENTS.md");
    fs::write(dist.join("AGENTS.md"), agents).expect("write AGENTS.md");
    fs::create_dir_all(dist.join("data")).expect("data dir");
    fs::write(dist.join("data/index.json"), data_index(ctx)).expect("write data/index.json");
    fs::write(dist.join("humans.txt"), humans_txt(ctx)).expect("write humans.txt");
    println!("  [GEN] sitemap.xml ({} urls), sitemap.md, AGENTS.md, robots.txt ({} named agents), feed.xml, now/feed.xml, llms.txt, llms-full.txt, data/index.json, humans.txt", pages.len(), AI_AGENTS.len());
}

/// No page may embed an outside script, even the consent-only X script.
/// Its sole URL belongs in the first-party loader and is tested before consent.
fn check_script_boundary(html: &str) -> Result<(), String> {
    let lower = html.to_ascii_lowercase();
    if lower.contains("uwt.js") || lower.contains("twq(") {
        return Err("X bootstrap must live only in /js/allen-intro.js".into());
    }
    if lower.contains("fonts.googleapis.com") || lower.contains("fonts.gstatic.com") {
        return Err("outside fonts are forbidden".into());
    }
    let mut rest = lower.as_str();
    while let Some(start) = rest.find("<script") {
        rest = &rest[start..];
        let end = rest.find('>').ok_or("unterminated script tag")?;
        let open = &rest[..=end];
        if let Some(src) = attr(open, "src") {
            if !src.starts_with('/') || src.starts_with("//") || src.contains('\\') || src.chars().any(char::is_whitespace) {
                return Err(format!("outside or ambiguous script source: {src}"));
            }
        } else if open.contains("src") {
            return Err("script sources must be quoted root-relative paths".into());
        }
        rest = &rest[end + 1..];
    }
    Ok(())
}

fn verify_consent_boundary(dist: &Path) {
    fn walk(dir: &Path) {
        for entry in fs::read_dir(dir).expect("read consent boundary directory") {
            let path = entry.expect("consent boundary entry").path();
            if path.is_dir() {
                walk(&path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
                let html = fs::read_to_string(&path).expect("read consent boundary HTML");
                check_script_boundary(&html).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                assert_eq!(html.matches("src=\"/js/allen-intro.js\"").count(), 1, "{} must have exactly one first-party consent loader", path.display());
                assert_eq!(html.matches("id=\"aien-allen-intro\"").count(), 1, "{} must have exactly one consent dialog", path.display());
                assert!(html.contains("href=\"/privacy/\""), "{} must link privacy", path.display());
            } else if path.extension().and_then(|e| e.to_str()) == Some("js") {
                let js = fs::read_to_string(&path).expect("read script");
                if js.contains("uwt.js") || js.contains("twq(") {
                    assert!(path.ends_with("js/allen-intro.js"), "{} must not initialize the X pixel", path.display());
                }
            }
        }
    }
    walk(dist);
    let js = fs::read_to_string(dist.join("js/allen-intro.js")).expect("first-party consent script");
    assert_eq!(js.matches("https://static.ads-twitter.com/uwt.js").count(), 1);
    assert_eq!(js.matches("twq('config','rfrlz')").count(), 1);
}

/// Fails the build when the machine surface is inconsistent with the pages.
pub fn verify(dist: &Path) {
    verify_consent_boundary(dist);
    let sitemap = fs::read_to_string(dist.join("sitemap.xml")).expect("sitemap.xml");
    let robots = fs::read_to_string(dist.join("robots.txt")).expect("robots.txt");
    let llms = fs::read_to_string(dist.join("llms.txt")).expect("llms.txt");
    let index: Value = serde_json::from_str(&fs::read_to_string(dist.join("data/index.json")).expect("data/index.json")).expect("data/index.json parses");
    for a in ["GPTBot", "ClaudeBot", "PerplexityBot", "Google-Extended", "CCBot"] {
        assert!(robots.contains(&format!("User-agent: {a}\nAllow: /")), "robots.txt must allow {a}");
    }
    assert!(robots.contains(&format!("Sitemap: {HOST}/sitemap.xml")), "robots.txt must point at the sitemap");
    assert!(!sitemap.contains("https://aienos.com/"), "sitemap must use the www host");
    let sitemap_md = fs::read_to_string(dist.join("sitemap.md")).expect("sitemap.md");
    assert!(sitemap_md.starts_with("# Sitemap\n"), "sitemap.md must start with its H1");
    assert!(sitemap_md.contains("\n## "), "sitemap.md must carry H2 sections");
    assert!(sitemap_md.contains(&format!("]({HOST}/glossary/)")), "sitemap.md must link the glossary");
    let agents = fs::read_to_string(dist.join("AGENTS.md")).expect("AGENTS.md");
    for section in ["## Installation", "## Configuration", "## Usage"] {
        assert!(agents.contains(section), "AGENTS.md must carry {section}");
    }
    let index_html = fs::read_to_string(dist.join("index.html")).expect("index.html");
    assert!(index_html.contains("href=\"/glossary/\""), "the homepage must link the glossary");
    assert!(index_html.contains("\"BreadcrumbList\""), "the homepage JSON-LD must carry a BreadcrumbList");
    for d in DATA_FILES {
        let rel = d.path.trim_start_matches('/');
        assert!(dist.join(rel).exists(), "data index lists {} but it was not written", d.path);
        assert!(llms.contains(d.path), "llms.txt must link {}", d.path);
    }
    assert_eq!(index["files"].as_array().map(|a| a.len()), Some(DATA_FILES.len()));
    for info in PAGES {
        assert!(sitemap.contains(&format!("<loc>{HOST}{}</loc>", info.path)), "sitemap must list {}", info.path);
        assert!(llms.contains(&format!("]({HOST}{})", info.path)), "llms.txt must link {}", info.path);
        let page = fs::read_to_string(dist.join(info.path.trim_start_matches('/')).join("index.html")).expect("page");
        assert!(page.contains(&format!("<link rel=\"canonical\" href=\"{HOST}{}\">", info.path)), "{} canonical must be on {HOST}", info.path);
        assert!(page.contains("application/ld+json"), "{} must carry JSON-LD", info.path);
        assert!(page.contains("twitter:site"), "{} must carry twitter:site", info.path);
        let md_rel = md_path(info.path);
        assert!(page.contains(&format!("<link rel=\"alternate\" type=\"text/markdown\" href=\"{md_rel}\">")), "{} must link its markdown mirror {md_rel}", info.path);
        assert!(page.contains(&format!("<link rel=\"describedby\" href=\"{HOST}/llms.txt\">")), "{} must declare llms.txt via describedby", info.path);
        assert!(page.contains("max-snippet:-1") && page.contains("max-image-preview:large"), "{} must carry the AI snippet directives", info.path);
        let md = fs::read_to_string(dist.join(md_rel.trim_start_matches('/'))).unwrap_or_else(|_| panic!("{} markdown mirror {md_rel} missing", info.path));
        assert!(md.starts_with("---\n"), "{} markdown mirror must open with frontmatter", info.path);
        assert!(md.contains("canonical_url:"), "{} markdown mirror must carry canonical_url", info.path);
        assert!(md.contains("\n## Sitemap\n"), "{} markdown mirror must carry a Sitemap section", info.path);
        if info.path != "/" {
            let dir_md = format!("{}index.md", info.path);
            assert!(dist.join(dir_md.trim_start_matches('/')).exists(), "{} markdown directory mirror {dir_md} missing", info.path);
        }
        assert!(page.contains("href=\"/glossary/\""), "{} must link the glossary", info.path);
        let ld_start = page.find("<script type=\"application/ld+json\">").expect("ld") + "<script type=\"application/ld+json\">".len();
        let ld_end = ld_start + page[ld_start..].find("</script>").expect("ld end");
        let v: Value = serde_json::from_str(&page[ld_start..ld_end]).unwrap_or_else(|e| panic!("{} JSON-LD does not parse: {e}", info.path));
        assert!(v["@graph"].as_array().map(|g| g.len() >= 4).unwrap_or(false), "{} JSON-LD graph too small", info.path);
        for f in ["feed.xml", "now/feed.xml", "llms.txt", "llms-full.txt", "humans.txt", "data/index.json", "404.html", "logo.png", "og.png"] {
            assert!(dist.join(f).exists(), "missing {f}");
        }
        for text in [page.as_str()] {
            assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'), "dash in {}", info.path);
        }
    }
    for f in ["feed.xml", "now/feed.xml", "llms.txt", "llms-full.txt", "humans.txt", "data/index.json", "sitemap.xml", "sitemap.md", "AGENTS.md", "index.md", "robots.txt", "404.html"] {
        let t = fs::read_to_string(dist.join(f)).expect(f);
        assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'), "dash in {f}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_script_boundary_rejects_external_and_inline_pixel_loads() {
        assert!(check_script_boundary("<script defer src=\"/js/allen-intro.js\"></script>").is_ok());
        for bad in [
            "<script src=\"https://static.ads-twitter.com/uwt.js\"></script>",
            "<script src='//example.com/anything.js'></script>",
            "<SCRIPT SRC='https://example.com/anything.js'></SCRIPT>",
            "<script src=https://example.com/anything.js></script>",
            "<script src='/\\example.com/anything.js'></script>",
            "<script>twq('config','rfrlz')</script>",
        ] {
            assert!(check_script_boundary(bad).is_err(), "must reject {bad}");
        }
    }

    #[test]
    fn rfc3339_round_trip() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(now::parse_utc("2026-10-19T16:43Z").unwrap()), "2026-10-19T16:43:00Z");
    }

    #[test]
    fn attributes_parse_in_any_order() {
        let t = "<meta content=\"x y\" property=\"og:url\"/>";
        assert_eq!(attr(t, "property").as_deref(), Some("og:url"));
        assert_eq!(attr(t, "content").as_deref(), Some("x y"));
        assert_eq!(attr("<link rel=canonical>", "rel"), None);
        assert!(owned_tag(t));
        assert!(!owned_tag("<meta name=\"viewport\" content=\"width=device-width\">"));
        assert!(owned_tag("<link href=\"https://www.aienos.com/\" hreflang=\"en\" rel=\"alternate\"/>"));
        assert!(owned_tag("<link rel=\"alternate\" type=\"text/markdown\" href=\"/index.md\">"));
        assert!(owned_tag("<link rel=\"describedby\" href=\"/llms.txt\">"));
        assert!(owned_tag("<meta name=\"robots\" content=\"noindex\">"));
        assert!(!owned_tag("<link rel=\"stylesheet\" href=\"/assets/style.css\">"));
    }

    #[test]
    fn strip_head_keeps_title_and_drops_owned_tags() {
        let head = "<meta charset=\"utf-8\"><title>T &amp; U</title><meta name=\"description\" content=\"D\"><link rel=\"canonical\" href=\"https://aienos.com/x\"><meta property=\"og:title\" content=\"OT\"><script type=\"application/ld+json\">{}</script><script>console.log('<b>')</script><style>a{}</style><link rel=\"stylesheet\" href=\"/a.css\">";
        let (kept, facts) = strip_head(head);
        assert_eq!(facts.title, "T &amp; U");
        assert_eq!(facts.description, "D");
        assert_eq!(facts.og_title, "OT");
        assert!(!kept.contains("canonical"));
        assert!(!kept.contains("og:title"));
        assert!(!kept.contains("ld+json"));
        assert!(kept.contains("<title>T &amp; U</title>"));
        assert!(kept.contains("console.log('<b>')"));
        assert!(kept.contains("stylesheet"));
    }

    #[test]
    fn every_listed_page_has_a_unique_path_and_no_dash() {
        let mut seen = std::collections::BTreeSet::new();
        for p in PAGES {
            assert!(p.path.starts_with('/') && p.path.ends_with('/'), "{}", p.path);
            assert!(seen.insert(p.path), "duplicate {}", p.path);
            for t in [p.name, p.what, p.published] {
                assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'), "dash in {}", p.path);
            }
            let card = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../public").join(p.image.trim_start_matches('/'));
            assert!(card.exists(), "card {} missing", p.image);
        }
    }

    #[test]
    fn robots_names_every_agent_and_the_sitemap() {
        let r = robots_txt();
        for a in AI_AGENTS {
            assert!(r.contains(&format!("User-agent: {a}\nAllow: /\n")));
        }
        assert!(r.ends_with(&format!("Sitemap: {HOST}/sitemap.xml\n")));
    }

    #[test]
    fn llms_txt_follows_the_convention() {
        let t = llms_txt(&["Layer: STATE. note".into()], "ladder sentence");
        assert!(t.starts_with("# AienOS\n\n> "));
        assert!(t.contains("\n## Optional\n"));
        assert_eq!(t.matches("\n# ").count(), 0, "exactly one H1");
        for p in PAGES {
            assert!(t.contains(&format!("]({HOST}{}): ", p.path)), "missing {}", p.path);
        }
        let opt = t.find("## Optional").unwrap();
        assert!(t[opt..].contains("/licensing/"));
    }

    #[test]
    fn markdown_mirror_paths_follow_the_public_convention() {
        assert_eq!(md_path("/"), "/index.md");
        assert_eq!(md_path("/machine/"), "/machine.md");
        assert_eq!(md_path("/zh-CN/"), "/zh-CN/index.md");
        assert_eq!(md_path("/zh-CN/machine/"), "/zh-CN/machine.md");
        assert_eq!(md_path("/research/status/"), "/research/status.md");
    }

    #[test]
    fn discovery_files_link_every_list_item_in_an_h2_section() {
        for (name, doc) in [
            ("llms.txt", crate::components::status::llms_txt()),
            ("sitemap.md", sitemap_md(&[])),
        ] {
            let mut in_h2 = false;
            for line in doc.lines() {
                if line.starts_with("## ") {
                    in_h2 = true;
                } else if in_h2 && line.starts_with("- ") {
                    assert!(line.contains("]("), "{name} has an unlinked item in an H2 section: {line}");
                }
            }
        }
    }

    #[test]
    fn html_to_text_keeps_headings_links_and_drops_chrome() {
        let h = "<html><head><title>x</title></head><body><header class=\"aien-header\">NAV</header><main><h1>Title</h1><p>Para with <a href=\"/now/\">a link</a> and <script>bad()</script> more.</p><ul><li>one</li><li>two</li></ul></main><footer>F</footer></body></html>";
        let t = html_to_text(h);
        assert!(t.starts_with("# Title"));
        assert!(t.contains("[a link](https://www.aienos.com/now/)"));
        assert!(t.contains("- one"));
        assert!(!t.contains("NAV") && !t.contains("bad()") && !t.contains("F\n"));
    }

    #[test]
    fn feeds_are_well_formed_atom() {
        let ctx = BuildCtx { built: "2026-10-19T00:00:00Z".into(), head_committed: "2026-10-19T00:00:00Z".into(), ledger_written: "2026-10-19T16:43:00Z".into(), facts_generated: "2026-10-19T00:00:00Z".into() };
        let pages: Vec<Emitted> = PAGES.iter().map(|p| Emitted { path: p.path.into(), file: PathBuf::from("x"), lang: "en".into(), lastmod: "2026-10-18T00:00:00Z".into() }).collect();
        let f = site_feed(&pages);
        assert!(f.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">"));
        assert_eq!(f.matches("<entry>").count(), PAGES.iter().filter(|p| matches!(p.kind, Kind::Article | Kind::Paper)).count());
        let tsv = "id\topened_utc\towner\tstatus\titem\tdone_when\tevidence\tupdated_utc\nL001\t2026-10-19T16:00Z\tsa-1\tdone\tFix <x>\twhen y\tnote a | note b\t2026-10-19T16:43Z\nL002\t2026-10-19T16:00Z\tsa-1\topen\tOpen one\twhen z\t\t2026-10-19T16:44Z\n";
        let items = now::parse_ledger(tsv, now::parse_utc("2026-10-19T17:00Z").unwrap());
        let (plan, models) = crate::components::now::models_from(crate::components::now::ROSTER, &None, &[]);
        let n = Now { built_utc: "x".into(), ledger_written_utc: "2026-10-19T16:44Z".into(), items, pulse: vec![], models_checked_utc: "never".into(), plan, models, claude: None };
        let nf = now_feed(&n, &ctx);
        assert_eq!(nf.matches("<entry>").count(), 1);
        assert!(nf.contains("Fix &lt;x&gt;"));
        assert!(nf.contains("- note a"));
        assert!(nf.contains("<updated>2026-10-19T16:43:00Z</updated>"));
    }

    #[test]
    fn sitemap_carries_alternates_only_for_mirrored_pages() {
        let mk = |p: &str| Emitted { path: p.into(), file: PathBuf::from("x"), lang: if p.starts_with("/zh-CN/") { "zh-CN".into() } else { "en".into() }, lastmod: "2026-10-18T00:00:00Z".into() };
        let pages = vec![mk("/"), mk("/zh-CN/"), mk("/now/")];
        let s = sitemap_xml(&pages);
        assert_eq!(s.matches("<url>").count(), 3);
        assert_eq!(s.matches("hreflang=\"x-default\"").count(), 2);
        assert!(s.contains("<loc>https://www.aienos.com/now/</loc>"));
    }

    #[test]
    fn graph_for_a_paper_carries_the_doi() {
        let ctx = BuildCtx { built: "b".into(), head_committed: "h".into(), ledger_written: "l".into(), facts_generated: "f".into() };
        let page = Emitted { path: "/research/computing-machinery-and-understanding/".into(), file: PathBuf::from("x"), lang: "en".into(), lastmod: "2026-10-18T00:00:00Z".into() };
        let info = info_for(&page.path).unwrap();
        let facts = HeadFacts { title: "T".into(), description: "D".into(), ..Default::default() };
        let g = graph(&page, info, &facts, true, &ctx, Path::new("/nonexistent"));
        let s = g.to_string();
        assert!(s.contains("10.5281/zenodo.23437498"));
        assert!(s.contains("ScholarlyArticle"));
        assert!(s.contains("BreadcrumbList"));
        assert!(s.contains("workTranslation"));
    }
}
