//! One publication boundary for every source-controlled string. Inspect before truncating.
use regex::Regex;
use std::sync::LazyLock;

struct Rules { secrets: Regex, cli_flags: Regex, paths: Regex, hosts: Regex, ipv4: Regex, ipv6: Regex, context: Regex, token: Regex }
static RULES: LazyLock<Rules> = LazyLock::new(|| Rules {
        secrets: Regex::new(r#"(?ix)(-----BEGIN\s+(?:[A-Z]+\s+)*PRIVATE\s+KEY|\b(?:bearer|basic)\s+[a-z0-9+/=_-]+|\b(?:[a-z0-9]+[_-])*(?:password|passwd|secret|api[_\x20-]?key|private[_\x20-]?key|access[_\x20-]?token|auth(?:orization)?|cookie|token|session[_\x20-]?(?:key|token|id))["\x27]?\s*(?::|=|\bis\b)\s*\S+|\b(?:sk-|ghp_|github_pat_|gho_|xox[baprs]-|AKIA)[a-z0-9_-]+|\beyJ[a-z0-9_-]+\.[a-z0-9_-]+\.[a-z0-9_-]+|(?:omp|ssh|agent|proc|history|local|memory|xd|skill|file|artifact|security|mcp)://|(?:https?|wss?)://[^\s<>]+[?\#][^\s<>]+|[a-z][a-z0-9+.-]*://[^\s/@]+:[^\s/@]+@|https?://(?:localhost|127\.|10\.|192\.168\.|\[))"#).unwrap(),
        // Command-line options often separate the credential from its label
        // with whitespace rather than an assignment operator.
        cli_flags: Regex::new(r"(?ix)--(?:[a-z0-9]+[_-])*(?:password|passwd|secret|api[_-]?key|private[_-]?key|access[_-]?token|auth(?:orization)?|cookie|token|session[_-]?(?:key|token|id))\s+\S+").unwrap(),
        // Absolute and home-relative paths only. A prefix group keeps the
        // neighbouring character, so relative paths such as tests/data/x are
        // published verbatim instead of being mangled mid-word.
        paths: Regex::new(r#"(?i)(^|[\s(<`"'\[=:,])(?:~[/\\][^\s<>"']*|/(?:[a-z0-9._-]+/)+[a-z0-9._-]*|[a-z]:\\[^\s<>"']+)"#).unwrap(),
        hosts: Regex::new(r#"(?i)\b[a-z0-9._-]+@[a-z0-9.-]+\b|\b(?:localhost|[a-z0-9_-]+(?:\.[a-z0-9_-]+)*\.(?:local|internal|ts\.net))\b"#).unwrap(),
        ipv4: Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
        // Candidate runs of colon-separated hex. The shape test rejects
        // clock times and C++ scope operators before redacting.
        ipv6: Regex::new(r"(?i)\[?[0-9a-f]{0,4}(?::[0-9a-f]{0,4}){2,7}\]?").unwrap(),
        context: Regex::new(r"(?i)<\s*/?\s*[a-z][a-z0-9_-]*(?:\s[^>]*|\s*)>|\[system[- ](?:reminder|notice)\]|\[Wait interrupted|^\s*<irc|^\s*#\s*(?:Shared goal and contract|Target|Change|Acceptance)\b").unwrap(),
        token: Regex::new(r"[A-Za-z0-9_+/=-]{24,}").unwrap(),
});

pub fn public_text(input: &str, limit: usize) -> Option<String> {
    let r = &*RULES;
    // Reject the entire original, including content beyond the display bound.
    if input.is_empty() || r.secrets.is_match(input) || r.cli_flags.is_match(input) || r.context.is_match(input) || input.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')) { return None; }
    for m in r.token.find_iter(input) {
        let s = m.as_str();
        let mut counts = [0usize; 128];
        for b in s.bytes() { counts[b as usize] += 1; }
        let entropy: f64 = counts.iter().filter(|&&n| n > 0).map(|&n| { let p = n as f64 / s.len() as f64; -p * p.log2() }).sum();
        let mixed = s.bytes().any(|b| b.is_ascii_digit()) && s.bytes().any(|b| b.is_ascii_alphabetic());
        if (mixed && entropy > 3.5) || entropy > 4.2 || (s.len() >= 32 && s.bytes().all(|b| b.is_ascii_hexdigit())) { return None; }
    }
    let text = r.paths.replace_all(input, |caps: &regex::Captures<'_>| format!("{}[private path]", &caps[1]));
    let text = r.hosts.replace_all(&text, "[private host]");
    // Every IPv4 literal is masked, public or not: a home or office address is
    // itself private detail, and nothing published here needs an address.
    let text = r.ipv4.replace_all(&text, "[network address]");
    let text = r.ipv6.replace_all(&text, |caps: &regex::Captures<'_>| {
        let candidate = caps[0].trim_matches(['[', ']']);
        let groups: Vec<&str> = candidate.split(':').collect();
        let hexdigits: usize = groups.iter().map(|g| g.len()).sum();
        // hh:mm:ss and C++ scope operators are not addresses.
        let clock = !candidate.contains("::") && groups.len() == 3 && groups.iter().all(|g| g.len() == 2);
        let shaped = if candidate.contains("::") { hexdigits > 0 } else { groups.len() >= 4 && hexdigits >= 8 };
        if shaped && !clock { "[network address]".to_owned() } else { caps[0].to_owned() }
    });
    let text = text.replace(['\u{2013}', '\u{2014}'], "-");
    let text = text.trim();
    if text.is_empty() { None } else { Some(text.chars().take(limit).collect()) }
}

pub fn label(input: &str, fallback: &str) -> String { public_text(input, 180).unwrap_or_else(|| fallback.to_owned()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_full_secret_suffix() {
        for secret in ["password=hunter22", "Bearer abcdef", "sk-abc123", "ghp_canary123", "-----BEGIN PRIVATE KEY-----", "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcd", "omp://session/control", "https://example.com/?token=secret", "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW"] {
            assert!(public_text(&format!("{} {secret}", "ordinary text ".repeat(500)), 4000).is_none(), "{secret}");
        }
    }
    #[test] fn rejects_injected_context() {
        for text in ["hello <system-reminder>secret</system-reminder>", "<irc from=\"agent\">data</irc>", "<project-context>data</project-context>", "<system-notice>data</system-notice>"] { assert!(public_text(text, 4000).is_none()); }
    }
    #[test] fn redacts_private_locations() {
        let value = public_text("Read /home/person/private and ~/notes at 100.100.1.2 or 192.168.1.4 or [fd00::1]; https://github.com/aien-dev/aienos.com", 4000).unwrap();
        for needle in ["/home/person", "~/notes", "100.100", "192.168", "fd00"] { assert!(!value.contains(needle)); }
        assert!(value.contains("https://github.com/aien-dev/aienos.com"));
    }
    #[test] fn keeps_times_scopes_and_relative_paths() {
        let value = public_text("at 12:30:45 and 20:04:51Z we ran tests/data/unit.sh then H::rvalue and crates/hive-feed/src/main.rs", 4000).unwrap();
        for needle in ["12:30:45", "20:04:51Z", "tests/data/unit.sh", "H::rvalue", "crates/hive-feed/src/main.rs"] { assert!(value.contains(needle), "{needle} in {value}"); }
        assert!(!value.contains("[network address]"));
    }
    #[test] fn masks_public_addresses_too() {
        let value = public_text("reach me on 203.0.113.7 or [2001:db8::20] but not /var/log/app.log", 4000).unwrap();
        for needle in ["203.0.113.7", "2001:db8", "/var/log"] { assert!(!value.contains(needle), "{needle} in {value}"); }
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;
    #[test] fn recognizes_quoted_assignments_and_control_urls() {
        for value in [
            r#"{"password": "hunter2"}"#, r#"'authorization': 'hunter2'"#,
            r#"api_key = "hunter2""#, "password is hunter2", "DB_PASSWORD=hunter2",
            "postgres://name:hunter2@database.example/db", "http://localhost:18080/control",
            "qWaErTyUiOpAsDfGhJkLzXcVbNmQ",
        ] { assert!(public_text(value,4000).is_none(),"{value}"); }
        assert!(public_text("Do not publish private keys. The model used 400 tokens.",4000).is_some());
    }
    #[test] fn rejects_whitespace_separated_credential_flags() {
        for value in [
            "Run example --password hunter22",
            "Run example --api-key abcdef1234567890",
            "Run example --password 'hunter22'",
            "Run example --access-token \"shortvalue\"",
            "Run example --db_password hunter22",
        ] {
            assert!(public_text(value, 4000).is_none(), "{value}");
        }
        assert!(public_text("Run example --output public/report.txt", 4000).is_some());
    }
}
