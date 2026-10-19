use maud::{html, Markup, PreEscaped, DOCTYPE};

pub fn render_base_layout(content: Markup) -> Markup {
    render_page_layout(
        "AIENOS | Persistent machine intelligence you own",
        "An experimental, open source machine for turning search into verified understanding, running on five real machines. Every claim carries a public receipt.",
        "https://www.aienos.com/",
        "website",
        html! {
            link rel="stylesheet" href="/assets/allen.css";
            script defer src="/allen/brain.js" {}
            script defer src="/allen/allen.js" {}
            // Zero-dependency terminal micro-script
            script defer src="/js/terminal.js" {}
            script defer src="/js/turing-game.js" {}
            (PreEscaped(r#"<script>
                document.addEventListener('DOMContentLoaded', () => {
                    console.log('[AIEN OS] Pure Rust Maud engine online. Zero hydration overhead.');
                });
            </script>"#))
        },
        content,
    )
}

/// Parameterized page layout for non-homepage pages (blog, posts).
/// Title, description, and canonical URL are set per page. `extra_head`
/// carries page-specific scripts (e.g. the comments loader on post pages).
/// Social cards, hreflang alternates, feed links and JSON-LD are added to
/// every written page by `seo::enrich`, so they are not repeated here.
pub fn render_page_layout(
    title: &str,
    description: &str,
    og_url: &str,
    og_type: &str,
    extra_head: Markup,
    content: Markup,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                meta name="description" content=(description);
                @if !og_url.is_empty() { link rel="canonical" href=(og_url); }
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                meta property="og:type" content=(og_type);

                // Stylesheet
                link rel="stylesheet" href="/assets/style.css";
                // Shared six-section header (also linked by hand-written pages)
                link rel="stylesheet" href="/assets/site-nav.css";
                // The Turing look, shared by every page (loaded last)
                link rel="stylesheet" href="/assets/turing-look.css";
                // The machines and the speeds (homepage and /machine/)
                link rel="stylesheet" href="/assets/machines.css";
                script defer src="/js/turing-look.js" {}

                (extra_head)
            }
            body {
                div id="root" style="min-height: 100vh; display: flex; flex-direction: column;" {
                    a class="skip-link" href="#main-content" { "Skip to content" }
                    (crate::components::navbar::render_navbar())
                    main id="main-content" style="flex: 1;" {
                        (content)
                    }
                    (crate::components::footer::render_footer())
                }
            }
        }
    }
}
