use serde_json::Value;
use std::{collections::HashSet, fs::{self, File}, io::{self, BufRead, BufReader, Read, Seek, SeekFrom}, os::unix::fs::MetadataExt, path::{Path, PathBuf}, process::{Command, Stdio}, time::{Duration, Instant}};

const MAX_LINE: usize = 8 * 1024 * 1024;
#[derive(Default)]
pub struct Cursor { offset: u64, identity: Option<(u64,u64)>, pending: Vec<u8>, discarding: bool, pub damaged: bool }
impl Cursor {
    pub fn read(&mut self, path: &Path) -> io::Result<Vec<Value>> {
        let mut file = File::open(path)?;
        let meta = file.metadata()?;
        let identity = (meta.dev(), meta.ino());
        if self.identity != Some(identity) || meta.len() < self.offset {
            self.offset = 0; self.pending.clear(); self.discarding = false; self.damaged = false;
        }
        self.identity = Some(identity);
        let mut values = Vec::new();
        // The fixed-width title can be rewritten without an append or size change.
        let mut title = Vec::new();
        BufReader::new((&mut file).take(65536)).read_until(b'\n', &mut title)?;
        if let Ok(v) = serde_json::from_slice::<Value>(&title) { if v["type"] == "title" { values.push(v); } }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = [0u8; 65536];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            self.offset += n as u64;
            for &b in &buf[..n] {
                if b == b'\n' {
                    if !self.discarding && !self.pending.is_empty() {
                        match serde_json::from_slice(&self.pending) { Ok(v) => values.push(v), Err(_) => self.damaged = true }
                    }
                    self.pending.clear(); self.discarding = false;
                } else if !self.discarding {
                    if self.pending.len() == MAX_LINE { self.pending.clear(); self.discarding = true; self.damaged = true; } else { self.pending.push(b); }
                }
            }
        }
        Ok(values)
    }
}

pub fn hosts(fixture: Option<&Path>) -> io::Result<Value> {
    let bytes = if let Some(path) = fixture { fs::read(path)? } else {
        // Drain stdout concurrently; never forward raw stderr or source errors.
        let mut child = Command::new("omp").args(["collab", "list", "--json"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| io::Error::other("source unavailable"))?;
        let reader = std::thread::spawn(move || { let mut bytes = Vec::new(); stdout.take(16 * 1024 * 1024).read_to_end(&mut bytes).map(|_| bytes) });
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? { if !status.success() { let _ = reader.join(); return Err(io::Error::other("source unavailable")); } break; }
            if start.elapsed() > Duration::from_secs(10) { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err(io::Error::other("source timeout")); }
            std::thread::sleep(Duration::from_millis(10));
        }
        reader.join().map_err(|_| io::Error::other("source unavailable"))??
    };
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| io::Error::other("invalid source"))?;
    if value["version"] != 1 || !value["hosts"].is_array() { return Err(io::Error::other("invalid source")); }
    Ok(value)
}

pub fn root_paths(base: &Path, ids: &HashSet<String>) -> io::Result<Vec<(String,PathBuf)>> {
    let mut result = Vec::new();
    for dir in fs::read_dir(base)? {
        let dir = dir?;
        if !dir.file_type()?.is_dir() { continue; }
        for entry in fs::read_dir(dir.path())? {
            let entry = entry?;
            let name = entry.file_name(); let name = name.to_string_lossy();
            if let Some(stem) = name.strip_suffix(".jsonl") {
                if let Some((_, id)) = stem.rsplit_once('_') { if ids.contains(id) { result.push((id.to_owned(), entry.path())); } }
            }
        }
    }
    Ok(result)
}

pub fn descendants(base: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !base.exists() { return Ok(files); }
    let mut dirs = vec![base.to_owned()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?; let kind = entry.file_type()?;
            if kind.is_dir() { dirs.push(entry.path()); }
            else if kind.is_file() && entry.path().extension().is_some_and(|x| x == "jsonl") {
                // These are mirrored advisory transcript wrappers, not workers. Real
                // advisor-spawned workers have normal session_init files and remain.
                if !entry.file_name().to_string_lossy().starts_with("__advisor.") { files.push(entry.path()); }
            }
        }
    }
    files.sort_by_key(|p| p.components().count());
    Ok(files)
}

pub struct Process { pub key: String, pub pid: u32, pub parent: u32, pub runtime: &'static str, pub model: String, pub headless: bool }
pub fn processes() -> io::Result<Vec<Process>> {
    let mut result = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else { continue; };
        let path = entry.path();
        let Ok(comm) = fs::read_to_string(path.join("comm")) else { continue; };
        let runtime = match comm.trim() { "omp" => "OMP", "codex" => "Codex", "claude" => "Claude Code", "opencode" => "OpenCode", "grok" => "Grok", "muse" => "Muse", _ => continue };
        let Ok(cmd) = fs::read(path.join("cmdline")) else { continue; };
        let args: Vec<_> = cmd.split(|&b| b == 0).filter_map(|a| std::str::from_utf8(a).ok()).collect();
        if args.iter().any(|a| matches!(*a, "app-server" | "mcp-server" | "--service" | "daemon" | "collab")) { continue; }
        let Ok(stat) = fs::read_to_string(path.join("stat")) else { continue; };
        let Some((_, fields)) = stat.rsplit_once(')') else { continue; };
        let fields: Vec<_> = fields.split_whitespace().collect();
        if fields.first() == Some(&"Z") { continue; }
        let parent = fields.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let born = fields.get(19).copied().unwrap_or("unknown");
        let model = args.windows(2).find(|a| a[0] == "--model").map(|a| a[1]).unwrap_or("unknown").to_owned();
        result.push(Process { key: format!("process:{pid}:{born}"), pid, parent, runtime, model, headless: args.contains(&"-p") || args.contains(&"--print") });
    }
    Ok(result)
}

pub fn nearest_parent(mut pid: u32, known: &std::collections::HashMap<u32,String>) -> Option<String> {
    let mut visited = HashSet::new();
    while pid > 1 && visited.insert(pid) {
        if let Some(key) = known.get(&pid) { return Some(key.clone()); }
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let (_, fields) = stat.rsplit_once(')')?;
        pid = fields.split_whitespace().nth(1)?.parse().ok()?;
    }
    None
}

/// Transcript formats read beside OMP collaboration trees.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Transcript { Claude, Codex }
impl Transcript {
    pub fn runtime(self) -> &'static str { match self { Self::Claude => "Claude Code", Self::Codex => "Codex" } }
}
/// Top-level session transcripts written within `window_ms` of `now_ms`, with
/// their modification time. Claude Code keeps `projects/<project>/<session>.jsonl`
/// (subagent sidechains live deeper and are skipped); Codex keeps
/// `sessions/YYYY/MM/DD/rollout-*.jsonl`. A missing directory is no transcripts.
pub fn recent_transcripts(kind: Transcript, base: &Path, now_ms: i64, window_ms: i64) -> io::Result<Vec<(PathBuf,i64)>> {
    let depth = match kind { Transcript::Claude => 1, Transcript::Codex => 3 };
    let mut result = Vec::new();
    if !base.exists() { return Ok(result); }
    let mut dirs = vec![(base.to_owned(),0)];
    while let Some((dir,level)) = dirs.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?; let kind_of = entry.file_type()?;
            if kind_of.is_dir() && level < depth { dirs.push((entry.path(),level + 1)); continue; }
            if !kind_of.is_file() || level != depth || entry.path().extension().is_none_or(|e| e != "jsonl") { continue; }
            let Ok(meta) = entry.metadata() else { continue; };
            let modified = meta.mtime() * 1000 + meta.mtime_nsec() / 1_000_000;
            if now_ms - modified <= window_ms { result.push((entry.path(),modified)); }
        }
    }
    result.sort();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test] fn incremental_partial_malformed_and_rotation() {
        let dir = std::env::temp_dir().join(format!("hive-cursor-{}", std::process::id())); fs::create_dir_all(&dir).unwrap();
        let p = dir.join("source.jsonl"); fs::write(&p, b"{\"type\":\"title\",\"title\":\"one\"}\n{\"a\":").unwrap();
        let mut c = Cursor::default(); assert_eq!(c.read(&p).unwrap().len(), 2);
        fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"1}\nbad\n").unwrap();
        let v = c.read(&p).unwrap(); assert_eq!(v.len(), 2); assert_eq!(v[1]["a"], 1); assert!(c.damaged);
        fs::write(&p, b"{\"b\":2}\n").unwrap(); let v = c.read(&p).unwrap(); assert_eq!(v[0]["b"], 2);
        fs::rename(&p, dir.join("old")).unwrap(); fs::write(&p, b"{\"c\":3}\n").unwrap(); assert_eq!(c.read(&p).unwrap()[0]["c"], 3);
        fs::remove_dir_all(dir).unwrap();
    }
}
