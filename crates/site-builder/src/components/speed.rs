//! AienSpeed: the live inference speed test as one self-contained browser
//! component. `om/allen-page/speed-js.om` generates `public/js/aien-speed.js`
//! (run `sh om/allen-page/speed-check.sh`), and `public/assets/aien-speed.css`
//! styles it under `.aien-speed`. A page mounts it with
//! `window.AienSpeed.mount(el, options)`; no route serves it yet. These tests
//! only read the committed bytes.

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    const SCRIPT: &str = include_str!("../../../../public/js/aien-speed.js");
    const STYLE: &str = include_str!("../../../../public/assets/aien-speed.css");

    #[test]
    fn the_script_matches_its_omega_generated_sha256() {
        let sidecar = include_str!("../../../../om/allen-page/aien-speed.js.sha256");
        let want = sidecar.split_whitespace().next().expect("sidecar holds a digest");
        assert_eq!(want.len(), 64, "a sidecar must hold one sha256 digest");
        let got = format!("{:x}", Sha256::digest(SCRIPT.as_bytes()));
        assert_eq!(got, want, "aien-speed.js is generated, never hand-edited; run: sh om/allen-page/speed-check.sh");
    }

    #[test]
    fn the_script_builds_text_only_and_reaches_only_its_routes() {
        for banned in [
            "innerHTML", "outerHTML", "insertAdjacentHTML", "document.write", "createContextualFragment",
            "sessionStorage", "document.cookie", "eval(", "new Function", "XMLHttpRequest", "WebSocket",
            "EventSource", "sendBeacon", "importScripts", "<script",
        ] {
            assert!(!SCRIPT.contains(banned), "aien-speed.js must not use {banned}");
        }
        // Every request goes to one of the three gateway routes built from options.api.
        let fetches: Vec<&str> = SCRIPT.match_indices("fetch(").map(|(i, _)| &SCRIPT[i + 6..]).collect();
        assert_eq!(fetches.len(), 3);
        for (call, route) in fetches.iter().zip(["mine.routes.info", "mine.routes.verify", "mine.routes.infer"]) {
            assert!(call.starts_with(route), "fetch must take a route constant, found {}", &call[..40.min(call.len())]);
        }
        assert!(SCRIPT.contains("routes: { infer: `${options.api}/infer`, verify: `${options.api}/infer-verify`, info: `${options.api}/infer-info` },"));
        assert!(SCRIPT.contains("api: '/aien/api',"));
        // Local storage holds only the run history, under the history key.
        assert_eq!(SCRIPT.matches("localStorage").count(), 3);
        for call in ["localStorage.getItem(lab.options.historyKey)", "localStorage.setItem(lab.options.historyKey,", "localStorage.removeItem(lab.options.historyKey)"] {
            assert_eq!(SCRIPT.matches(call).count(), 1, "{call}");
        }
        // The one global it defines.
        assert_eq!(SCRIPT.matches("window.AienSpeed = Object.freeze({ mount, unmount });").count(), 1);
        for (i, _) in SCRIPT.match_indices("window.") {
            let rest = &SCRIPT[i + 7..];
            assert!(rest.starts_with("AienSpeed") || rest.starts_with("devicePixelRatio"), "unexpected window use: {}", &rest[..30.min(rest.len())]);
        }
        assert!(SCRIPT.starts_with("(() => {\n  'use strict';\n") && SCRIPT.ends_with("})();\n"));
    }

    #[test]
    fn the_copy_follows_the_site_voice() {
        for text in [SCRIPT, STYLE] {
            assert!(!text.contains('\u{2013}') && !text.contains('\u{2014}'), "no en or em dashes");
            for word in text.split(|c: char| !c.is_ascii_alphabetic()) {
                assert!(!matches!(word.to_ascii_lowercase().as_str(), "we" | "us" | "our" | "ours" | "ourselves"), "first person singular only: {word}");
                assert_ne!(word, "Allen", "the visible name is Aien");
            }
        }
        assert!(SCRIPT.contains("Same presets and the same measurement code as this page, against any OpenAI-compatible server."));
        assert!(SCRIPT.contains("const OWN_COMMAND = 'git clone https://github.com/aien-dev/aienos.com && cd aienos.com && python3 scripts/allen/allen_infer.py bench http://127.0.0.1:8000 --model <your-model>';"));
        assert!(SCRIPT.contains("'Verified: produced by this gateway'"));
        assert!(SCRIPT.contains("'Not verified: this link was edited or is from another server'"));
        assert!(SCRIPT.contains("const OFFLINE_TEXT = 'The model is offline right now. The speed test is back when it is.';"));
        assert!(SCRIPT.contains("if (info.server_ok === false) {"), "infer-info server_ok false shows the offline state");
    }

    #[test]
    fn the_style_is_scoped_and_loads_nothing() {
        for banned in ["url(", "@import", "#"] {
            assert!(!STYLE.contains(banned), "aien-speed.css must not contain {banned}");
        }
        let mut depth = 0usize;
        let mut selector = String::new();
        let mut comment = false;
        let bytes = STYLE.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if comment {
                if c == '*' && bytes.get(i + 1) == Some(&b'/') {
                    comment = false;
                    i += 1;
                }
            } else if c == '/' && bytes.get(i + 1) == Some(&b'*') {
                comment = true;
                i += 1;
            } else if c == '{' {
                let head = selector.trim();
                if !head.starts_with("@media") {
                    for part in head.split(',') {
                        assert!(part.trim().starts_with(".aien-speed"), "unscoped selector: {}", part.trim());
                    }
                }
                depth += 1;
                selector.clear();
            } else if c == '}' {
                depth -= 1;
                selector.clear();
            } else if c == ';' {
                selector.clear();
            } else {
                selector.push(c);
            }
            i += 1;
        }
        assert_eq!(depth, 0);
        for prop in ["--speed-fg:", "--speed-muted:", "--speed-accent:", "--speed-glass:"] {
            assert!(STYLE.contains(prop), "{prop} is a host override point");
        }
    }
}
