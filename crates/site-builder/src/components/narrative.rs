//! Shared pieces for the research narrative pages (/machine/ and /discovery/).
//!
//! Every status word on these pages (levels, PASS/FAIL style verdicts and
//! implementation classes) comes from the one canonical record,
//! data/research_status.json, through components::research_status. This
//! module only turns a data row into a badge; it never names a status itself.

use maud::{html, Markup};

use super::research_status::{ImplementationRow, TuringRow};

/// The research doctrine line (synthesis handoff sections 2 and 22).
pub const SLOGAN: &str =
    "Weights suggest. Programs explain. Verification decides. Evidence teaches. The Turing keeps score.";

/// The Turing definition (synthesis handoff section 3).
pub const TURING_FORMULA: &str = "T(M; B, D, P) = [L(B) + L(D | B)] - [L(M) + L(D | M)]";

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

/// Badge for a Turing or experiment row. Text and modifier both come from
/// the row's status, so the shared rs-badge styles apply unchanged.
pub fn status_badge(row: &TuringRow) -> Markup {
    html! {
        span class=(format!("rs-badge rs-badge--{}", slug(&row.status))) data-row=(row.id) {
            span class="visually-hidden" { "Status: " }
            (row.status)
        }
    }
}

/// Badge for an implementation row. The text is the row's class from the
/// data; the modifier gives each class its own border style, so the class
/// never depends on colour alone.
pub fn class_badge(row: &ImplementationRow) -> Markup {
    html! {
        span class=(format!("rs-badge rs-class--{}", slug(&row.class))) data-impl=(row.id) {
            span class="visually-hidden" { "Status: " }
            (row.class)
        }
    }
}

#[cfg(test)]
pub mod check {
    //! Test helper: find status words in page markup that the data-driven
    //! components did not produce.

    /// Status words that only the data may supply. Upper-case verdicts are
    /// matched exactly; class labels are matched as words in either case.
    const VERDICTS: [&str; 7] =
        ["PASS", "FAIL", "BLOCKED", "INCOMPLETE", "PARTIAL", "NOT STARTED", "NOT ESTABLISHED"];
    const CLASS_WORDS: [&str; 5] = ["implemented", "planned", "experimental", "hypothesis", "achieved"];
    /// Lower-case uses allowed in fixed prose: the mission line calls AIENOS
    /// "an owned experimental machine", which describes the project, not a
    /// status; Cortex keeps "experimental conditions" as evidence; the Zeta
    /// exam names the Riemann hypothesis; the nature section says "Science
    /// runs hypothesis, experiment and theory". None of these is a status.
    const ALLOWED_LOWER: [&str; 4] =
        ["owned experimental machine", "experimental conditions", "Riemann hypothesis", "Science runs hypothesis"];

    fn strip_tags(s: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for ch in s.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    out.push(' ');
                }
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        out
    }

    /// Remove every occurrence of each component rendering from `page`, then
    /// return the hand-written status words that remain.
    pub fn stray_status_words(page: &str, components: &[String]) -> Vec<String> {
        let mut rest = page.to_string();
        for c in components.iter().filter(|c| !c.is_empty()) {
            rest = rest.replace(c.as_str(), " ");
        }
        let mut text = strip_tags(&rest);
        for a in ALLOWED_LOWER {
            text = text.replace(a, " ");
        }
        let mut found = Vec::new();
        for v in VERDICTS {
            if text.contains(v) {
                found.push(v.to_string());
            }
        }
        for w in text.split(|c: char| !c.is_ascii_alphanumeric()) {
            let lw = w.to_ascii_lowercase();
            if CLASS_WORDS.contains(&lw.as_str()) {
                found.push(w.to_string());
            }
            let b = w.as_bytes();
            if b.len() == 2 && b[0] == b'L' && b[1].is_ascii_digit() {
                found.push(w.to_string());
            }
        }
        found
    }
}
