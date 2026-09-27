use maud::{html, Markup};

pub const POST_SLUG: &str = "aien-v2-research-plan";
pub const POST_TITLE: &str = "The AIEN V2 Research Plan";
pub const POST_DATE: &str = "September 27, 2026";
pub const POST_DESCRIPTION: &str = "AIEN V2 is a research program asking whether machine intelligence can be built from a small, inspectable software lineage instead of an inherited opaque stack. This post introduces the public research brief: the thesis, the construction method, and an honest ledger of what is proven, what is underway, and what remains ahead.";

pub fn render_blog_index() -> Markup {
    html! {
        div class="container blog-wrap" {
            p class="blog-kicker" { "Research notes" }
            h1 class="blog-title" { "Blog" }
            p class="blog-standfirst" {
                "Occasional notes on building AIEN V2: what shipped, what the evidence says, and what remains an open question."
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

fn render_pdf_callout(title: &str, sub: &str) -> Markup {
    html! {
        div class="pdf-callout" {
            div {
                div class="pdf-callout-title" { (title) }
                div class="pdf-callout-sub" { (sub) }
            }
            a class="pdf-button" href="/research/aien-research-plan.pdf" {
                "Read the full 25-page research brief (PDF)"
            }
        }
    }
}

fn render_post_body() -> Markup {
    html! {
        p class="lead" {
            "Today we are publishing the first public research brief for AIEN V2: a 25-page document that lays out the program in full. What has been demonstrated. What is actively being built or repaired. What remains planned. It is written to be read as a research program, not a manifesto, and it holds itself to one standard: separate what is proven from what is hoped for, and let evidence decide."
        }
        (render_pdf_callout("AIEN Research Plan", "The full 25-page public brief, September 27, 2026 edition"))
        h2 { "The thesis" }
        p {
            "The brief opens with a single sentence: " strong { "\"Intelligence should be allowed to propose. Evidence should decide.\"" }
            " It then turns that into a falsifiable engineering question: can intelligence be built from a small, inspectable lineage that learns through verified programs and machine evidence, rather than permanently inheriting an opaque software tower?"
        }
        p {
            "The lineage is deliberately small. A minimal seed (Atlas) awakens AIENOS, the trusted substrate. Above it, OMEGA defines, synthesizes, verifies, and realizes programs, keeping the ones that work in a reusable library. AIEN, the neural component, learns to guide the search from OMEGA's own verified search traces, never from human text pretraining. The architecture is explicit on this point: neural weights suggest where to search; they do not define meaning, truth, or authority. Verification does."
        }
        h2 { "The Infinite Game" }
        p {
            "The brief also documents how the system is being built, because the construction method mirrors the system itself. Multiple independent AI systems (Gemini, Claude, Codex, Grok, DeepSeek, and Muse) each contribute parts under a human conductor. No single system is trusted with the whole picture. Independent AI watchers review every commit, checking both the work and whether the other reviewers caught the mistakes, and every finding feeds back into a compounding loop that improves the building and the oversight together."
        }
        p {
            "The loop is designed to keep going, including through succession beyond its original conductor. That is the name: this is not a game played to win and finish. It is a game played to keep playing."
        }
        h2 { "The claim ledger" }
        p {
            "The most important discipline in the brief is also the simplest. Every claim is sorted into one of three buckets: demonstrated (built and verified, with evidence), active (being built or repaired right now), or target (planned, with the conditions that would weaken or falsify the hypothesis stated up front). The document gives the project a way to fail, and says so plainly:"
        }
        blockquote {
            "\"The correct response to weak evidence is not stronger language. It is a stronger experiment.\""
        }
        p {
            "A brief is a snapshot. This post accompanies the September 27, 2026 edition of the research brief; the live project roadmap remains canonical and will keep moving as the work proceeds."
        }
        (render_pdf_callout("Read the whole thing", "Thesis, lineage, method, evidence, risks, and the full claim ledger"))
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
