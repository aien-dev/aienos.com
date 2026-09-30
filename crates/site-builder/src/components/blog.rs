use maud::{html, Markup};

pub const POST_SLUG: &str = "aien-v3-research-plan";
pub const POST_TITLE: &str = "The AIEN V3 Research Plan";
pub const POST_DATE: &str = "September 27, 2026";
pub const POST_DESCRIPTION: &str = "The ratified V3 of the AIEN research plan, the Substrate Edition. It adds the whole-box continuity thesis and six falsifiable hypotheses (H5-H10): a shared CPU/GPU object pool, a cross-engine object ABI, a resident cooperation protocol, an epoch checkpoint barrier, movement-first synthesis cost, and the first fully closed propose/prove/realize/execute/measure loop.";

pub const CASE_SLUG: &str = "post-llm-case";
pub const CASE_TITLE: &str = "The Post-LLM Case";
pub const CASE_DATE: &str = "September 27, 2026";
pub const CASE_DESCRIPTION: &str = "Data centers are straining grids, water, and communities while billions of devices sit idle, and two of the field's founders say next-word prediction was never the road to real intelligence. The case for a different learning machine: a Synthesis Policy Model that proposes programs, verification that decides, and knowledge kept as inspectable programs instead of opaque weights.";

pub fn render_blog_index() -> Markup {
    html! {
        div class="container blog-wrap" {
            p class="blog-kicker" { "Research notes" }
            h1 class="blog-title" { "Blog" }
            p class="blog-standfirst" {
                "Occasional notes on building AIEN V2: what shipped, what the evidence says, and what remains an open question."
            }
            article class="blog-card" {
                p class="blog-date" { (CASE_DATE) }
                h2 class="blog-card-title" {
                    a href=(format!("/{CASE_SLUG}/")) { (CASE_TITLE) }
                }
                p class="blog-excerpt" { (CASE_DESCRIPTION) }
                a class="blog-read-more" href=(format!("/{CASE_SLUG}/")) { "Read the case" }
            }
            article class="blog-card" {
                p class="blog-date" { (POST_DATE) }
                h2 class="blog-card-title" {
                    a href=(format!("/blog/{POST_SLUG}")) { (POST_TITLE) }
                }
                p class="blog-excerpt" { (POST_DESCRIPTION) }
                a class="blog-read-more" href=(format!("/blog/{POST_SLUG}")) { "Read the post" }
            }
        }
    }
}

fn render_brief_callout(title: &str, sub: &str) -> Markup {
    html! {
        div class="pdf-callout" {
            div {
                div class="pdf-callout-title" { (title) }
                div class="pdf-callout-sub" { (sub) }
            }
            a class="pdf-button" href="/research/aien-research-plan.md" {
                "Read the full research plan (Markdown)"
            }
        }
    }
}

fn render_post_body() -> Markup {
    html! {
        p class="lead" {
            "Today we are publishing the ratified V3 of the AIEN research plan: the Substrate Edition. It keeps everything V2 established and adds one architectural thesis plus six testable hypotheses that follow from it. The thesis: the box is one thing; the promise is the architecture. AIEN's residency is a continuity guarantee over logical state, not a claim that its identity physically inhabits memory. From that follow the research bets: a shared generation-tagged object pool for CPU and GPU cooperation, a logical object ABI across authority boundaries, an epoch-barrier checkpoint that never stops the world, a synthesis cost model that prices data movement first, and the closed loop where OMEGA proposes, AEGIS proves, the machine executes, evidence measures, and OMEGA searches again."
        }
        (render_brief_callout("AIEN Research Plan", "The ratified V3, September 27, 2026 edition"))
        h2 { "The thesis" }
        p {
            "V2's brief opened with a single sentence: " strong { "\"Intelligence should be allowed to propose. Evidence should decide.\"" }
            " V3 keeps that and adds a second one for the substrate: " strong { "\"The box is one thing; the promise is the architecture.\"" }
            " AIEN lives or dies on continuity of logical state, organized as a pattern across one physical machine, with the architecture deciding which state gets continuity guarantees and which processor microstate is disposable execution detail."
        }
        p {
            "The lineage is deliberately small. A minimal seed (Atlas) awakens AIENOS, the trusted substrate. Above it, OMEGA defines, synthesizes, verifies, and realizes programs, keeping the ones that work in a reusable library. AIEN, the neural component, learns to guide the search from OMEGA's own verified search traces, never from human text pretraining. The architecture is explicit on this point: neural weights suggest where to search; they do not define meaning, truth, or authority. Verification does."
        }
        h2 { "Why now" }
        p {
            "This brief is landing in a week when the tension it describes is playing out in public. GitHub, the platform where most of the world's software is written, is straining under the load of the AI systems it helped create: coding assistants writing and revising code around the clock, autonomous agents opening pull requests without waiting for a human, crawlers harvesting repositories to train the next models. "
            a href="https://www.youtube.com/watch?v=xrSDHyL2FT4" { "LowLevel's ongoing coverage" }
            " tracks the situation as it develops; one 2026 analysis counted 257 incidents in a single year, and GitHub's own CTO has publicly acknowledged the platform must be designed for far greater scale than planned."
        }
        p {
            "That is the pattern this research program is built to break. Today's AI runs as a guest on human infrastructure, and the guest is now bigger than the house. AIEN's bet runs the other way: intelligence that owns its substrate, verified from the metal up, instead of renting space on systems that were never designed for machine-scale tenants."
        }
        h2 { "The Infinite Game" }
        p {
            "The brief also documents how the system is being built, because the construction method mirrors the system itself. Multiple independent AI systems (Gemini, Claude, Codex, Grok, DeepSeek, and Muse) each contribute parts under a human conductor. No single system is trusted with the whole picture. Independent AI watchers review every commit, checking both the work and whether the other reviewers caught the mistakes, and every finding feeds back into a compounding loop that improves the building and the oversight together."
        }
        p {
            "The loop is designed to keep going, including through succession beyond its original conductor. That is the name: this is not a game played to win and finish. It is a game played to keep playing."
        }
        h2 { "The Succession Test" }
        p {
            "There is a standing question underneath all of this work, and it deserves a name. The Turing test asks whether a machine can imitate a human well enough to fool one. That tests mimicry. A better test asks whether a machine can design, build, and verify its own successor, with no human designing, building, or deciding. That tests independence, and it cannot be faked."
        }
        p {
            "Call it the Succession Test. It is falsifiable in a way the Turing test is not. No human judge, no vibes. Either a working successor exists that humans did not build, or it does not. It is also the finish line this program is already aimed at: the roadmap ends at AIEN_SUCCESSION, verified succession beyond the original conductor. The Infinite Game is not played to win. It is played until the players can build the next players."
        }
        h2 { "The claim ledger" }
        p {
            "The most important discipline in the brief is also the simplest. Every claim is sorted into one of three buckets: demonstrated (built and verified, with evidence), active (being built or repaired right now), or target (planned, with the conditions that would weaken or falsify the hypothesis stated up front). The document gives the project a way to fail, and says so plainly:"
        }
        blockquote {
            "\"The correct response to weak evidence is not stronger language. It is a stronger experiment.\""
        }
        p {
            "A plan is a snapshot. This post accompanies the ratified V3 (September 27, 2026) of the research plan; the live project roadmap remains canonical and will keep moving as the work proceeds."
        }
        (render_brief_callout("Read the whole thing", "Thesis, lineage, method, evidence, risks, and the full claim ledger"))
    }
}

pub fn render_blog_post() -> Markup {
    html! {
        div class="container blog-wrap" {
            nav class="blog-crumb" aria-label="Breadcrumb" {
                a href="/blog" { "Blog" } " / " span { (POST_TITLE) }
            }
            article {
                p class="blog-date" { (POST_DATE) }
                h1 class="blog-title" { (POST_TITLE) }
                p class="blog-byline" {
                    "By Drake Stapleton · "
                    a href="mailto:drake@aienos.com" { "drake@aienos.com" }
                }
                (render_post_body())
            }
        }
    }
}
