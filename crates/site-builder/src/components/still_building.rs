//! The #StillBuilding campaign page is authored in seeded Omega.
use maud::{html, Markup, PreEscaped};

pub const TITLE: &str = "If you built it with AI, you built it | AienOS";
pub const DESCRIPTION: &str = "#StillBuilding: people who build with AI get judged for it, and it has been measured. Show the work, say how you made it, and keep leveling up. Sources linked.";
pub const URL: &str = "https://www.aienos.com/still-building/";
pub const PAGE: &str = include_str!("../../../../om/still-building-page/main.html");

pub fn render_page() -> Markup { html! { (PreEscaped(PAGE)) } }

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn still_building_artifacts_match_recorded_hashes() {
        for (bytes, receipt) in [
            (PAGE.as_bytes(), include_str!("../../../../om/still-building-page/main.html.sha256")),
            (include_bytes!("../../../../public/assets/still-building.css").as_slice(), include_str!("../../../../om/still-building-page/still-building.css.sha256")),
        ] {
            assert_eq!(format!("{:x}", Sha256::digest(bytes)), receipt.split_whitespace().next().unwrap());
        }
    }
    #[test]
    fn still_building_cites_sources_and_discloses() {
        for href in [
            "https://doi.org/10.1073/pnas.2426766122",
            "https://hbr.org/2025/08/research-the-hidden-penalty-of-using-ai-at-work",
            "https://www.microsoft.com/en-us/research/publication/after-organizational-ai-acceptance-ai-bias-fades-but-a-junior-penalty-persists-in-code-review/",
            "https://www.npr.org/2025/09/05/nx-s1-5529404/anthropic-settlement-authors-copyright-ai",
            "/learn/",
        ] {
            assert!(PAGE.contains(&format!("href=\"{href}\"")), "missing {href}");
        }
        for text in ["#StillBuilding", "The concerns are real", "my AI agents drafted this page", "adds no tracking of its own", "href=\"/privacy/\"", "I&#39;ll ask in public before I share anything"] {
            assert!(PAGE.contains(text), "missing {text}");
        }
        assert!(!PAGE.contains("<script") && !PAGE.contains("<img"));
        assert!(!PAGE.contains('\u{2014}') && !PAGE.contains('\u{2013}'));
    }
}
