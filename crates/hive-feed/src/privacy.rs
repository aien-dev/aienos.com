//! One publication boundary for every source-controlled string. Inspect before truncating.
use regex::Regex;
use std::sync::LazyLock;

struct Rules { secrets: Regex, paths: Regex, ipv4: Regex, ipv6: Regex, context: Regex, token: Regex }
static RULES: LazyLock<Rules> = LazyLock::new(|| Rules {
        secrets: Regex::new(r#"(?ix)(-----BEGIN\s+(?:[A-Z]+\s+)*PRIVATE\s+KEY|\b(?:bearer|basic)\s+[a-z0-9+/=_-]+|\b(?:[a-z0-9]+[_-])*(?:password|passwd|secret|api[_\x20-]?key|private[_\x20-]?key|access[_\x20-]?token|auth(?:orization)?|cookie|token|session[_\x20-]?(?:key|token|id))["\x27]?\s*(?::|=|\bis\b)\s*\S+|\b(?:sk-|ghp_|github_pat_|gho_|xox[baprs]-|AKIA)[a-z0-9_-]+|\beyJ[a-z0-9_-]+\.[a-z0-9_-]+\.[a-z0-9_-]+|(?:omp|ssh|agent|proc|history|local|memory|xd|skill|file|artifact|security|mcp)://|(?:https?|wss?)://[^\s<>]+[?\#][^\s<>]+|[a-z][a-z0-9+.-]*://[^\s/@]+:[^\s/@]+@|https?://(?:localhost|127\.|10\.|192\.168\.|\[))"#).unwrap(),
        paths: Regex::new(r#"(?i)(?:~[/\\][^\s<>"\x27]*|/(?:home|Users|root|tmp|var|run|etc|mnt|media|private|opt|proc|sys|dev|workspace|srv|scratch|data)(?:/[^\s<>"\x27]*)?|(?:^|[\s(`"\x27])/[a-z0-9._-]+(?:/[^\s<>"\x27]*)+|[a-z]:\\[^\s<>"\x27]+|\b[a-z0-9._-]+@(?:[a-z0-9._-]+)|\b(?:localhost|[a-z0-9_-]+(?:\.[a-z0-9_-]+)*\.(?:local|internal|ts\.net))\b)"#).unwrap(),
        ipv4: Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
        ipv6: Regex::new(r"(?i)(?:\[)?(?:[0-9a-f]{0,4}:){2,}[0-9a-f:]{0,39}(?:\])?").unwrap(),
        context: Regex::new(r"(?i)<\s*/?\s*[a-z][a-z0-9_-]*(?:\s[^>]*|\s*)>|\[system[- ](?:reminder|notice)\]|\[Wait interrupted|^\s*<irc|^\s*#\s*(?:Shared goal and contract|Target|Change|Acceptance)\b").unwrap(),
        token: Regex::new(r"[A-Za-z0-9_+/=-]{24,}").unwrap(),
});

pub fn public_text(input: &str, limit: usize) -> Option<String> {
    let r = &*RULES;
    // Reject the entire original, including content beyond the display bound.
    if input.is_empty() || r.secrets.is_match(input) || r.context.is_match(input) || input.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')) { return None; }
    for m in r.token.find_iter(input) {
        let s = m.as_str();
        let mut counts = [0usize; 128];
        for b in s.bytes() { counts[b as usize] += 1; }
        let entropy: f64 = counts.iter().filter(|&&n| n > 0).map(|&n| { let p = n as f64 / s.len() as f64; -p * p.log2() }).sum();
        let mixed = s.bytes().any(|b| b.is_ascii_digit()) && s.bytes().any(|b| b.is_ascii_alphabetic());
        if (mixed && entropy > 3.5) || entropy > 4.2 || (s.len() >= 32 && s.bytes().all(|b| b.is_ascii_hexdigit())) { return None; }
    }
    let text = r.paths.replace_all(input, "[private path]");
    let text = r.ipv4.replace_all(&text, |caps: &regex::Captures<'_>| {
        let nums: Vec<u16> = caps[0].split('.').filter_map(|s| s.parse().ok()).collect();
        if nums.len() == 4 && (nums[0] == 10 || nums[0] == 127 || nums[0] == 0 || (nums[0] == 192 && nums[1] == 168) || (nums[0] == 172 && (16..=31).contains(&nums[1])) || (nums[0] == 169 && nums[1] == 254) || (nums[0] == 100 && (64..=127).contains(&nums[1]))) { "[private address]".to_owned() } else { caps[0].to_owned() }
    });
    let text = r.ipv6.replace_all(&text, "[network address]");
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
}
