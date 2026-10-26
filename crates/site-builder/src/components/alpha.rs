//! Seeded Omega owns the page structure and browser client.
use maud::{html, Markup, PreEscaped};
pub const TITLE: &str = "Sovereign Intelligence: alpha testing is open | AienOS";
pub const DESCRIPTION: &str = "Sovereign Intelligence, built on AienOS. Try my Omega installer and 35B engine for Linux arm64 and NVIDIA GB10. The local Aien experience is coming in the next update.";
pub const URL: &str = "https://www.aienos.com/alpha/";
pub const PAGE: &str = include_str!("../../../../om/alpha-page/main.html");
pub fn render_alpha_page() -> Markup { html! { (PreEscaped(PAGE)) } }
pub fn render_alpha_entry() -> Markup {
    html! {
        a href="/alpha/" style="display:block;padding:24px;text-align:center;background:var(--bg-surface);border-bottom:1px solid var(--accent-green);color:var(--text-primary);" {
            strong style="color:var(--accent-green);font-family:var(--font-mono);" { "SOVEREIGN INTELLIGENCE · ALPHA TESTING IS OPEN" }
            " · Built on AienOS. Keep the GPU. Lose the stack. "
            span style="text-decoration:underline;text-underline-offset:4px;" { "Get the installer →" }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn alpha_generated_artifacts_match_hashes() {
        for (bytes, receipt) in [
            (PAGE.as_bytes(), include_str!("../../../../om/alpha-page/main.html.sha256")),
            (include_bytes!("../../../../public/js/alpha.js").as_slice(), include_str!("../../../../om/alpha-page/alpha.js.sha256")),
            (include_bytes!("../../../../public/alpha/wizard.om").as_slice(), include_str!("../../../../om/alpha-page/wizard.om.sha256")),
        ] {
            assert_eq!(format!("{:x}", Sha256::digest(bytes)), receipt.split_whitespace().next().expect("digest"));
        }
    }
    #[test]
    fn alpha_preserves_install_and_evidence_boundaries() {
        for text in ["curl -fsSL https://aienos.com/install.sh | sh", "NOT_RUN", "GB10", "kernel driver stay", "alpha-uninstall", "download=\"wizard.om\"", "id=\"alpha-motion\""] {
            assert!(PAGE.contains(text), "missing {text}");
        }
        let css = include_str!("../../../../public/assets/alpha.css");
        assert!(css.contains("prefers-reduced-motion:no-preference"));
        assert!(css.contains("[data-motion=\"on\"]"));
        assert!(!css.contains("http"));
    }
}
