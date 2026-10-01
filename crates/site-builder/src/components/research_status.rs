//! The canonical research status source for the site.
//!
//! Every research status shown on aienos.com that is rendered by this
//! builder comes from one data file, `data/research_status.json`, which
//! holds the rows of the consolidated research status table word for word.
//! The file is compiled in with `include_str!` and parsed with serde_json,
//! then validated. A malformed file, an unknown status word, or a ladder
//! that contradicts its own current level makes the build and the tests
//! fail. Statuses change when the evidence changes, not when the copy does.

use std::sync::OnceLock;

use maud::{html, Markup};
use serde::Deserialize;

/// The raw data file, compiled into the binary.
pub const RAW: &str = include_str!("../../data/research_status.json");

/// The only status words a Turing row may carry.
pub const ALLOWED_STATUSES: [&str; 7] = [
    "PASS",
    "FAIL",
    "INCOMPLETE",
    "BLOCKED",
    "PARTIAL",
    "NOT ESTABLISHED",
    "NOT STARTED",
];

/// The only states a ladder step may carry.
pub const ALLOWED_LADDER_STATES: [&str; 4] =
    ["achieved", "partial", "not achieved", "not attempted"];

/// The only classes an implementation row may carry, in display order.
pub const ALLOWED_CLASSES: [&str; 5] = [
    "Implemented",
    "Partially implemented",
    "Experimental",
    "Planned",
    "Research hypothesis",
];

/// The ladder levels, in order, exactly as the protocol defines them.
pub const LADDER_LEVELS: [&str; 8] = ["L0", "L1", "L2", "L3", "L4", "L5", "L6", "L7+"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusData {
    pub as_of: String,
    pub source_table: String,
    pub source_commits: SourceCommits,
    pub evidence_hierarchy: String,
    pub abbreviations: Vec<Abbreviation>,
    pub caveats: Vec<String>,
    pub turing_definition: String,
    pub allowed_statuses: Vec<String>,
    pub allowed_ladder_states: Vec<String>,
    pub allowed_classes: Vec<String>,
    pub current_level: CurrentLevel,
    pub ladder: Vec<LadderStep>,
    pub attempts: Vec<Attempt>,
    pub turing_rows: Vec<TuringRow>,
    pub implementation_rows: Vec<ImplementationRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCommits {
    pub omega: String,
    pub aienos: String,
    pub aien_architecture: String,
    pub aienos_com: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Abbreviation {
    pub short: String,
    pub long: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentLevel {
    pub level: String,
    pub summary: String,
    pub outside_scope: String,
    pub note: String,
    pub meaning: String,
    pub limitations: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LadderStep {
    pub level: String,
    pub name: String,
    pub state: String,
    pub detail: String,
    pub basis: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub id: String,
    pub beside: String,
    pub outcome: String,
    pub summary: String,
    pub led_to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuringRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub status_detail: String,
    pub plain: String,
    #[serde(default)]
    pub headline: Option<String>,
    #[serde(default)]
    pub not_yield: Option<String>,
    pub scope: String,
    pub claim_level: String,
    pub receipt: String,
    pub commit: String,
    pub date: String,
    pub limitations: String,
    pub next_dependency: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementationRow {
    pub id: String,
    pub class: String,
    pub status_detail: String,
    pub scope: String,
    pub claim_level: String,
    pub receipt: String,
    pub commit: String,
    pub date: String,
    pub limitations: String,
    pub next_dependency: String,
    pub evidence: String,
}

impl StatusData {
    pub fn turing_row(&self, id: &str) -> Option<&TuringRow> {
        self.turing_rows.iter().find(|r| r.id == id)
    }

    pub fn implementation_row(&self, id: &str) -> Option<&ImplementationRow> {
        self.implementation_rows.iter().find(|r| r.id == id)
    }

    pub fn step(&self, level: &str) -> Option<&LadderStep> {
        self.ladder.iter().find(|s| s.level == level)
    }
}

/// Parse and validate a data file. Used for the compiled-in file and,
/// in tests, for deliberately broken copies.
pub fn parse(raw: &str) -> Result<StatusData, String> {
    if raw.contains('\u{2014}') || raw.contains('\u{2013}') {
        return Err("research status data contains an em dash or en dash".to_string());
    }
    let data: StatusData =
        serde_json::from_str(raw).map_err(|e| format!("research status data does not parse: {e}"))?;
    validate(&data)?;
    Ok(data)
}

/// Structural and honesty checks on the data. Any failure stops the build.
pub fn validate(d: &StatusData) -> Result<(), String> {
    if d.allowed_statuses != ALLOWED_STATUSES {
        return Err("allowed_statuses in the data file differs from the fixed set".to_string());
    }
    if d.allowed_ladder_states != ALLOWED_LADDER_STATES {
        return Err("allowed_ladder_states in the data file differs from the fixed set".to_string());
    }
    if d.allowed_classes != ALLOWED_CLASSES {
        return Err("allowed_classes in the data file differs from the fixed set".to_string());
    }

    let levels: Vec<&str> = d.ladder.iter().map(|s| s.level.as_str()).collect();
    if levels != LADDER_LEVELS {
        return Err(format!("ladder levels must be exactly {:?}, found {:?}", LADDER_LEVELS, levels));
    }
    for s in &d.ladder {
        if !ALLOWED_LADDER_STATES.contains(&s.state.as_str()) {
            return Err(format!("ladder step {} has unknown state {:?}", s.level, s.state));
        }
    }

    // The stated current level must be the top of the unbroken run of
    // achieved steps from L0. A partial step above it does not count.
    let mut top: Option<&str> = None;
    for s in &d.ladder {
        if s.state == "achieved" {
            top = Some(s.level.as_str());
        } else {
            break;
        }
    }
    if top != Some(d.current_level.level.as_str()) {
        return Err(format!(
            "current_level {} does not match the highest unbroken achieved step {:?}",
            d.current_level.level, top
        ));
    }
    // No step above a gap may claim to be achieved.
    let first_gap = d.ladder.iter().position(|s| s.state != "achieved");
    if let Some(g) = first_gap {
        if let Some(s) = d.ladder[g..].iter().find(|s| s.state == "achieved") {
            return Err(format!("ladder step {} is marked achieved above an unachieved step", s.level));
        }
    }

    let mut seen: Vec<&str> = Vec::new();
    for r in &d.turing_rows {
        if seen.contains(&r.id.as_str()) {
            return Err(format!("turing row {} appears more than once", r.id));
        }
        seen.push(&r.id);
        if !ALLOWED_STATUSES.contains(&r.status.as_str()) {
            return Err(format!("turing row {} has status {:?} outside the allowed set", r.id, r.status));
        }
    }

    let mut seen_impl: Vec<&str> = Vec::new();
    for r in &d.implementation_rows {
        if seen_impl.contains(&r.id.as_str()) {
            return Err(format!("implementation row {} appears more than once", r.id));
        }
        seen_impl.push(&r.id);
        if !ALLOWED_CLASSES.contains(&r.class.as_str()) {
            return Err(format!("implementation row {} has class {:?} outside the allowed set", r.id, r.class));
        }
    }

    for a in &d.attempts {
        let row = d
            .turing_row(&a.id)
            .ok_or_else(|| format!("attempt {} has no matching turing row", a.id))?;
        if row.status != a.outcome {
            return Err(format!("attempt {} outcome {} disagrees with its row status {}", a.id, a.outcome, row.status));
        }
        if d.step(&a.beside).is_none() {
            return Err(format!("attempt {} sits beside unknown level {}", a.id, a.beside));
        }
        if d.turing_row(&a.led_to).is_none() {
            return Err(format!("attempt {} leads to unknown row {}", a.id, a.led_to));
        }
    }
    Ok(())
}

/// The compiled-in data, parsed once. Panics (failing the build run and
/// every test that touches it) if the file is broken.
pub fn data() -> &'static StatusData {
    static DATA: OnceLock<StatusData> = OnceLock::new();
    DATA.get_or_init(|| parse(RAW).unwrap_or_else(|e| panic!("{e}")))
}

fn ty2() -> &'static TuringRow {
    data().turing_row("TY-2").expect("TY-2 row missing from research status data")
}

// ------------------------------------------------------------------
// Lookups used by the older hand-written components (status, evidence,
// research). They return &'static str so those components keep their
// existing shapes.
// ------------------------------------------------------------------

pub fn ty2_status() -> &'static str {
    ty2().status.as_str()
}
pub fn ty2_title() -> &'static str {
    ty2().title.as_str()
}
pub fn ty2_headline() -> &'static str {
    ty2().headline.as_deref().expect("TY-2 row needs a headline")
}
pub fn ty2_plain() -> &'static str {
    ty2().plain.as_str()
}
pub fn ty2_date() -> &'static str {
    ty2().date.as_str()
}
pub fn ty2_receipt() -> &'static str {
    ty2().receipt.as_str()
}
pub fn ty2_scope() -> &'static str {
    ty2().scope.as_str()
}
pub fn ty2_limitations() -> &'static str {
    ty2().limitations.as_str()
}

/// Map an implementation class onto the homepage layer vocabulary
/// (IMPLEMENTED, QUALIFIED, EXPERIMENTAL, BLOCKED, PLANNED). Never maps
/// anything to QUALIFIED: no row in the table carries that word.
pub fn layer_status_for_class(class: &str) -> &'static str {
    match class {
        "Implemented" => "IMPLEMENTED",
        "Partially implemented" | "Experimental" => "EXPERIMENTAL",
        _ => "PLANNED",
    }
}

fn instrument_row(d: &StatusData) -> &ImplementationRow {
    d.implementation_row("Turing measurement machinery")
        .expect("Turing measurement machinery row missing")
}

/// Homepage layer status for the Turing instrument, from a given data set.
pub fn turing_instrument_status_for(d: &StatusData) -> &'static str {
    layer_status_for_class(&instrument_row(d).class)
}

pub fn turing_instrument_status() -> &'static str {
    turing_instrument_status_for(data())
}

/// One sentence for the homepage layer list: the instrument row plus the
/// current level, its limits and the TY-2 gain, all from a given data set.
pub fn turing_instrument_note_for(d: &StatusData) -> String {
    let row = instrument_row(d);
    let t = d.turing_row("TY-2").expect("TY-2 row missing from research status data");
    format!(
        "{}. Current level: {}. {} {} TY-2 recorded {}. {}",
        row.status_detail,
        d.current_level.summary,
        d.current_level.limitations,
        d.current_level.outside_scope,
        t.headline.as_deref().unwrap_or(""),
        t.not_yield.as_deref().unwrap_or("")
    )
    .trim_end()
    .to_string()
}

pub fn turing_instrument_note() -> &'static str {
    static NOTE: OnceLock<String> = OnceLock::new();
    NOTE.get_or_init(|| turing_instrument_note_for(data()))
}

/// Experiment roll call for the homepage layer list, e.g.
/// "EXP-001 FAIL, EXP-001R PASS, ... EXP-003 BLOCKED."
pub fn experiments_note_for(d: &StatusData) -> String {
    let parts: Vec<String> = d
        .turing_rows
        .iter()
        .filter(|r| r.id.starts_with("EXP-"))
        .map(|r| format!("{} {}", r.id, r.status))
        .collect();
    format!("{}. Failures stay on the record.", parts.join(", "))
}

pub fn experiments_note() -> &'static str {
    static NOTE: OnceLock<String> = OnceLock::new();
    NOTE.get_or_init(|| experiments_note_for(data()))
}

/// The research paper card line on the homepage.
pub fn paper_note() -> &'static str {
    static NOTE: OnceLock<String> = OnceLock::new();
    NOTE.get_or_init(|| {
        let d = data();
        format!("The formal paper. {} Current level: {}.", d.turing_definition, d.current_level.summary)
    })
}

/// The current level in one sentence, for embedding in other pages.
pub fn current_level_sentence() -> &'static str {
    static NOTE: OnceLock<String> = OnceLock::new();
    NOTE.get_or_init(|| {
        let c = &data().current_level;
        format!("Current level: {}. {} {}", c.summary, c.outside_scope, c.note)
    })
}

// ------------------------------------------------------------------
// Rendering
// ------------------------------------------------------------------

fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn badge_modifier(status: &str) -> &'static str {
    match status {
        "PASS" => "pass",
        "FAIL" => "fail",
        "INCOMPLETE" => "incomplete",
        "BLOCKED" => "blocked",
        "PARTIAL" => "partial",
        "NOT ESTABLISHED" => "not-established",
        "NOT STARTED" => "not-started",
        _ => "unknown",
    }
}

fn badge(status: &str) -> Markup {
    html! {
        span class=(format!("rs-badge rs-badge--{}", badge_modifier(status))) {
            (status)
        }
    }
}

/// The research ladder as a staircase. The list is in reading order
/// L0 to L7+ for screen readers; CSS draws it climbing upward.
pub fn render_ladder(d: &StatusData) -> Markup {
    let current = d.current_level.level.as_str();
    html! {
        div class="rs-ladder" {
            p class="rs-current" {
                strong { "Current level: " (d.current_level.summary) "." }
                " " (d.current_level.outside_scope) " " (d.current_level.note)
            }
            p class="rs-current-meaning" { (d.current_level.meaning) }
            p class="rs-ladder-hint" { "Read from the bottom up. Each step rests on the one below it. Filled steps are achieved, half-filled steps are partial, hollow steps are not achieved." }
            ol class="rs-stairs" {
                @for (i, step) in d.ladder.iter().enumerate() {
                    li class=(format!("rs-step rs-step--{}", slug(&step.state))) data-level=(step.level) data-state=(step.state) style=(format!("--i: {};", i)) {
                        span class="rs-step-mark" aria-hidden="true" {}
                        div class="rs-step-body" {
                            div class="rs-step-head" {
                                span class="rs-step-level" { (step.level) }
                                span class="rs-step-name" { (step.name) }
                                @if step.level == current {
                                    span class="rs-here" { "Current level" }
                                }
                            }
                            p class="rs-step-state" { (step.state) }
                            p class="rs-step-detail" { (step.detail) }
                            p class="rs-step-basis" { "Evidence: " (step.basis) }
                            @for a in d.attempts.iter().filter(|a| a.beside == step.level) {
                                div class="rs-attempt" data-attempt=(a.id) {
                                    div class="rs-attempt-head" {
                                        (badge(&a.outcome))
                                        strong { (a.id) }
                                        span { "failed attempt, kept on the record" }
                                    }
                                    p { (a.summary) " Led to " (a.led_to) "." }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One card per Turing row.
pub fn render_cards(d: &StatusData) -> Markup {
    html! {
        div class="rs-cards" {
            @for r in &d.turing_rows {
                article class=(format!("rs-card rs-card--{}", badge_modifier(&r.status))) id=(format!("rs-{}", slug(&r.id))) data-row=(r.id) data-status=(r.status) {
                    div class="rs-card-top" {
                        (badge(&r.status))
                        span class="rs-card-id" { (r.id) }
                    }
                    h3 class="rs-card-title" data-role="result-label" { (r.title) }
                    @if let Some(h) = &r.headline {
                        p class="rs-card-headline" data-role="result-label" { (h) }
                    }
                    @if let Some(n) = &r.not_yield {
                        p class="rs-card-callout" { (n) }
                    }
                    p class="rs-card-plain" { (r.plain) }
                    dl class="rs-facts" {
                        div { dt { "Status" } dd { (r.status_detail) } }
                        div { dt { "Scope" } dd { (r.scope) } }
                        div { dt { "Claim level" } dd { (r.claim_level) } }
                        div { dt { "Limitations" } dd { (r.limitations) } }
                        div { dt { "Next dependency" } dd { (r.next_dependency) } }
                        div { dt { "Evidence" } dd class="rs-mono" { (r.evidence) } }
                        div { dt { "Receipt" } dd class="rs-mono" { (r.receipt) } }
                        div { dt { "Commit" } dd class="rs-mono" { (r.commit) } }
                        div { dt { "Date" } dd { (r.date) } }
                    }
                }
            }
        }
    }
}

fn class_blurb(class: &str) -> &'static str {
    match class {
        "Implemented" => "Code and tests exist. Each entry says how far that goes.",
        "Partially implemented" => "Some of it exists. The missing part is named.",
        "Experimental" => "Being tried. Results so far, including failures, are on the record.",
        "Planned" => "Written down as a plan or specification. No code yet.",
        "Research hypothesis" => "Not built. An idea to test, not a feature.",
        _ => "",
    }
}

/// The implementation list, grouped by class. Research hypotheses get
/// their own visibly different group.
pub fn render_implementation(d: &StatusData) -> Markup {
    html! {
        div class="rs-impl" {
            @for class in ALLOWED_CLASSES {
                @let rows = d.implementation_rows.iter().filter(|r| r.class == class).collect::<Vec<&ImplementationRow>>();
                @if !rows.is_empty() {
                    section class=(format!("rs-group rs-group--{}", slug(class))) data-class=(class) {
                        h3 class="rs-group-title" { (class) span class="rs-group-count" { " (" (rows.len()) ")" } }
                        p class="rs-group-blurb" { (class_blurb(class)) }
                        ul class="rs-impl-list" {
                            @for r in &rows {
                                li class="rs-impl-item" data-impl=(r.id) {
                                    div class="rs-impl-head" {
                                        span class="rs-impl-name" { (r.id) }
                                        span class="rs-impl-status" { (r.status_detail) }
                                    }
                                    p class="rs-impl-scope" { (r.scope) }
                                    details class="rs-impl-more" {
                                        summary { "Limits and evidence" }
                                        dl class="rs-facts" {
                                            div { dt { "Claim level" } dd { (r.claim_level) } }
                                            div { dt { "Limitations" } dd { (r.limitations) } }
                                            div { dt { "Next dependency" } dd { (r.next_dependency) } }
                                            div { dt { "Evidence" } dd class="rs-mono" { (r.evidence) } }
                                            div { dt { "Receipt" } dd class="rs-mono" { (r.receipt) } }
                                            div { dt { "Commit" } dd class="rs-mono" { (r.commit) } }
                                            div { dt { "Date" } dd { (r.date) } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Compact block other pages can embed: the current level and the ladder,
/// with a link to the full status page.
pub fn render_research_status_summary() -> Markup {
    let d = data();
    html! {
        div class="rs-embed" {
            (render_ladder(d))
            p class="rs-embed-link" {
                a href="/research/status/" { "Every experiment, its receipt and its limits" }
            }
        }
    }
}

/// Body of the generated /research/status/ page.
pub fn render_research_status_page() -> Markup {
    let d = data();
    let c = &d.source_commits;
    html! {
        div class="container rs-page" {
            header class="rs-page-head" {
                p class="rs-kicker" { "Research status" }
                h1 class="rs-page-title" { "Where the research stands" }
                p class="rs-standfirst" { (d.turing_definition) }
                p class="rs-standfirst" {
                    "Every status on this page is rendered from one data file, built from the consolidated research status table. Failed and incomplete experiments are shown, not hidden. "
                    "As of " (d.as_of) "."
                }
                nav class="rs-toc" aria-label="On this page" {
                    a href="#ladder" { "The ladder" }
                    a href="#experiments" { "Experiments" }
                    a href="#implementation" { "What is built" }
                    a href="#sources" { "Sources" }
                    a href="/research-status.json" { "Data file (JSON)" }
                }
            }

            section id="ladder" class="rs-section" {
                h2 class="rs-h2" { "The research ladder" }
                (render_ladder(d))
            }

            section id="experiments" class="rs-section" {
                h2 class="rs-h2" { "Experiment status" }
                p class="rs-section-lede" {
                    "One card per result. The badge is the verdict; the lines below say what it covers, what it does not, and what it waits on."
                }
                (render_cards(d))
            }

            section id="implementation" class="rs-section" {
                h2 class="rs-h2" { "What is built and what is still an idea" }
                p class="rs-section-lede" {
                    "Implemented work and research hypotheses are kept apart. A hypothesis listed here is a question, not a feature."
                }
                (render_implementation(d))
            }

            section id="sources" class="rs-section rs-sources" {
                h2 class="rs-h2" { "Sources and caveats" }
                p { (d.source_table) "." }
                p {
                    "Repository commits read: omega " span class="rs-mono" { (c.omega) }
                    ", aienos " span class="rs-mono" { (c.aienos) }
                    ", aien-architecture " span class="rs-mono" { (c.aien_architecture) }
                    ", aienos.com " span class="rs-mono" { (c.aienos_com) } "."
                }
                p { "Evidence ranks: " span class="rs-mono" { (d.evidence_hierarchy) } "." }
                ul {
                    @for cv in &d.caveats {
                        li { (cv) }
                    }
                    li { (d.current_level.limitations) " Evidence for the current level: " span class="rs-mono" { (d.current_level.evidence) } }
                }
                h3 class="rs-h3" { "Abbreviations" }
                dl class="rs-facts" {
                    @for a in &d.abbreviations {
                        div { dt { (a.short) } dd class="rs-mono" { (a.long) } }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turing row ids from the consolidated table, Table 1.
    const TABLE_TURING_IDS: [&str; 13] = [
        "CAL-0",
        "EXP-001",
        "EXP-001R",
        "TY-2",
        "EXP-002A",
        "EXP-002B",
        "EXP-002C",
        "EXP-002D",
        "EXP-003",
        "ENERGY-ATTRIBUTION",
        "TURING-YIELD-TJ",
        "BROWNIAN-CEILING",
        "GENERAL-DISCOVERY",
    ];

    /// Ladder states from the consolidated table. Changing a state in the
    /// data file without changing it here fails `ladder_matches_table`.
    const TABLE_LADDER: [(&str, &str); 8] = [
        ("L0", "achieved"),
        ("L1", "achieved"),
        ("L2", "achieved"),
        ("L3", "not achieved"),
        ("L4", "not achieved"),
        ("L5", "partial"),
        ("L6", "not achieved"),
        ("L7+", "not attempted"),
    ];

    fn real() -> StatusData {
        parse(RAW).expect("real data must parse")
    }

    /// The markup of one ladder step, from its opening attribute to </li>.
    fn step_html<'a>(html: &'a str, level: &str) -> &'a str {
        let key = format!("data-level=\"{level}\"");
        let start = html.find(&key).unwrap_or_else(|| panic!("no step {level}"));
        let end = html[start..].find("</li>").expect("step not closed") + start;
        &html[start..end]
    }

    fn card_html<'a>(html: &'a str, id: &str) -> &'a str {
        let key = format!("data-row=\"{id}\"");
        let start = html.find(&key).unwrap_or_else(|| panic!("no card {id}"));
        let end = html[start..].find("</article>").expect("card not closed") + start;
        &html[start..end]
    }

    fn result_labels(card: &str) -> Vec<String> {
        let key = "data-role=\"result-label\">";
        let mut out = Vec::new();
        let mut rest = card;
        while let Some(i) = rest.find(key) {
            let after = &rest[i + key.len()..];
            let end = after.find('<').unwrap_or(after.len());
            out.push(after[..end].to_string());
            rest = &after[end..];
        }
        out
    }

    fn achieved_levels_in(html: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = html;
        while let Some(i) = rest.find("data-level=\"") {
            let after = &rest[i + "data-level=\"".len()..];
            let end = after.find('"').unwrap();
            let level = after[..end].to_string();
            let tail = &after[end..];
            if tail.starts_with("\" data-state=\"achieved\"") {
                out.push(level);
            }
            rest = tail;
        }
        out
    }

    /// The "not achieved" check, as a function so a mutated copy can be
    /// shown to fail it.
    fn check_not_achieved(html: &str) -> Result<(), String> {
        for level in ["L3", "L4", "L6"] {
            if !step_html(html, level).contains(">not achieved<") {
                return Err(format!("{level} is not rendered as not achieved"));
            }
        }
        Ok(())
    }

    /// The TY-2 label check, as a function so a mutated copy can be shown
    /// to fail it.
    fn check_ty2_card(html: &str) -> Result<(), String> {
        let card = card_html(html, "TY-2");
        let labels = result_labels(card);
        if labels.is_empty() {
            return Err("TY-2 card has no result label".to_string());
        }
        if labels.iter().any(|l| l.to_lowercase().contains("turing yield")) {
            return Err("TY-2 result label says Turing yield".to_string());
        }
        if !labels.iter().any(|l| l.contains("gain")) {
            return Err("TY-2 result label does not say gain".to_string());
        }
        if !card.contains("T/J") || !card.contains("not established") {
            return Err("TY-2 card does not say T/J is not established".to_string());
        }
        Ok(())
    }

    #[test]
    fn data_file_parses() {
        let d = real();
        assert_eq!(d.as_of, "2026-10-01");
        assert_eq!(d.source_commits.omega, "38919c8");
        // Counterexample: a truncated file must not parse.
        assert!(parse(&RAW[..RAW.len() / 2]).is_err());
        // Counterexample: an unknown field must not parse.
        let extra = RAW.replacen("\"as_of\"", "\"surprise\": 1, \"as_of\"", 1);
        assert!(parse(&extra).is_err());
    }

    #[test]
    fn every_table_turing_row_present_exactly_once() {
        let d = real();
        for id in TABLE_TURING_IDS {
            let n = d.turing_rows.iter().filter(|r| r.id == id).count();
            assert_eq!(n, 1, "row {id} appears {n} times");
        }
        assert_eq!(d.turing_rows.len(), TABLE_TURING_IDS.len(), "unexpected extra turing rows");
        // Counterexample: duplicating a row must be rejected.
        let mut dup = d.clone();
        dup.turing_rows.push(dup.turing_rows[0].clone());
        assert!(validate(&dup).is_err());
    }

    #[test]
    fn no_status_outside_allowed_set() {
        let d = real();
        for r in &d.turing_rows {
            assert!(ALLOWED_STATUSES.contains(&r.status.as_str()), "{} has {}", r.id, r.status);
        }
        for r in &d.implementation_rows {
            assert!(ALLOWED_CLASSES.contains(&r.class.as_str()), "{} has {}", r.id, r.class);
        }
        // Counterexample: the old site word VERIFIED is not allowed.
        let mut bad = d.clone();
        bad.turing_rows.iter_mut().find(|r| r.id == "TY-2").unwrap().status = "VERIFIED".into();
        assert!(validate(&bad).is_err());
        // Counterexample: an implementation class outside the set.
        let mut bad2 = d.clone();
        bad2.implementation_rows[0].class = "Qualified".into();
        assert!(validate(&bad2).is_err());
    }

    const NEW_ROW_IDS: [&str; 6] = [
        "Dirac (DIRAC-0 and sealed exam ladder)",
        "AEGIS",
        "ARGUS (ARGUS-0, ARGUS-1)",
        "FORGE (v1 and V2)",
        "ATLAS (M1 ATLAS_BOOT and roadmap)",
        "Typed result contracts",
    ];

    fn new_rows_have_evidence(d: &StatusData) -> Result<(), String> {
        for id in NEW_ROW_IDS {
            let n = d.implementation_rows.iter().filter(|r| r.id == id).count();
            if n != 1 {
                return Err(format!("row {id} appears {n} times"));
            }
            let row = d.implementation_row(id).unwrap();
            if row.evidence.trim().is_empty() {
                return Err(format!("row {id} has no evidence"));
            }
        }
        Ok(())
    }

    #[test]
    fn new_implementation_rows_present_once_with_evidence() {
        let d = real();
        new_rows_have_evidence(&d).unwrap();
        // Counterexample: one row with its evidence blanked must fail.
        let mut bad = d.clone();
        bad.implementation_rows
            .iter_mut()
            .find(|r| r.id == "AEGIS")
            .unwrap()
            .evidence = String::new();
        assert!(new_rows_have_evidence(&bad).is_err());
    }

    #[test]
    fn ladder_matches_table() {
        let d = real();
        for (level, state) in TABLE_LADDER {
            assert_eq!(d.step(level).unwrap().state, state, "ladder {level}");
        }
        assert_eq!(d.current_level.level, "L2");
        // Counterexample: flipping L3 to achieved breaks the current level rule.
        let mut up = d.clone();
        up.ladder[3].state = "achieved".into();
        assert!(validate(&up).is_err());
        // Counterexample: claiming L6 while L3 is open is rejected.
        let mut skip = d.clone();
        skip.ladder[6].state = "achieved".into();
        assert!(validate(&skip).is_err());
    }

    #[test]
    fn rendered_ladder_marks_exactly_the_achieved_levels() {
        let d = real();
        let html = render_ladder(&d).into_string();
        let want: Vec<String> = d
            .ladder
            .iter()
            .filter(|s| s.state == "achieved")
            .map(|s| s.level.clone())
            .collect();
        assert_eq!(achieved_levels_in(&html), want);
        assert_eq!(want, vec!["L0", "L1", "L2"]);
        check_not_achieved(&html).unwrap();
        assert!(html.contains("Current level: L2, Brownian scope only, instrument certification"));

        // Counterexample: a test copy with L3 flipped to achieved renders a
        // different achieved set and fails the not-achieved check.
        let mut up = d.clone();
        up.ladder[3].state = "achieved".into();
        let bad = render_ladder(&up).into_string();
        assert_ne!(achieved_levels_in(&bad), vec!["L0", "L1", "L2"]);
        assert!(check_not_achieved(&bad).is_err());
    }

    #[test]
    fn exp001_failure_is_on_the_ladder_and_its_card() {
        let d = real();
        let ladder = render_ladder(&d).into_string();
        let l1 = step_html(&ladder, "L1");
        assert!(l1.contains("data-attempt=\"EXP-001\""));
        assert!(l1.contains(">FAIL<"));
        assert!(l1.contains("EXP-001R"));
        let cards = render_cards(&d).into_string();
        let card = card_html(&cards, "EXP-001");
        assert!(card.contains("data-status=\"FAIL\""));
        assert!(card.contains(">FAIL<"));
        // Counterexample: an attempt whose outcome disagrees with its row.
        let mut hidden = d.clone();
        hidden.turing_rows.iter_mut().find(|r| r.id == "EXP-001").unwrap().status = "PASS".into();
        assert!(validate(&hidden).is_err());
    }

    #[test]
    fn ty2_card_says_gain_not_yield() {
        let d = real();
        let html = render_cards(&d).into_string();
        check_ty2_card(&html).unwrap();
        // Counterexample: relabelling TY-2 as yield fails the check.
        let mut bad = d.clone();
        bad.turing_rows.iter_mut().find(|r| r.id == "TY-2").unwrap().title = "TY-2 Turing yield".into();
        assert!(check_ty2_card(&render_cards(&bad).into_string()).is_err());
        // Counterexample: dropping the not-yield statement fails the check.
        let mut quiet = d.clone();
        quiet.turing_rows.iter_mut().find(|r| r.id == "TY-2").unwrap().not_yield = None;
        assert!(check_ty2_card(&render_cards(&quiet).into_string()).is_err());
    }

    /// Research hypothesis rows from the consolidated table, Table 2.
    const TABLE_HYPOTHESES: [&str; 5] = [
        "General causal discovery",
        "Full active experimentation",
        "Equality saturation",
        "General representation discovery",
        "SUSY / Zeta rediscovery",
    ];

    fn group_html<'a>(html: &'a str, class: &str) -> Option<&'a str> {
        let key = format!("data-class=\"{class}\"");
        let start = html.find(&key)?;
        let end = html[start..].find("</section>")? + start;
        Some(&html[start..end])
    }

    /// The grouping check, as a function so a mutated copy can be shown
    /// to fail it: every table hypothesis renders inside the hypothesis
    /// group and inside no other group.
    fn check_grouping(html: &str) -> Result<(), String> {
        let hyp = group_html(html, "Research hypothesis").ok_or("hypothesis group missing")?;
        for id in TABLE_HYPOTHESES {
            let marker = format!("data-impl=\"{}\"", id.replace('/', "&#x2F;"));
            let plain = format!("data-impl=\"{id}\"");
            let inside = hyp.contains(&plain) || hyp.contains(&marker);
            if !inside {
                return Err(format!("{id} is not in the research hypothesis group"));
            }
            for class in ALLOWED_CLASSES.iter().filter(|c| **c != "Research hypothesis") {
                if let Some(g) = group_html(html, class) {
                    if g.contains(&plain) || g.contains(&marker) {
                        return Err(format!("{id} also renders under {class}"));
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn research_hypotheses_are_grouped_apart() {
        let d = real();
        check_grouping(&render_implementation(&d).into_string()).unwrap();
        // Counterexample: a copy that reclassifies SUSY / Zeta as
        // Implemented renders it in the wrong group and fails the check.
        let mut bad = d.clone();
        bad.implementation_rows
            .iter_mut()
            .find(|r| r.id == "SUSY / Zeta rediscovery")
            .unwrap()
            .class = "Implemented".into();
        assert!(check_grouping(&render_implementation(&bad).into_string()).is_err());
    }

    #[test]
    fn rendered_output_has_no_em_or_en_dash() {
        let page = render_research_status_page().into_string();
        let embed = render_research_status_summary().into_string();
        for s in [
            page.as_str(),
            embed.as_str(),
            turing_instrument_note(),
            experiments_note(),
            paper_note(),
            current_level_sentence(),
        ] {
            assert!(!s.contains('\u{2014}'), "em dash in rendered output");
            assert!(!s.contains('\u{2013}'), "en dash in rendered output");
        }
        // Counterexample: a data file carrying an em dash is rejected.
        let dashed = RAW.replacen("Definition", "Definition \u{2014} zero", 1);
        assert!(parse(&dashed).is_err());
    }

    /// The homepage lookup check, as a function so a mutated copy can be
    /// shown to fail it.
    fn check_homepage(d: &StatusData) -> Result<(), String> {
        if turing_instrument_status_for(d) != "IMPLEMENTED" {
            return Err(format!("instrument status is {}, table says Implemented", turing_instrument_status_for(d)));
        }
        let note = turing_instrument_note_for(d);
        if note.to_lowercase().contains("turing yield") || !note.contains("gain") {
            return Err("instrument note does not say gain, or says Turing yield".to_string());
        }
        if !note.contains("checked against the private sealed records on 2026-10-01") {
            return Err("instrument note drops the sealed-records check statement".to_string());
        }
        let exp = experiments_note_for(d);
        for want in ["EXP-001 FAIL", "EXP-002D INCOMPLETE", "EXP-003 BLOCKED"] {
            if !exp.contains(want) {
                return Err(format!("experiments note is missing {want}"));
            }
        }
        Ok(())
    }

    #[test]
    fn homepage_lookups_never_upgrade() {
        let d = real();
        check_homepage(&d).unwrap();
        assert_eq!(turing_instrument_note(), turing_instrument_note_for(&d));
        assert_eq!(experiments_note(), experiments_note_for(&d));
        assert_ne!(layer_status_for_class("Research hypothesis"), "IMPLEMENTED");
        // Counterexample: instrument row reclassified as a hypothesis.
        let mut hyp = d.clone();
        hyp.implementation_rows
            .iter_mut()
            .find(|r| r.id == "Turing measurement machinery")
            .unwrap()
            .class = "Research hypothesis".into();
        assert!(check_homepage(&hyp).is_err());
        // Counterexample: EXP-003 quietly upgraded to PASS.
        let mut up = d.clone();
        up.turing_rows.iter_mut().find(|r| r.id == "EXP-003").unwrap().status = "PASS".into();
        assert!(check_homepage(&up).is_err());
        // Counterexample: TY-2 headline relabelled as yield.
        let mut y = d.clone();
        y.turing_rows.iter_mut().find(|r| r.id == "TY-2").unwrap().headline =
            // Built from two pieces so the repo-wide stale scan (status.rs)
            // keeps covering this file without flagging this counterexample.
            Some(["2,559,679.825 bits of Turing", " yield"].concat());
        assert!(check_homepage(&y).is_err());
        // Counterexample: the sealed-records check statement removed.
        let mut s = d.clone();
        s.current_level.limitations = "L2 rests on private sealed records.".into();
        assert!(check_homepage(&s).is_err());
    }

    /// EXP-002A, B and C must each point at an aien-sealed commit.
    fn check_sealed_pointers(d: &StatusData) -> Result<(), String> {
        for (id, commits) in [
            ("EXP-002A", &["760bfe9"][..]),
            ("EXP-002B", &["1310e06", "111a5ea"][..]),
            ("EXP-002C", &["f546162", "111a5ea"][..]),
        ] {
            let r = d.turing_row(id).ok_or_else(|| format!("{id} row missing"))?;
            if !r.receipt.contains("aien-sealed") {
                return Err(format!("{id} receipt does not name aien-sealed"));
            }
            for c in commits {
                if !r.receipt.contains(c) {
                    return Err(format!("{id} receipt lacks aien-sealed commit {c}"));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn exp_002_rows_carry_sealed_commit_pointers() {
        let d = real();
        check_sealed_pointers(&d).unwrap();
        // Counterexample: each row in turn loses its pointers.
        for id in ["EXP-002A", "EXP-002B", "EXP-002C"] {
            let mut bad = d.clone();
            bad.turing_rows.iter_mut().find(|r| r.id == id).unwrap().receipt =
                "PRIVATE aien-sealed R3-49 (not read)".into();
            assert!(check_sealed_pointers(&bad).is_err(), "{id} without commit passed");
        }
    }
}
