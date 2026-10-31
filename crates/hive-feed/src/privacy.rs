//! One publication boundary for every source-controlled string. Inspect before truncating.
use regex::Regex;
use std::sync::{LazyLock, OnceLock};

struct Rules { secrets: Regex, cli_flags: Regex, paths: Regex, hosts: Regex, ipv4: Regex, ipv6: Regex, context: Regex, token: Regex, email: Regex, vault: Regex, secret_names: Regex }
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
        // Outreach contacts appear in agent replies; no address is ever published.
        email: Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)+\b").unwrap(),
        // A vault command names the secret it reads or writes; withhold the name and any value.
        vault: Regex::new(r"(?i)\batlas-vault\s+[a-z-]+(?:\s+[^\s`'\x22]+)?").unwrap(),
        // Secret-shaped identifiers (DEEPSEEK_API_KEY, zenodo-token, SSH_KEY_B64) with any assigned value.
        secret_names: Regex::new(r"(?i)\b(?:[a-z0-9]+[_-])+(?:api[_-]?key|keys?|tokens?|secrets?|password|passwd|pat|credentials?|creds)(?:[_-][a-z0-9]+)*\b(?:\s*[=:]\s*\S+)?").unwrap(),
});

/// Private repository names and their public replacement (`private-NN` or `[private repo]`).
struct PrivateRepos { pattern: Regex, replacement: std::collections::HashMap<String, String> }
static PRIVATE_REPOS: OnceLock<Option<PrivateRepos>> = OnceLock::new();

/// Install the private repository list once at startup. Each entry is a name and
/// an optional anonymised id; names without an id publish as `[private repo]`.
pub fn set_private_repos(entries: Vec<(String, Option<String>)>) -> Result<(), &'static str> {
    let nonempty = !entries.is_empty();
    let compiled = compile_repos(entries);
    if nonempty && compiled.is_none() { return Err("invalid private repository list"); }
    PRIVATE_REPOS.set(compiled).map_err(|_| "private repository list already initialized")
}
fn compile_repos(entries: Vec<(String, Option<String>)>) -> Option<PrivateRepos> {
    if entries.iter().any(|(name, id)| name.trim().is_empty() || id.as_ref().is_some_and(|id| {
        !id.strip_prefix("private-").is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    })) { return None; }
    let mut names: Vec<_> = entries.iter().map(|(n, _)| n.to_ascii_lowercase()).filter(|n| !n.is_empty()).collect();
    if names.is_empty() { return None; }
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    let alternation = names.iter().map(|n| regex::escape(n)).collect::<Vec<_>>().join("|");
    let pattern = Regex::new(&format!("(?i)(?:aien-dev/)?(?:{alternation})")).ok()?;
    let replacement = entries.into_iter().map(|(n, id)| (n.to_ascii_lowercase(), id.unwrap_or_else(|| "[private repo]".to_owned()))).collect();
    Some(PrivateRepos { pattern, replacement })
}
fn redact_repos(text: &str, repos: &PrivateRepos) -> String {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for m in repos.pattern.find_iter(text) {
        let before = text[..m.start()].chars().next_back();
        let mut after = text[m.end()..].chars();
        let next = after.next();
        // Whole names only: omega-learn must not match inside omega-learning or a.omega-learn.
        let joined_after = next.is_some_and(word) || (next == Some('.') && after.next().is_some_and(|c| c.is_ascii_alphanumeric()));
        if before.is_some_and(|c| word(c) || c == '.') || joined_after { continue; }
        let name = m.as_str().to_ascii_lowercase();
        let name = name.strip_prefix("aien-dev/").unwrap_or(&name);
        let Some(public) = repos.replacement.get(name) else { continue; };
        out.push_str(&text[at..m.start()]); out.push_str(public); at = m.end();
    }
    out.push_str(&text[at..]);
    out
}

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
    let text = r.email.replace_all(input, "[email withheld]");
    let text = r.vault.replace_all(&text, "atlas-vault [secret withheld]");
    let text = r.secret_names.replace_all(&text, "[secret name withheld]");
    let text = match PRIVATE_REPOS.get() { Some(Some(repos)) => redact_repos(&text, repos), _ => text.into_owned() };
    let text = r.paths.replace_all(&text, |caps: &regex::Captures<'_>| format!("{}[private path]", &caps[1]));
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

#[cfg(test)]
mod agent_reply_tests {
    use super::*;
    #[test] fn invalid_public_repo_alias_is_refused() {
        for alias in ["my-private-repository", "private-", "private-03 extra", "private-03\nsecret"] {
            assert!(compile_repos(vec![("fixture-repo".into(), Some(alias.into()))]).is_none(), "invalid alias accepted");
        }
    }
    #[test] fn redacts_email_addresses() {
        let value = public_text("I sent the note to first.last+tag@example.ac.uk and copied drake@aienos.com.", 4000).unwrap();
        assert!(!value.contains('@') && value.contains("[email withheld]"), "{value}");
    }
    #[test] fn redacts_vault_and_secret_names() {
        let value = public_text("I read it with atlas-vault get ZENODO_TOKEN, then exported SANDBOX_SSH_KEY and named deepseek-api-key in the notes.", 4000).unwrap();
        for needle in ["ZENODO", "SANDBOX", "deepseek"] { assert!(!value.contains(needle), "{needle} in {value}"); }
        assert!(public_text("The model used 400 tokens and a key frame.", 4000).unwrap().contains("400 tokens and a key frame"));
    }
    #[test] fn private_repo_names_use_the_local_mapping() {
        let repos = compile_repos(vec![("aien-math".into(), Some("private-03".into())), ("omega-learn".into(), None)]).unwrap();
        let value = redact_repos("Merged aien-dev/aien-math#12 and omega-learn; kept aien-math-oracle-work, omega-learning and aienos.com.", &repos);
        assert_eq!(value, "Merged private-03#12 and [private repo]; kept aien-math-oracle-work, omega-learning and aienos.com.");
    }
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
