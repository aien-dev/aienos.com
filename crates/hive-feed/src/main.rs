mod feed;
mod privacy;
mod quotes;
mod source;

use chrono::{DateTime, Utc};
use std::{fs::{self, File, OpenOptions}, io::{self, Read, Write}, os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt}, path::{Path, PathBuf}, time::{Duration, Instant}};

struct Options { once: bool, out: PathBuf, sessions: PathBuf, hosts: Option<PathBuf>, since: i64, salt: PathBuf, claude: PathBuf, codex: PathBuf, private_repos: PathBuf }
fn options() -> Result<Options, &'static str> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("HOME is unavailable")?;
    let mut opts = Options { once: false, out: PathBuf::new(), sessions: home.join(".omp/agent/sessions"), hosts: None, since: Utc::now().timestamp_millis(), salt: home.join(".local/state/hive-feed/salt"), claude: home.join(".claude/projects"), codex: home.join(".codex/sessions"), private_repos: home.join(".local/state/aienos-hive/private-repos.tsv") };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--once" { opts.once = true; continue; }
        if arg == "--help" { println!("hive-feed [--once] --out PATH [--sessions DIR] [--claude DIR] [--codex DIR] [--private-repos PATH] [--hosts-file PATH] [--since RFC3339] [--salt PATH]\nDefault: poll every second. Conversation starts at startup unless --since is supplied. Only agents' final replies are published, never human messages. --private-repos lists one private repository per line as NAME or NAME<TAB>private-NN. Keep --salt and --private-repos private. No raw transcript, arguments or source errors are logged."); std::process::exit(0); }
        let value = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--out" => opts.out = value.into(), "--sessions" => opts.sessions = value.into(), "--hosts-file" => opts.hosts = Some(value.into()), "--salt" => opts.salt = value.into(),
            "--claude" => opts.claude = value.into(), "--codex" => opts.codex = value.into(), "--private-repos" => opts.private_repos = value.into(),
            "--since" => opts.since = DateTime::parse_from_rfc3339(&value).map_err(|_| "invalid --since timestamp")?.timestamp_millis(),
            _ => return Err("unknown option"),
        }
    }
    if opts.out.as_os_str().is_empty() { return Err("--out is required"); }
    if opts.out == opts.salt { return Err("output and private salt must be separate"); }
    Ok(opts)
}
fn salt(path: &Path) -> io::Result<Vec<u8>> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) { fs::create_dir_all(parent)?; }
    match OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
        Ok(mut f) => { let mut bytes = [0u8;32]; File::open("/dev/urandom")?.read_exact(&mut bytes)?; f.write_all(&bytes)?; f.sync_all()?; Ok(bytes.to_vec()) },
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            let meta = fs::symlink_metadata(path)?;
            if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 || meta.nlink() != 1 { return Err(io::Error::other("unsafe private salt")); }
            let bytes = fs::read(path)?;
            if bytes.len() != 32 { return Err(io::Error::other("invalid private salt")); }
            Ok(bytes)
        }, Err(e) => Err(e),
    }
}
/// Private repository names, one per line, optionally followed by a tab and the
/// anonymised id they publish as. An empty file explicitly declares no private names.
fn private_repos(path: &Path) -> io::Result<Vec<(String, Option<String>)>> {
    let text = fs::read_to_string(path)?;
    Ok(text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| match l.split_once('\t') {
        Some((name, id)) if !id.trim().is_empty() => (name.trim().to_owned(), Some(id.trim().to_owned())),
        Some((name, _)) => (name.trim().to_owned(), None),
        None => (l.to_owned(), None),
    }).collect())
}
fn publish(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) { fs::create_dir_all(parent)?; }
    let tmp = path.with_extension(format!("hive-{}.tmp",std::process::id()));
    let mut f = OpenOptions::new().write(true).create_new(true).mode(0o644).open(&tmp)?;
    // Ephemeral feed: runtime atomicity only, no crash durability; fsync stalls the 1 s cadence.
    let result = f.write_all(bytes).and_then(|_| fs::rename(&tmp,path));
    if result.is_err() { let _ = fs::remove_file(&tmp); }
    result
}
fn run() -> Result<(), &'static str> {
    let opts = options()?;
    let key = salt(&opts.salt).map_err(|_| "private salt unavailable or unsafe")?;
    privacy::set_private_repos(private_repos(&opts.private_repos).map_err(|_| "private repository list unreadable; provide --private-repos (an empty file declares none)")?)?;
    let mut collector = feed::Collector::new(key,opts.since);
    if opts.hosts.is_none() {
        collector = collector.with_transcripts(source::Transcript::Claude, opts.claude).with_transcripts(source::Transcript::Codex, opts.codex);
    }
    loop {
        let start = Instant::now();
        let snapshot = collector.poll(&opts.sessions,opts.hosts.as_deref(),Utc::now().timestamp_millis());
        let bytes = serde_json::to_vec(&snapshot).map_err(|_| "snapshot encoding failed")?;
        if publish(&opts.out,&bytes).is_err() { if opts.once { return Err("snapshot publication failed"); } eprintln!("hive-feed: snapshot publication failed"); }
        if opts.once { return Ok(()); }
        std::thread::sleep(Duration::from_secs(1).saturating_sub(start.elapsed()));
    }
}
fn main() { if let Err(error) = run() { eprintln!("hive-feed: {error}"); std::process::exit(1); } }

#[cfg(test)]
mod config_tests {
    use super::*;
    #[test]
    fn missing_private_repo_list_is_refused() {
        let file = std::env::temp_dir().join(format!("hive-missing-private-list-{}", std::process::id()));
        assert!(!file.exists());
        assert!(private_repos(&file).is_err());
    }
    #[test]
    fn empty_private_repo_list_is_an_explicit_empty_set() {
        let file = std::env::temp_dir().join(format!("hive-empty-private-list-{}", std::process::id()));
        drop(OpenOptions::new().write(true).create_new(true).open(&file).unwrap());
        let result = private_repos(&file);
        fs::remove_file(&file).unwrap();
        assert!(result.unwrap().is_empty());
    }
}
