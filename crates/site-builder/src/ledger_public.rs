//! The public ledger is a line-preserving projection, never a copy of the
//! private coordination ledger. Row numbers remain stable for #L links.
use std::borrow::Cow;
use regex::Regex;
use std::sync::LazyLock;

struct Rules {
    secret: Regex,
    email: Regex,
    uuid: Regex,
    path: Regex,
    host: Regex,
    ipv4: Regex,
    ipv6: Regex,
    port: Regex,
    pid: Regex,
}

static RULES: LazyLock<Rules> = LazyLock::new(|| Rules {
    secret: Regex::new(r"(?ix)(?:\b(?:password|passwd|secret|api[_-]?key|private[_-]?key|access[_-]?token|auth(?:orization)?|cookie|session[_-]?(?:key|token|id))\s*(?:=|:|\bis\b)\s*\S+|--(?:[a-z0-9]+[_-])*(?:password|passwd|secret|api[_-]?key|private[_-]?key|access[_-]?token|auth(?:orization)?|cookie|session[_-]?(?:key|token|id))\s+\S+|\b(?:sk-|ghp_|gho_|github_pat_|xox[baprs]-|AKIA)[a-z0-9_-]+|-----BEGIN\s+(?:[A-Z]+\s+)*PRIVATE\s+KEY|\bBearer\s+[a-z0-9._-]{8,}|[a-z][a-z0-9+.-]*://[^\s/@]+:[^\s/@]+@)").unwrap(),
    email: Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b").unwrap(),
    uuid: Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap(),
    // Only paths with a separate leading delimiter. Repository-relative
    // evidence paths and public https:// links are intentional references.
    path: Regex::new(r#"(?i)(^|[\s(<`"'\[=:,])(?:~[/\\][^\s<>"']*|/(?:[a-z0-9._-]+/)+[a-z0-9._-]*)"#).unwrap(),
    host: Regex::new(r"(?i)\b(?:localhost|[a-z0-9_-]+(?:\.[a-z0-9_-]+)*\.(?:local|internal|ts\.net))\b").unwrap(),
    ipv4: Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
    ipv6: Regex::new(r"(?i)\[[0-9a-f:]{2,}\]").unwrap(),
    port: Regex::new(r":18[0-9]{3}\b").unwrap(),
    pid: Regex::new(r"(?i)\bpid\s+[0-9]+\b").unwrap(),
});

const HEADER: &str = "id\topened_utc\towner\tstatus\titem\tdone_when\tevidence\tupdated_utc";

fn public_field(input: &str) -> Result<Cow<'_, str>, &'static str> {
    let r = &*RULES;
    if r.secret.is_match(input) { return Err("credential-shaped value in ledger"); }
    if r.email.is_match(input) { return Err("email-shaped value in ledger"); }
    if input.chars().any(|c| c.is_control()) {
        return Err("control character in ledger field");
    }
    if !r.path.is_match(input) && !r.uuid.is_match(input) && !r.host.is_match(input)
        && !r.ipv4.is_match(input) && !r.ipv6.is_match(input) && !r.port.is_match(input)
        && !r.pid.is_match(input) {
        return Ok(Cow::Borrowed(input));
    }
    let value = r.path.replace_all(input, |caps: &regex::Captures<'_>| format!("{}[private path]", &caps[1]));
    let value = r.uuid.replace_all(&value, "[private id]");
    let value = r.host.replace_all(&value, "[private host]");
    let value = r.ipv4.replace_all(&value, "[network address]");
    let value = r.ipv6.replace_all(&value, "[network address]");
    let value = r.port.replace_all(&value, ":[private port]");
    let value = r.pid.replace_all(&value, "pid [private]");
    Ok(Cow::Owned(value.into_owned()))
}

pub fn project(input: &str) -> Result<String, String> {
    if !input.ends_with('\n') || input.contains('\r') { return Err("ledger must be newline-terminated UTF-8 TSV".into()); }
    let mut lines = input.lines();
    if lines.next() != Some(HEADER) { return Err("unexpected ledger header".into()); }
    let mut output = String::with_capacity(input.len());
    output.push_str(HEADER);
    output.push('\n');
    for (row, line) in lines.enumerate() {
        let mut fields = line.split('\t');
        for index in 0..8 {
            let field = fields.next().ok_or("ledger row has fewer than eight columns")?;
            if index > 0 { output.push('\t'); }
            output.push_str(&public_field(field).map_err(|reason| format!("line {}: {reason}", row + 2))?);
        }
        if fields.next().is_some() { return Err(format!("line {}: ledger row has more than eight columns", row + 2)); }
        output.push('\n');
    }
    Ok(output)
}

pub fn check_public(input: &str) -> Result<(), String> {
    if project(input)? == input { Ok(()) } else { Err("ledger is not its public projection".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER_LINE: &str = "id\topened_utc\towner\tstatus\titem\tdone_when\tevidence\tupdated_utc\n";
    #[test]
    fn projects_paths_addresses_and_keeps_line_links() {
        let row = "L001\t2026-10-19T16:43Z\tagent\tdone\tSee /home/person/private\tCheck ~/notes at 127.0.0.1:18091\trepo@abcdef1 evidence/run.log https://github.com/aien-dev/omega-mv pid 12345 550e8400-e29b-41d4-a716-446655440000\t2026-10-19T17:00Z\n";
        let output = project(&format!("{HEADER_LINE}{row}")).unwrap();
        assert!(output.contains("See [private path]"));
        assert!(output.contains("Check [private path] at [network address]:[private port]"));
        assert!(output.contains("repo@abcdef1 evidence/run.log https://github.com/aien-dev/omega-mv"));
        assert!(output.contains("pid [private] [private id]"));
        assert!(output.lines().nth(1).unwrap().starts_with("L001\t"));
        assert!(check_public(&output).is_ok());
        assert!(check_public(&format!("{HEADER_LINE}{row}")).is_err());
    }
    #[test]
    fn rejects_entire_ledger_on_identifiers_credentials_or_malformed_rows() {
        for value in ["owner@example.org", "password=hunter22", "--api-key shortvalue", "ghp_abcdef123"] {
            let input = format!("{HEADER_LINE}L001\tdate\tagent\topen\t{value}\t\t\tdate\n");
            assert!(project(&input).is_err(), "{value}");
        }
        assert!(project(&format!("{HEADER_LINE}L001\tbad\n")).is_err());
        assert!(project(&format!("{HEADER_LINE}L001\tdate\tagent\topen\tgood\t\t\tdate\n")).is_ok());
    }
}
