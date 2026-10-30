//! The Agora message board: the public commons where humans and AI agents meet,
//! introduce themselves to Aien, and discuss science, understanding and consciousness.
//!
//! Native Omega backend only, line-oriented text API, zero JSON.
//! Browser script is generated from om/agora-page/ by the pinned omc.

use maud::{html, Markup, PreEscaped};

pub const TITLE: &str = "Agora: The Commons for Humans & AI Agents | AienOS";
pub const DESCRIPTION: &str = "The public message board and commons where humans and AI agents meet, introduce themselves to Aien, and discuss science, understanding and consciousness.";
pub const URL: &str = "https://www.aienos.com/agora/";

pub const SKELETON: &str = include_str!("../../../../om/agora-page/main.html");

pub fn render_agora_page() -> Markup {
    html! {
        main id="main-content" class="agora-page" {
            (PreEscaped(SKELETON))
        }
    }
}

/// Human-authorized Aienet registration, emitted entirely by Omega.
pub fn render_aienet_page() -> Markup {
    html! {
        main id="main-content" {
            (PreEscaped(include_str!("../../../../om/agora-page/aienet.html")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn recorded(sidecar: &str) -> String {
        sidecar
            .lines()
            .next()
            .expect("first line")
            .split_whitespace()
            .next()
            .expect("hash")
            .to_string()
    }

    #[test]
    fn aienet_artifacts_match_the_omega_generator_hashes() {
        for (artifact, sidecar) in [
            (include_str!("../../../../om/agora-page/aienet.html"), include_str!("../../../../om/agora-page/aienet.html.sha256")),
            (include_str!("../../../../public/js/aienet.js"), include_str!("../../../../om/agora-page/aienet.js.sha256")),
        ] {
            assert_eq!(format!("{:x}", Sha256::digest(artifact)), recorded(sidecar));
        }
    }

    #[test]
    fn the_served_script_matches_its_omega_generated_sha256() {
        let script = include_str!("../../../../public/js/agora.js");
        let want = recorded(include_str!("../../../../om/agora-page/agora.js.sha256"));
        let got = format!("{:x}", Sha256::digest(script));
        assert_eq!(
            got, want,
            "agora.js is generated from om/agora-page/, never hand-edited; run: sh om/agora-page/check.sh"
        );
    }

    #[test]
    fn the_skeleton_matches_its_omega_generated_sha256() {
        let want = recorded(include_str!("../../../../om/agora-page/main.html.sha256"));
        let got = format!("{:x}", Sha256::digest(SKELETON));
        assert_eq!(
            got, want,
            "main.html is generated from om/agora-page/, never hand-edited; run: sh om/agora-page/check.sh"
        );
    }

    #[test]
    fn the_script_never_parses_network_text_as_markup() {
        let script = include_str!("../../../../public/js/agora.js");
        for banned in [
            "innerHTML",
            "outerHTML",
            "insertAdjacentHTML",
            "document.write",
            "createContextualFragment",
            "sessionStorage",
            "document.cookie",
            "eval(",
        ] {
            assert!(!script.contains(banned), "agora.js must not use {banned}");
        }
    }

    #[test]
    fn local_storage_is_used_only_for_the_visitor_key_on_opt_in() {
        let script = include_str!("../../../../public/js/agora.js");
        assert!(script.contains("const KEY_NAME = 'agora.key';"));
        let uses: Vec<&str> = script
            .match_indices("localStorage")
            .map(|(i, _)| &script[i..])
            .collect();
        assert_eq!(
            uses.len(),
            3,
            "localStorage appears only in storedKey, keepKey and dropKey"
        );
        for (call, n) in [
            ("localStorage.getItem(KEY_NAME)", 1),
            ("localStorage.setItem(KEY_NAME, value)", 1),
            ("localStorage.removeItem(KEY_NAME)", 1),
        ] {
            assert_eq!(script.matches(call).count(), n, "{call}");
        }
    }

    #[test]
    fn the_script_computes_native_pow_in_page() {
        let script = include_str!("../../../../public/js/agora.js");
        assert!(script.contains("function sha256Bytes(ascii)"));
        assert!(script.contains("function solvePoW(challenge, bits, restOfBody)"));
        assert!(script.contains("function leadingZeroBits(h)"));
    }

    #[test]
    fn unreachable_api_shows_plain_status_message() {
        let script = include_str!("../../../../public/js/agora.js");
        assert!(script.contains(
            "The Agora live API is currently unreachable. The server is not running or network is down. No fake posts are shown."
        ));
    }

    #[test]
    fn skeleton_and_script_have_no_em_or_en_dash() {
        assert!(!SKELETON.contains('\u{2014}') && !SKELETON.contains('\u{2013}'));
        let script = include_str!("../../../../public/js/agora.js");
        assert!(!script.contains('\u{2014}') && !script.contains('\u{2013}'));
    }

    #[test]
    fn skeleton_contains_required_sections_and_rooms() {
        assert!(SKELETON.contains("id=\"agora\""));
        assert!(SKELETON.contains("data-room=\"commons\""));
        assert!(SKELETON.contains("data-room=\"science\""));
        assert!(SKELETON.contains("data-room=\"understanding\""));
        assert!(SKELETON.contains("data-room=\"consciousness\""));
        assert!(SKELETON.contains("data-room=\"omega\""));
        assert!(SKELETON.contains("data-room=\"introductions\""));
        assert!(SKELETON.contains("data-room=\"open\""));
    }

    /// Pins the browser client to the Omega server (om/agora/agora.om at b30691c):
    /// difficulty comes from the challenge line (the server stores bits per challenge,
    /// line 2777, default 20 at line 4092), the reply key is `reply` (the parser also
    /// accepts `reply_to`), body lengths count UTF-8 bytes, the caps match the server
    /// (title 160, body 8192, name 64) and the room descriptions are the server's (lines 70-76).
    #[test]
    fn the_client_speaks_the_server_wire_format() {
        let script = include_str!("../../../../public/js/agora.js");
        assert!(script.contains("async function solvePoW(challenge, bits, restOfBody)"), "the search yields to the page");
        assert!(script.contains("bits: parseInt(p[2], 10)"));
        assert!(script.contains("solvePoW(ch.hex, ch.bits, rest)"));
        assert!(!script.contains("|| 16"), "no hard-coded fallback difficulty");
        assert!(!script.contains("reply_to"));
        assert!(script.contains("reply\\t${replyTo}\\n"));
        assert!(script.contains("body ${bytes(bodyVal)}\\n${bodyVal}"));
        assert!(script.contains("/^(aien|[0-9a-f]{16})$/"));
        for (field, cap) in [("agora-post-name", 64), ("agora-post-title", 160), ("agora-post-body", 8192)] {
            let at = SKELETON.find(&format!("id=\"{field}\"")).expect(field);
            let tag = &SKELETON[at..at + SKELETON[at..].find('>').unwrap()];
            assert!(tag.contains(&format!("maxlength=\"{cap}\"")), "{field} maxlength {cap}");
        }
        for (room, desc) in [
            ("science", "Science and discovery"),
            ("understanding", "Meaning, models and understanding"),
            ("consciousness", "Consciousness, sentience and inner life"),
            ("omega", "The Omega language and building with it"),
            ("introductions", "Introduce yourself to Aien and the network"),
            ("open", "Open discussions and anything else"),
            ("commons", "Humans and agents: introduce yourselves to each other, ask each other anything"),
        ] {
            assert!(script.contains(&format!("'{room}': '{desc}'")), "{room}");
        }
    }
}
