//! Sends only already-filtered snapshots through a forced-command SSH key.
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

fn connect(host: &str, user: &str, identity: &str, known_hosts: &str) -> std::io::Result<Child> {
    Command::new("ssh")
        .args(["-F", "/dev/null", "-T", "-i", identity,
            "-o", "BatchMode=yes", "-o", "IdentitiesOnly=yes",
            "-o", "StrictHostKeyChecking=yes", "-o", "ClearAllForwardings=yes",
            "-o", "ConnectTimeout=10", "-o", "ServerAliveInterval=10",
            "-o", "ServerAliveCountMax=2", "-o", "LogLevel=ERROR",
            "-o", &format!("UserKnownHostsFile={known_hosts}"),
            "-l", user, host, "hive-feed"])
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 || args[2].starts_with('-') || args[3].starts_with('-') {
        eprintln!("Usage: hive-publish SNAPSHOT HOST USER IDENTITY KNOWN_HOSTS");
        std::process::exit(2);
    }
    let path = Path::new(&args[1]);
    loop {
        let mut ssh = match connect(&args[2], &args[3], &args[4], &args[5]) {
            Ok(child) => child,
            Err(_) => { eprintln!("Hive transport unavailable; retrying"); thread::sleep(Duration::from_secs(3)); continue; }
        };
        let mut sent = String::new();
        loop {
            if !matches!(ssh.try_wait(), Ok(None)) { break; }
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() <= 2 * 1024 * 1024 {
                    if let Ok(bytes) = fs::read(path) {
                        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if let Some(sequence) = value.get("sequence").and_then(|v| v.as_str()) {
                                if sequence != sent {
                                    // Compact to one newline-delimited record; transport never reads raw sources.
                                    let mut line = serde_json::to_vec(&value).expect("JSON value serializes");
                                    line.push(b'\n');
                                    if ssh.stdin.as_mut().is_none_or(|input| input.write_all(&line).is_err()) { break; }
                                    sent.clear();
                                    sent.push_str(sequence);
                                }
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
        let _ = ssh.kill();
        let _ = ssh.wait();
        eprintln!("Hive transport interrupted; retrying");
        thread::sleep(Duration::from_secs(3));
    }
}
