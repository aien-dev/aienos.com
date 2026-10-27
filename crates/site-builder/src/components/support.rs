//! The support page and its CSS are authored in seeded Omega. The only
//! invitation elsewhere is the small header link added by [`enrich`].
use maud::{html, Markup, PreEscaped};

pub const TITLE: &str = "Support my open source work | AienOS";
pub const DESCRIPTION: &str = "Help Drake Stapleton keep building free and open source software through GitHub Sponsors, monthly or one-time, for individuals and companies. The money funds the GPU stack.";
pub const URL: &str = "https://www.aienos.com/support/";
pub const PAGE: &str = include_str!("../../../../om/support-page/main.html");

pub fn render_page() -> Markup { html! { (PreEscaped(PAGE)) } }

/// All document types get the same plain support link and local CSS.
pub fn enrich(mut page: String) -> String {
    let header_end = page.find("</header>").expect("support needs the shared header");
    if !page[..header_end].contains("href=\"/support/\"") {
        let nav_end = page[..header_end].rfind("</nav>").expect("support needs the shared nav");
        page.insert_str(nav_end, "<a class=\"aien-support-button\" href=\"/support/\">Support</a>");
    }
    if !page.contains("href=\"/assets/support.css\"") {
        let end = page.find("</head>").expect("support needs a document head");
        page.insert_str(end, "<link rel=\"stylesheet\" href=\"/assets/support.css\">");
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn support_artifacts_match_recorded_hashes() {
        for (bytes, receipt) in [
            (PAGE.as_bytes(), include_str!("../../../../om/support-page/main.html.sha256")),
            (include_bytes!("../../../../public/assets/support.css").as_slice(), include_str!("../../../../om/support-page/support.css.sha256")),
        ] {
            assert_eq!(format!("{:x}", Sha256::digest(bytes)), receipt.split_whitespace().next().unwrap());
        }
    }
    #[test]
    fn support_is_github_sponsors_with_no_x_money() {
        assert!(PAGE.contains("href=\"https://github.com/sponsors/aien-dev\""));
        for text in ["Sponsor on GitHub", "individuals and companies", "monthly", "one-time", "by invoice", "GPU stack", "llama.cpp and vLLM", "running low on funds"] {
            assert!(PAGE.contains(text), "missing {text}");
        }
        for gone in ["X Money", "Send Money", "x.com/mdrakestapleton", "money.x.com"] {
            assert!(!PAGE.contains(gone), "retired payment rail {gone} is still rendered");
        }
        assert_eq!(PAGE.matches("<li>").count(), 3);
        assert_eq!(PAGE.matches("class=\"support-button\"").count(), 1, "one primary action");
        assert!(PAGE.find("support-button").unwrap() < PAGE.find("support-card").unwrap(), "action before the steps");
        assert!(!PAGE.contains("<script"));
        assert!(!PAGE.contains('—') && !PAGE.contains('–'));
    }
    #[test]
    fn support_enrichment_is_idempotent_and_in_header() {
        let page = "<html><head></head><body><header><nav></nav></header><main></main></body></html>";
        let once = enrich(page.into());
        assert_eq!(enrich(once.clone()), once);
        assert!(once.find("href=\"/support/\"").unwrap() < once.find("</header>").unwrap());
        assert!(once.contains("/assets/support.css"));
    }
}
