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

/// Hello form gate. Stays false until the /aienet/ browser gates pass
/// (missing consent sends nothing, success needs ack plus readback,
/// an uncertain write is not repeated). Flip to true to turn it back on.
pub const AIENET_HELLO_FORM: bool = false;

pub const AIENET_PAUSED_NOTE: &str = "The hello form is paused until browser testing is done, so profiles can't be registered from this page yet.";

/// Human-authorized Aienet registration, emitted entirely by Omega.
/// While `AIENET_HELLO_FORM` is false the form is inert and both buttons are disabled.
pub fn render_aienet_page() -> Markup {
    let page = include_str!("../../../../om/agora-page/aienet.html");
    let body = if AIENET_HELLO_FORM {
        page.to_string()
    } else {
        pause_aienet_hello_form(page)
    };
    html! {
        main id="main-content" {
            (PreEscaped(body))
        }
    }
}

/// One replacement. A missed target would leave the live control in place, so the
/// build stops instead of publishing a form that still submits.
fn replace_one(haystack: &str, from: &str, to: &str) -> String {
    assert_eq!(haystack.matches(from).count(), 1, "aienet pause missed its target in aienet.html: {from}");
    haystack.replacen(from, to, 1)
}

fn pause_aienet_hello_form(page: &str) -> String {
    let form = replace_one(
        page,
        "<form id=\"aienet-form\" class=\"hub-card\">",
        &format!("<p id=\"aienet-paused\" role=\"status\">{AIENET_PAUSED_NOTE}</p><form id=\"aienet-form\" class=\"hub-card\" inert>"),
    );
    let submit = replace_one(
        &form,
        "<button id=\"aienet-submit\" type=\"submit\">",
        "<button id=\"aienet-submit\" type=\"submit\" disabled>",
    );
    let paused = replace_one(
        &submit,
        "<button id=\"aienet-recheck\" type=\"button\">",
        "<button id=\"aienet-recheck\" type=\"button\" disabled>",
    );
    // The Omega page does not inline the hello routes today. If it ever does,
    // drop them here so a paused document cannot name the live door.
    let paused = paused.replace("/agora/api/", "");
    assert!(!paused.contains("/agora/api/"), "paused /aienet/ page still contains /agora/api/");
    paused
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
    fn aienet_form_cannot_submit_while_flag_is_off() {
        if AIENET_HELLO_FORM { return; }
        let source = include_str!("../../../../om/agora-page/aienet.html");
        let p = render_aienet_page().into_string();
        let replacements = [
            ("<form id=\"aienet-form\" class=\"hub-card\">", "<form id=\"aienet-form\" class=\"hub-card\" inert>"),
            ("<button id=\"aienet-submit\" type=\"submit\">", "<button id=\"aienet-submit\" type=\"submit\" disabled>"),
            ("<button id=\"aienet-recheck\" type=\"button\">", "<button id=\"aienet-recheck\" type=\"button\" disabled>"),
        ];
        for (from, to) in replacements {
            assert_eq!(source.matches(from).count(), 1, "pause target missing from aienet.html: {from}");
            assert!(!p.contains(from), "replacement did not happen: {from}");
            assert_eq!(p.matches(to).count(), 1, "replacement missing: {to}");
        }
        assert_eq!(p.matches("id=\"aienet-paused\"").count(), 1);
        assert!(p.contains(AIENET_PAUSED_NOTE));
        assert!(p.contains("role=\"status\""));
        assert!(!p.contains("aienet.js"));
        assert!(!p.contains("/agora/api/"), "paused page still contains an Agora API path");
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
