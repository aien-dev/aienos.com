mod feed;
mod privacy;
mod source;

use chrono::{DateTime, Utc};
use std::{fs::{self, File, OpenOptions}, io::{self, Read, Write}, os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt}, path::{Path, PathBuf}, time::{Duration, Instant}};

struct Options { once: bool, out: PathBuf, sessions: PathBuf, hosts: Option<PathBuf>, since: i64, salt: PathBuf }
fn options() -> Result<Options, &'static str> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("HOME is unavailable")?;
    let mut opts = Options { once: false, out: PathBuf::new(), sessions: home.join(".omp/agent/sessions"), hosts: None, since: Utc::now().timestamp_millis(), salt: home.join(".local/state/hive-feed/salt") };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--once" { opts.once = true; continue; }
        if arg == "--help" { println!("hive-feed [--once] --out PATH [--sessions DIR] [--hosts-file PATH] [--since RFC3339] [--salt PATH]\nDefault: poll every second. Conversation starts at startup unless --since is supplied. Keep --salt private and persistent. No raw transcript, arguments or source errors are logged."); std::process::exit(0); }
        let value = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--out" => opts.out = value.into(), "--sessions" => opts.sessions = value.into(), "--hosts-file" => opts.hosts = Some(value.into()), "--salt" => opts.salt = value.into(),
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
    let mut collector = feed::Collector::new(key,opts.since);
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
