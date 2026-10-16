use maud::{html, Markup, DOCTYPE};

pub const SITE_URL: &str = "https://www.aienos.com/";
pub const TITLE: &str = "AIENOS | Persistent machine intelligence on a machine you own";
pub const DESCRIPTION: &str = "AIEN is a sovereign computing stack: an operating system, a language with its own GPU engine, a host runtime and a verification pipeline, built in the open on one NVIDIA DGX Spark. Pre-alpha.";

pub fn render_base_layout(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (TITLE) }
                meta name="description" content=(DESCRIPTION);
                link rel="canonical" href=(SITE_URL);
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                meta property="og:title" content=(TITLE);
                meta property="og:description" content=(DESCRIPTION);
                meta property="og:url" content=(SITE_URL);
                meta property="og:type" content="website";
                meta property="og:image" content="https://www.aienos.com/og.png";
                meta name="twitter:card" content="summary_large_image";
                meta name="twitter:title" content=(TITLE);
                meta name="twitter:description" content=(DESCRIPTION);
                meta name="twitter:image" content="https://www.aienos.com/og.png";
                meta name="color-scheme" content="dark";
                meta name="theme-color" content="#0c1017";
                link rel="stylesheet" href="/assets/style.css";
            }
            body {
                div id="root" {
                    a class="skip-link" href="#main-content" { "Skip to content" }
                    (crate::components::navbar::render_navbar())
                    main id="main-content" {
                        (content)
                    }
                    (crate::components::footer::render_footer())
                }
            }
        }
    }
}
