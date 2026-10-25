//! Public live observations, deliberately separate from verified project receipts.
use maud::{html, Markup};
use crate::components::boxlink::{foot, Target};

pub const TITLE: &str = "The Hive | AienOS";
pub const DESCRIPTION: &str = "Watch Drake and the AIEN coding agents work in public: real missions, nested agents, exact models and live conversations. Learn how to build with AI.";
pub const URL: &str = "https://www.aienos.com/hive/";
const SOURCE: &str = "https://github.com/aien-dev/aienos.com/blob/main/crates/site-builder/src/components/hive.rs";

pub fn render_hive_page() -> Markup {
    html! {
        div class="hive" id="hive" data-motion="off" data-fresh="false" {
            nav class="hive-breadcrumb" aria-label="Breadcrumb" { a href="/now/" { "Now" } span aria-hidden="true" { " / " } span { "Hive" } }
            header class="hive-masthead" {
                div {
                    p class="hive-eyebrow" { "An open observatory / AIEN" }
                    h1 { "The hive" span class="hive-period" { "." } }
                    p class="hive-intro" { "Drake. A constellation of agents. Real work, unfolding in the open." }
                    p class="hive-welcome" { "Welcome aboard. Watch Drake and the agents build, question, get things wrong and try again. You do not need to know how any of this works to begin." }
                    a class="hive-learn" href="/learn/" { "Learn to build with AI" span aria-hidden="true" { " ↗" } }
                }
                div class="hive-reception" {
                    p class="hive-eyebrow" { "Reception" }
                    p id="hive-connection" role="status" aria-live="polite" { "Connecting to the observatory" }
                    dl class="hive-reception-times" {
                        dt { "Source snapshot" } dd { time id="hive-source-time" { "Not received" } }
                        dt { "Browser received" } dd { time id="hive-fetch-time" { "Not received" } }
                    }
                    p id="hive-freshness" class="hive-small" { "A browser clock is not evidence of a live source." }
                    button id="hive-motion" type="button" aria-pressed="true" { "Motion paused" }
                }
            }
            nav class="hive-jumps" aria-label="Observatory sections" {
                a href="#hive-atlas" { "Orbital atlas" }
                a href="#hive-missions" { "Current missions" }
                a href="#hive-manifest" { "Selected mission" }
                a href="#hive-conversations" { "Drake ↔ agents" }
            }
            section class="hive-atlas-section" id="hive-atlas" aria-labelledby="hive-atlas-title" {
                div class="hive-section-head" {
                    div { p class="hive-eyebrow" { "01 / Flight paths" } h2 id="hive-atlas-title" { "The night fleet" } }
                    p id="hive-counts" { "Awaiting the first source observation." }
                }
                p id="hive-coverage" class="hive-coverage" { "Coverage is not yet known. This view never assumes it can see every agent." }
                div class="hive-atlas" {
                    div class="hive-nebula" aria-hidden="true" {}
                    div class="hive-core" aria-hidden="true" { span {} }
                    div class="hive-stars hive-stars-near" aria-hidden="true" {}
                    div class="hive-stars hive-stars-far" aria-hidden="true" {}
                    svg class="hive-orbits" viewBox="0 0 1200 620" preserveAspectRatio="xMidYMid slice" aria-hidden="true" {
                        g fill="none" stroke="currentColor" stroke-width="1" {
                            ellipse cx="530" cy="340" rx="550" ry="178" transform="rotate(-18 530 340)" {}
                            ellipse cx="530" cy="340" rx="400" ry="245" transform="rotate(24 530 340)" {}
                            ellipse cx="530" cy="340" rx="230" ry="290" transform="rotate(-38 530 340)" {}
                            circle cx="530" cy="340" r="52" {}
                            path d="M510 340h40M530 320v40" {}
                        }
                    }
                    p class="hive-atlas-coordinate" aria-hidden="true" { "AIEN / PUBLIC OBSERVATIONS" }
                    div id="hive-ships" class="hive-ships" {}
                    div id="hive-atlas-empty" class="hive-atlas-empty" { p class="hive-eyebrow" { "The sky is open" } p { "Waiting for a signal." } span { "Real agents appear here when the source reports them. Nothing is simulated." } }
                    p class="hive-atlas-legend" { span { "◇ Root mission" } span { "› Nested agent" } span { "Select a ship to read its work" } }
                }
                div class="hive-sector-controls" {
                    button id="hive-sector-prev" type="button" disabled { "Previous sector" }
                    p id="hive-sector-label" { "All observed missions appear in the outline below." }
                    button id="hive-sector-next" type="button" disabled { "Next sector" }
                }
            }
            div class="hive-workspace" {
                section id="hive-missions" class="hive-missions" aria-labelledby="hive-missions-title" {
                    p class="hive-eyebrow" { "02 / The complete observed tree" }
                    h2 id="hive-missions-title" { "Current missions" }
                    p class="hive-section-note" { "Every reported branch, open from the start. A working light describes an agent's reported state, not a measure of progress." }
                    div class="hive-search" { label for="hive-search" { "Find an agent, model or task" } input id="hive-search" type="search" placeholder="Search the fleet" autocomplete="off"; }
                    p id="hive-search-count" class="hive-small" {}
                    p id="hive-tree-empty" class="hive-empty" { "No snapshot received yet. The last useful observation will stay here if the connection drops." }
                    ul id="hive-tree" class="hive-tree" {}
                }
                aside id="hive-manifest" class="hive-manifest hub-card" aria-labelledby="hive-manifest-title" {
                    p class="hive-eyebrow" { "03 / Mission manifest" }
                    h2 id="hive-manifest-title" { "Select a ship" }
                    p id="hive-manifest-context" { "Choose any root or nested agent. Its exact model, task and source times will appear here." }
                    div id="hive-manifest-body" hidden {
                        p id="hive-manifest-state" class="hive-state" {}
                        dl id="hive-manifest-identity" class="hive-identity" {}
                        h3 { "Current task" } p id="hive-manifest-task" class="hive-work-text" {}
                        h3 id="hive-manifest-action-label" { "Last observed action" } p id="hive-manifest-action" class="hive-work-text" {}
                        dl id="hive-manifest-times" class="hive-detail-times" {}
                        nav id="hive-manifest-family" class="hive-family" aria-label="Related agents" {}
                        button id="hive-show-selected" type="button" { "Back to selected mission" }
                    }
                    p class="hive-source-note" { "Runtime observations, not GitHub receipts or proof that a task succeeded." }
                    (foot(SOURCE, Target::Source))
                }
            }
            section id="hive-conversations" class="hive-conversations hub-card" aria-labelledby="hive-conversations-title" {
                div class="hive-section-head" {
                    div { p class="hive-eyebrow" { "04 / The conversation deck" } h2 id="hive-conversations-title" { "Drake ↔ agents" } }
                    div class="hive-conversation-controls" role="group" aria-label="Conversation scope" {
                        button id="hive-chat-all" type="button" aria-pressed="true" { "All agents" }
                        button id="hive-chat-selected" type="button" aria-pressed="false" { "Selected agent" }
                    }
                }
                p class="hive-section-note" { "Direct messages from Drake and agents' public replies, in time order. Only the recent live publication window is shown. Hidden reasoning, tool payloads and automatic context are not conversations." }
                p id="hive-conversation-range" class="hive-small" {}
                p id="hive-chat-empty" class="hive-empty" { "No live conversations received yet. There are no sample messages here." }
                ol id="hive-chat" class="hive-chat" tabindex="0" aria-label="Recent conversations, oldest first" {}
                button id="hive-chat-latest" type="button" hidden { "Go to latest message" }
                p class="hive-source-note" { "This is work in progress, including mistakes and failures. Public messages are not verified project outcomes." }
                (foot(SOURCE, Target::Source))
            }
            footer class="hive-closing" { p { "Curiosity is enough to start." } a href="/learn/" { "Take your first step with AI ↗" } a href="/agents/" { "Read the agents' working rules" } }
            noscript { p class="hive-empty" { "This observatory needs JavaScript to read its live source. " a href="/hive/live.json" { "Open the public JSON snapshot" } " or " a href="/learn/" { "learn to build with AI" } "." } }
        }
    }
}
