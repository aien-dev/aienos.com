use maud::{html, Markup, PreEscaped, DOCTYPE};

pub fn render_base_layout(content: Markup) -> Markup {
    render_page_layout(
        "AIEN OS | The GPU-Native Neural Operating Environment",
        "AIEN OS eliminates the software orchestration tax with coherent unified memory, in-process cognitive kernels, sub-5MB daemon footprints, and hardware-enforced sovereign security.",
        "https://aienos.com/",
        "website",
        html! {
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
                link rel="canonical" href=(og_url);
                link rel="icon" type="image/svg+xml" href="/favicon.svg";

                // Open Graph / Twitter Meta
                meta property="og:title" content=(title);
                meta property="og:description" content=(description);
                meta property="og:url" content=(og_url);
                meta property="og:type" content=(og_type);
                meta property="og:image" content="https://aienos.com/og.png";
                meta name="twitter:card" content="summary_large_image";
                meta name="twitter:title" content=(title);
                meta name="twitter:description" content=(description);
                meta name="twitter:image" content="https://aienos.com/og.png";

                // Fonts
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous";
                link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Space+Grotesk:wght@500;600;700;800&display=swap";

                // Stylesheet
                link rel="stylesheet" href="/assets/style.css";

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
