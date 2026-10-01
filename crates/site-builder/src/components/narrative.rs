//! Shared pieces for the research narrative pages (/machine/ and /discovery/).
//!
//! Status labels on these pages come ONLY from the verified implementation
//! table in ~/handoffs/2026-10-01-NARR-IMPL-scout.md. Planned and hypothesis
//! items are written in future tense or as open questions, never as present
//! capabilities.

use maud::{html, Markup};

/// The research doctrine line (synthesis handoff sections 2 and 22).
pub const SLOGAN: &str =
    "Weights suggest. Programs explain. Verification decides. Evidence teaches. The Turing keeps score.";

/// The Turing definition (synthesis handoff section 3).
pub const TURING_FORMULA: &str = "T(M; B, D, P) = [L(B) + L(D | B)] - [L(M) + L(D | M)]";

/// TY-2 result. This is a Turing GAIN in bits. It is never a yield (T/J).
pub const TY2_GAIN_BITS: &str = "2,559,679.825";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Implemented,
    Partial,
    Experimental,
    Planned,
    Hypothesis,
}

impl Status {
    pub fn class(self) -> &'static str {
        match self {
            Status::Implemented => "status-implemented",
            Status::Partial => "status-partial",
            Status::Experimental => "status-experimental",
            Status::Planned => "status-planned",
            Status::Hypothesis => "status-hypothesis",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Implemented => "Implemented",
            Status::Partial => "Partial",
            Status::Experimental => "Experimental",
            Status::Planned => "Planned",
            Status::Hypothesis => "Hypothesis",
        }
    }
}

/// A text badge. The label text carries the meaning; the class adds a
/// distinct border style and shape, so status never depends on color alone.
pub fn badge(status: Status) -> Markup {
    html! {
        span class=(format!("status-badge {}", status.class())) {
            span class="visually-hidden" { "Status: " }
            (status.label())
        }
    }
}

/// Legend shown above status lists.
pub fn status_legend() -> Markup {
    html! {
        ul class="narr-legend" aria-label="What each status label means" {
            li { (badge(Status::Implemented)) " code exists on main and a test exercises it" }
            li { (badge(Status::Partial)) " some of it exists; the limits are stated" }
            li { (badge(Status::Experimental)) " written as an experiment, not part of the system" }
            li { (badge(Status::Planned)) " designed, not built yet" }
            li { (badge(Status::Hypothesis)) " a research question, not a feature" }
        }
    }
}
