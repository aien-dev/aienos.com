//! Forced-command receiver: one validated public JSON document per line.
//! No paths or commands are accepted from the SSH client.
use serde_json::Value;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

const MAX: u64 = 2 * 1024 * 1024;
// Collectors truncate to 180 and 4000 characters, which is at most this many
// bytes of UTF-8. One emoji-heavy label must not freeze the whole feed.
const LABEL_MAX: usize = 720;
const BODY_MAX: usize = 16000;

fn object<'a>(v: &'a Value, keys: &[&str]) -> Result<&'a serde_json::Map<String, Value>, ()> {
    let o = v.as_object().ok_or(())?;
    if o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)) { return Err(()); }
    Ok(o)
}
fn text(v: &Value, max: usize) -> Result<&str, ()> {
    let s = v.as_str().ok_or(())?;
    if s.len() > max || s.contains('\0') { return Err(()); }
    Ok(s)
}
fn ident(v: &Value) -> Result<&str, ()> {
    let s = text(v, 128)?;
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') { return Err(()); }
    Ok(s)
}
fn stamp(v: &Value, nullable: bool) -> Result<(), ()> {
    if nullable && v.is_null() { return Ok(()); }
    let s = text(v, 40)?;
    if s.len() < 20 || !s.ends_with('Z') || !s.contains('T') { return Err(()); }
    chrono::DateTime::parse_from_rfc3339(s).map_err(|_| ())?;
    Ok(())
}
fn validate(v: &Value) -> Result<(), ()> {
    object(v, &["schema_version", "generated_at", "sequence", "poll_ms", "stale_after_ms", "coverage", "agents", "interactions"])?;
    if v["schema_version"] != 1 || v["poll_ms"] != 1000 || v["stale_after_ms"] != 15000 { return Err(()); }
    stamp(&v["generated_at"], false)?;
    text(&v["sequence"], 128)?;
    let c = &v["coverage"];
    object(c, &["label", "roots", "agents", "complete", "notice"])?;
    text(&c["label"], 512)?;
    text(&c["notice"], 4096)?;
    c["complete"].as_bool().ok_or(())?;
    let agents = v["agents"].as_array().ok_or(())?;
    if agents.len() > 10000 || c["agents"].as_u64() != Some(agents.len() as u64) { return Err(()); }
    let mut ids = HashSet::new();
    for a in agents {
        object(a, &["id", "parent_id", "name", "runtime", "provider", "model", "role", "task", "status", "activity", "activity_state", "started_at", "updated_at", "observed_at"])?;
        if !ids.insert(ident(&a["id"])?) { return Err(()); }
        if !a["parent_id"].is_null() { ident(&a["parent_id"])?; }
        for k in ["name", "runtime", "provider", "model", "role"] { text(&a[k], LABEL_MAX)?; }
        for k in ["task", "activity"] { text(&a[k], BODY_MAX)?; }
        if !["working", "waiting", "idle", "completed", "failed", "unknown"].contains(&text(&a["status"], 20)?) { return Err(()); }
        if !["running", "finished", "unknown"].contains(&text(&a["activity_state"], 20)?) { return Err(()); }
        stamp(&a["started_at"], true)?;
        stamp(&a["updated_at"], true)?;
        stamp(&a["observed_at"], false)?;
    }
    let roots = agents.iter().filter(|a| a["parent_id"].is_null()).count();
    if c["roots"].as_u64() != Some(roots as u64) { return Err(()); }
    let parents: std::collections::HashMap<&str, Option<&str>> = agents.iter().map(|a| (a["id"].as_str().unwrap(), a["parent_id"].as_str())).collect();
    for a in agents {
        let mut seen = HashSet::new();
        let mut id = a["id"].as_str();
        while let Some(key) = id {
            if !seen.insert(key) { return Err(()); }
            id = *parents.get(key).ok_or(())?;
        }
    }
    let messages = v["interactions"].as_array().ok_or(())?;
    if messages.len() > 100 { return Err(()); }
    let mut message_ids = HashSet::new();
    for m in messages {
        object(m, &["id", "agent_id", "role", "text", "at", "runtime", "provider", "model"])?;
        if !message_ids.insert(ident(&m["id"])?) { return Err(()); }
        ident(&m["agent_id"])?;
        if !["drake", "assistant"].contains(&text(&m["role"], 20)?) { return Err(()); }
        text(&m["text"], 16000)?;
        for key in ["runtime", "provider", "model"] { text(&m[key], LABEL_MAX)?; }
        stamp(&m["at"], false)?;
    }
    Ok(())
}

fn canonical(line: &[u8]) -> Result<(Value, Vec<u8>), ()> {
    let data: Value = serde_json::from_slice(line).map_err(|_| ())?;
    validate(&data)?;
    // Publish exactly the checked value, never unchecked duplicate-key bytes.
    let mut bytes = serde_json::to_vec(&data).map_err(|_| ())?;
    bytes.push(b'\n');
    Ok((data, bytes))
}

fn stamp_of(v: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(v["generated_at"].as_str()?).ok().map(|t| t.timestamp_millis())
}

// Unfinished Rust migration: replace process orchestration with Omega's native
// fork/dupfd/execprog/waitpid host words (introduced by omega-mv PR #678).
// No renderer path or arguments ever come from the SSH client or feed.
fn render_board(dir: &Path, renderer: &Path) -> io::Result<()> {
    let tmp = dir.join(format!(".board-tmp-{}", std::process::id()));
    let result = (|| {
        let input = fs::File::open(dir.join("live.json"))?;
        let output = OpenOptions::new().write(true).create_new(true).mode(0o644).open(&tmp)?;
        let status = Command::new(renderer)
            .stdin(Stdio::from(input)).stdout(Stdio::from(output)).status()?;
        if !status.success() || fs::metadata(&tmp)?.len() == 0 {
            return Err(io::Error::other(format!("renderer failed: {status} or empty output")));
        }
        fs::rename(&tmp, dir.join("board.html"))
    })();
    if result.is_err() { let _ = fs::remove_file(&tmp); }
    result
}

fn receive(dir: &Path) -> Result<(), ()> {
    let metadata = fs::symlink_metadata(dir).map_err(|_| ())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() { return Err(()); }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut line = Vec::new();
    let tmp = dir.join(format!(".incoming-{}", std::process::id()));
    loop {
        line.clear();
        let n = (&mut input).take(MAX + 1).read_until(b'\n', &mut line).map_err(|_| ())?;
        if n == 0 { return Ok(()); }
        if n as u64 > MAX || line.last() != Some(&b'\n') { return Err(()); }
        let (data, bytes) = canonical(&line)?;
        let stamp = stamp_of(&data).ok_or(())?;
        // A forced-command process lives for one SSH stream. Serialize the
        // compare/install/render transaction across streams, never stdin waits.
        // File locking is std-only unfinished migration to Omega lock orchestration.
        let publication_lock = OpenOptions::new().read(true).write(true).create(true)
            .truncate(false).mode(0o600).open(dir.join(".publication-lock")).map_err(|_| ())?;
        publication_lock.lock().map_err(|_| ())?;
        let latest = match fs::read(dir.join("live.json")) {
            Ok(bytes) => {
                let previous: Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
                Some(stamp_of(&previous).ok_or(())?)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(_) => return Err(()),
        };
        if let Some(previous) = latest { if stamp < previous { return Err(()); } }
        let mut out = OpenOptions::new().write(true).create_new(true).mode(0o644).open(&tmp).map_err(|_| ())?;
        // Ephemeral feed: write_all plus same-directory rename gives runtime atomicity.
        // Durability across a crash is not promised, and fsync here stalls publication.
        let result = (|| {
            out.write_all(&bytes).map_err(|_| ())?;
            fs::rename(&tmp, dir.join("live.json")).map_err(|_| ())?;
            Ok(())
        })();
        if result.is_err() { let _ = fs::remove_file(&tmp); }
        result?;
        let renderer = std::env::var_os("HIVE_RENDER").unwrap_or_else(|| "/usr/local/bin/hive-render".into());
        if let Err(error) = render_board(dir, Path::new(&renderer)) {
            eprintln!("Hive board retained previous observation: {error}");
        }
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 || std::env::var("SSH_ORIGINAL_COMMAND").as_deref() != Ok("hive-feed") {
        eprintln!("Hive receiver refused request");
        std::process::exit(1);
    }
    if receive(Path::new(&args[1])).is_err() {
        eprintln!("Hive receiver rejected snapshot");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Value {
        serde_json::json!({"schema_version":1,"generated_at":"2026-10-25T20:00:00Z","sequence":"one","poll_ms":1000,"stale_after_ms":15000,"coverage":{"label":"OMP","roots":0,"agents":0,"complete":true,"notice":""},"agents":[],"interactions":[]})
    }
    #[test]
    fn rejects_unknown_public_fields_and_false_counts() {
        let mut snapshot = empty();
        assert!(validate(&snapshot).is_ok());
        snapshot["private"] = "not public".into();
        assert!(validate(&snapshot).is_err());
        snapshot.as_object_mut().unwrap().remove("private");
        snapshot["coverage"]["agents"] = 1.into();
        assert!(validate(&snapshot).is_err());
    }
    #[test]
    fn rejects_invalid_dates_and_roles() {
        let mut snapshot = empty();
        snapshot["generated_at"] = "2026-99-25T20:00:00Z".into();
        assert!(validate(&snapshot).is_err());
        snapshot = empty();
        snapshot["interactions"] = serde_json::json!([{"id":"m1","agent_id":"a1","role":"system","text":"private","at":"2026-10-25T20:00:00Z","runtime":"OMP","provider":"unknown","model":"unknown"}]);
        assert!(validate(&snapshot).is_err());
    }
    #[test]
    fn accepts_the_collector_maximum_label_length() {
        let mut snapshot = empty();
        let agent = serde_json::json!({"id":"a1","parent_id":null,"name":"x".repeat(LABEL_MAX),"runtime":"OMP","provider":"unknown","model":"unknown","role":"root","task":"t","status":"working","activity":"","activity_state":"unknown","started_at":null,"updated_at":null,"observed_at":"2026-10-25T20:00:00Z"});
        snapshot["agents"] = serde_json::json!([agent]);
        snapshot["coverage"]["agents"] = 1.into();
        snapshot["coverage"]["roots"] = 1.into();
        assert!(validate(&snapshot).is_ok(), "collector output at the byte bound must publish");
        snapshot["agents"][0]["name"] = "x".repeat(LABEL_MAX + 1).into();
        assert!(validate(&snapshot).is_err(), "oversize labels stay refused");
    }
    #[test]
    fn duplicate_keys_cannot_smuggle_unchecked_fields() {
        let valid = empty().to_string();
        let input = format!("{{\"agents\":[{{\"password\":\"private-canary\"}}],{}", &valid[1..]);
        let output = canonical(input.as_bytes()).unwrap().1;
        let text = String::from_utf8(output).unwrap();
        assert!(!text.contains("private-canary"));
        assert!(!text.contains("password"));
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), empty());
    }
    #[test]
    fn renderer_failure_and_empty_output_retain_previous_board() {
        let dir = std::env::temp_dir().join(format!("hive-board-test-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("live.json"), b"checked snapshot\n").unwrap();
        fs::write(dir.join("board.html"), b"last useful board").unwrap();
        // Fake renderer executable: all outcomes consume stdin, never consult a shell.
        for (name, source, success) in [
            ("fail", "#!/bin/sh\ncat >/dev/null\nprintf partial\nexit 7\n", false),
            ("empty", "#!/bin/sh\ncat >/dev/null\nexit 0\n", false),
            ("ok", "#!/bin/sh\ncat\n", true),
        ] {
            use std::os::unix::fs::PermissionsExt;
            let renderer = dir.join(name);
            fs::write(&renderer, source).unwrap();
            fs::set_permissions(&renderer, fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(render_board(&dir, &renderer).is_ok(), success);
            assert_eq!(fs::read(dir.join("board.html")).unwrap(),
                if success { b"checked snapshot\n".as_slice() } else { b"last useful board".as_slice() });
            assert!(!dir.join(format!(".board-tmp-{}", std::process::id())).exists());
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
