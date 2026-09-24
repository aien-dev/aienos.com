use maud::{html, Markup, PreEscaped, DOCTYPE};

pub fn render_base_layout(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "AIEN OS | The GPU-Native Neural Operating Environment" }
                meta name="description" content="AIEN OS eliminates the software orchestration tax with coherent unified memory, in-process cognitive kernels, sub-5MB daemon footprints, and hardware-enforced sovereign security.";
                link rel="canonical" href="https://aienos.com/";
                link rel="icon" type="image/svg+xml" href="/favicon.svg";

                // Open Graph / Twitter Meta
                meta property="og:title" content="AIEN OS | The GPU-Native Neural Operating Environment";
                meta property="og:description" content="AIEN OS eliminates the software orchestration tax with coherent unified memory, in-process cognitive kernels, and hardware TPM security.";
                meta property="og:url" content="https://aienos.com/";
                meta property="og:type" content="website";
                meta property="og:image" content="https://aienos.com/og-card.png";
                meta name="twitter:card" content="summary_large_image";
                meta name="twitter:title" content="AIEN OS | The GPU-Native Neural Operating Environment";
                meta name="twitter:description" content="Replacing the CPU von Neumann bottleneck with coherent unified memory on Grace Blackwell silicon.";
                meta name="twitter:image" content="https://aienos.com/og-card.png";

                // Fonts
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous";
                link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Space+Grotesk:wght@500;600;700;800&display=swap";

                // Stylesheet
                link rel="stylesheet" href="/assets/style.css";

                // Zero-dependency terminal micro-script
                script defer src="/js/terminal.js" {}
                (PreEscaped(r#"<script>
                    document.addEventListener('DOMContentLoaded', () => {
                        console.log('[AIEN OS] Pure Rust Maud engine online. Zero hydration overhead.');
                    });
                </script>"#))
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
