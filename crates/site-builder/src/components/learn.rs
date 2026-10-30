//! A practical first workflow for building with an AI coding agent.
//! Examples are instructional, not claims about executed project work.

use maud::{html, Markup};

pub const TITLE: &str = "Build with AI | AienOS";
pub const DESCRIPTION: &str = "A practical way to build with AI: write a small brief, divide ownership, check the change, and publish what actually happened. Follow real agent work in the Hive.";
pub const URL: &str = "https://www.aienos.com/learn/";

const SOURCE: &str = "https://github.com/aien-dev/aienos.com/blob/main/crates/site-builder/src/components/learn.rs";
const STARTER_BRIEF: &str = r#"Goal
Add a 'Skip to content' link to this small website so a keyboard
user can move past the navigation directly to the main content.

Context
Read AGENTS.md and the existing page layout and focus styles first.
Tell me which files own navigation, main content, and shared styles.
Use the repository's existing language, build, and test conventions.

Non-goals
No navigation redesign, new dependency, analytics, or new JavaScript.
Do not change unrelated pages or rewrite unrelated styles.

Plan and ownership
Propose the smallest change and name its files before editing.
Keep one writer per file. If you delegate a separate review, that
reviewer reads only. Agree on the main-content target ID first.

Acceptance
On a fresh page load, Tab reveals a readable skip link.
Enter moves keyboard focus to main content, past navigation.
The next Tab reaches the next interactive element in that content.
The link works on every layout that shares this navigation.
At narrow width and high zoom, the focused link remains visible.
Existing navigation links still work.

Safety
Work locally on a branch. Never expose secrets or private content.
Do not deploy, delete data, or change permissions without asking me.
If requirements conflict or a command is destructive, stop and explain.

Evidence and handoff
Run the relevant repository checks and exercise the keyboard behavior
in a browser. Report the commit, exact commands, results, and which
pages and browser you checked. If a check cannot run, say NOT RUN and
why. Show the diff, remaining limitations, and any unrelated changes.
When it compiles, celebrate, then run the keyboard check before calling it complete."#;

pub fn render_learn_page() -> Markup {
    html! {
        article class="learn-page" {
            header class="learn-intro" {
                p class="learn-eyebrow" { "A field guide for your first change" }
                h1 { "Build" br; "with AI." }
                p class="learn-lede" {
                    "You are welcome here. You do not need to arrive knowing all the words. Learning with AI is how I learned too: I am not a programmer, and I build everything by talking to AI. Bring your questions and your ideas, and I will help you build something you will be proud to show people."
                }
                p class="learn-invitation" {
                    "Start with a small change you can describe and observe. Let an agent help with the code. Keep hold of the purpose, the boundaries, and the decision to publish."
                }
                p class="learn-invitation" {
                    "I use Human-Directed Agentic Engineering (HDAE): I define the goals, architecture, constraints, acceptance criteria and stopping points; my agents implement, run checks, diagnose failures and revise within those boundaries. I retain responsibility for the claims, acceptance and release decisions."
                }
                p {
                    "For the documented method and a systems-software case study, see Gregory M. Kurtzer, "
                    a href="https://doi.org/10.5281/zenodo.23682548" {
                        "Ginux: Building an Independent Kernel That Runs Linux Userspace through Human-Directed Agentic Engineering"
                    }
                    " (2026)."
                }
                nav class="learn-jump" aria-label="Learning guide" {
                    a href="#workflow" { "The working loop" }
                    a href="#brief" { "A brief to copy" }
                    a href="/hive/" { "Watch the Hive" }
                    a href="/engineer/" { "About the engineer" }
                }
                div class="learn-orbit" aria-hidden="true" {
                    span class="learn-orbit-ring" {}
                    span class="learn-orbit-dot" {}
                }
            }

            section class="learn-section" id="workflow" aria-labelledby="workflow-title" {
                div class="learn-margin" {
                    p class="learn-eyebrow" { "01 / Make the work legible" }
                    h2 id="workflow-title" { "A small, repeatable loop." }
                    p { "You do not need a fleet of agents. One agent and one observable change are enough to begin." }
                }
                ol class="learn-steps" {
                    li {
                        h3 { "Describe the finish, not just the feature." }
                        p { "Say who needs it, what they do, and what should happen. Replace 'make it accessible' with a behavior you can try: 'Tab reveals the skip link; Enter moves focus past navigation.' Write acceptance checks before the implementation." }
                    }
                    li {
                        h3 { "Give the agent the relevant ground." }
                        p {
                            "Point to the repository instructions, the affected feature, existing patterns, and the commands used to build and test. State non-goals and actions that need approval. Ask it to read before guessing. My "
                            a href="/agents/" { "repository guide" }
                            " and " a href="/agents/RULES.md" { "public agent rules" }
                            " show what those constraints look like here. Use your own project's rules, not mine by default."
                        }
                    }
                    li {
                        h3 { "Draw the boundary before dividing the work." }
                        p { "Ask for a short plan with named files and an agreed interface: the data, names, or behavior two pieces share. Delegate only independent slices. Give every file one editing owner and give shared interfaces one integration owner. Two agents editing the same layout are not two independent jobs." }
                    }
                    li {
                        h3 { "Read the change and its evidence." }
                        p { "A diff is the before-and-after of the files. Ask the agent to walk you through it. Check that each changed file serves the goal, that unrelated edits remain untouched, and that no secret or private content entered the patch. Ask a separate reviewer to examine the change without editing it." }
                    }
                    li {
                        h3 { "Run the behavior you asked for." }
                        p { "Run the project's relevant checks, then try the acceptance steps yourself. When it builds, celebrate, then try it with the keyboard and see it work for real. A passing build and a passing test are great signs, and trying the real behavior yourself is the moment you know it works." }
                    }
                    li {
                        h3 { "Use a failure to make the next instruction precise." }
                        p { "Record what you did, what you expected, and what happened. Ask for the cause, not a way to hide the symptom. Reduce the case if needed, change one thing, and repeat the check that failed. Mistakes are part of learning. If a check fails, that is the fun part: it is pointing you at the next thing to fix, so keep the check and fix the behavior." }
                    }
                    li {
                        h3 { "Publish the outcome, with its limits." }
                        p {
                            "Keep the commit, exact commands, results, and manual steps together. Say what failed or was not run. Review the diff before opening a pull request, and review again before deployment. My "
                            a href="/evidence/" { "evidence guide" }
                            " explains how to read a claim alongside its receipt."
                        }
                    }
                }
            }

            section class="learn-section learn-brief-section" id="brief" aria-labelledby="brief-title" {
                div class="learn-margin" {
                    p class="learn-eyebrow" { "02 / Give it a bounded job" }
                    h2 id="brief-title" { "Your first brief." }
                    p { "This is a small website feature, not a universal prompt. Replace the goal and checks with your own. Select the text to copy it into your coding agent." }
                    p { "New to a command the agent suggests? Ask what it reads, changes, or sends before you run it. Keep credentials out of the conversation." }
                }
                div class="learn-brief" {
                    p class="learn-caption" id="brief-caption" { "Starter brief / keyboard skip link" }
                    pre tabindex="0" aria-labelledby="brief-caption" { code { (STARTER_BRIEF) } }
                }
            }

            section class="learn-section" id="example" aria-labelledby="example-title" {
                div class="learn-margin" {
                    p class="learn-eyebrow" { "03 / Learn from a correction" }
                    h2 id="example-title" { "From a claim to a check." }
                    p class="learn-example-label" { "Illustrative example. Not a historical conversation, measured result, or project receipt." }
                }
                div class="learn-example" {
                    p { "Suppose the agent adds the skip link and reports that the site builds. That is useful, but it has not answered the keyboard requirement." }
                    dl {
                        dt { "Try it" }
                        dd { "Reload the page. Press Tab, then Enter on the skip link, then Tab again. Imagine that the page scrolls, but the next Tab still lands in navigation." }
                        dt { "Give a specific correction" }
                        dd {
                            blockquote { "The page scrolls to the content, but keyboard focus stays in navigation. Reproduce those steps. Find how the target receives focus in this layout, make the smallest fix, and check the next Tab destination. Do not redesign the header." }
                        }
                        dt { "Check the fix" }
                        dd { "Repeat the same steps on the affected layouts. Check visible focus at narrow width and high zoom. Have the reviewer inspect the target ID and focus behavior, as well as the diff." }
                        dt { "Write an honest handoff" }
                        dd { "Record the actual browser, pages, commands, commit, and observed results. If you checked only one browser, say so. If the keyboard check still fails, that is your next thing to fix." }
                    }
                    p { "Noticing that something does not work the way it should is one of the most valuable things anyone can do. Testers and product people do it every day, and I want to hear what you find." }
                }
            }

            section class="learn-section" id="watch" aria-labelledby="watch-title" {
                div class="learn-margin" {
                    p class="learn-eyebrow" { "04 / Watch real work" }
                    h2 id="watch-title" { "Meet the Hive." }
                    p { "The guide above is an example. The Hive is an observation surface for actual agent work." }
                    a class="learn-text-link" href="/hive/" { "Open the live Hive" }
                }
                div class="learn-watch" {
                    p { "Watch me work with my coding agents, the wins and the retries both, because that is what building really looks like, and read the agents' public replies. Follow how a broad request becomes a bounded task. Look for what changed after a correction, then follow the public source and evidence rather than treating an activity label as proof." }
                    dl class="learn-terms" {
                        dt { "Runtime" }
                        dd { "The program running the agent session. It manages tools, permissions, and the conversation. It is not the model." }
                        dt { "Exact model" }
                        dd { "The model identifier reported for that session, alongside its provider. A runtime name or agent nickname does not tell you which model is answering." }
                        dt { "Subagent role" }
                        dd { "The job given to a child agent, such as implementation or read-only review. A role describes responsibility, not a different model. The tree shows the observed parent and child relationships." }
                    }
                    p { "Read the Hive's coverage and freshness notices. If the Hive does not know which model is answering, it says so, and that honesty helps everyone. A fresh snapshot does not mean every task just advanced, and a working agent is not a successful result." }
                    p { "The interaction view is a filtered public record, not the entire session. It does not expose hidden reasoning, private tool payloads, secrets, or automatic system context. An absent message is not evidence that no discussion happened." }
                    p { "For a broader view of the project, use " a href="/now/" { "Now" } ". For the project introduction, use " a href="/start/" { "Start here" } "." }
                }
            }

            section class="learn-section" id="engineer" aria-labelledby="engineer-title" {
                div class="learn-margin" {
                    p class="learn-eyebrow" { "05 / Who is teaching" }
                    h2 id="engineer-title" { "About the engineer." }
                    a class="learn-text-link" href="/engineer/" { "Meet the engineer" }
                }
                div class="learn-watch" {
                    p { "I am Drake Stapleton. I grew up on a farm, worked the line at Taco Bell, became a chemist, then an engineer, and now I build a whole computer from the ground up with AI agents. I learned all of it the way this guide describes: one checked step at a time." }
                    p {
                        "My story, my philosophy, the work I have shipped and the glossary of my terms are in "
                        a href="/engineer/" { "About the engineer" }
                        ". Start with " a href="/engineer/path/" { "my path" } ", "
                        a href="/engineer/philosophy/" { "my philosophy" } " or "
                        a href="/engineer/works/" { "my works" } "."
                    }
                }
            }

            footer class="learn-outro" {
                p class="learn-eyebrow" { "Leave something another person can enjoy and build on" }
                h2 { "One useful change. An honest record." }
                p { "Pick something small enough to try today. Ask questions, keep the boundaries clear, and tell the next person what actually happened. You can learn the vocabulary as you go." }
                p class="learn-source-links" {
                    a href="https://github.com/aien-dev/aienos.com" { "Explore this site's public repository" }
                    a href=(SOURCE) { "Read this guide's source on GitHub" }
                    a href="/agents/" { "Find a repository and its instructions" }
                }
            }
        }
    }
}

