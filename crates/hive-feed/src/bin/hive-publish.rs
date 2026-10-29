//! Sends only already-filtered snapshots through a forced-command SSH key.
//!
//! A local write is not delivery. The sequence is marked sent only after the
//! write finishes, the child is still running, and stderr has no refusal.
//! A write that does not finish within five seconds, a child that exits, or
//! a refusal tears the session down and the next session sends the snapshot
//! again. That is what stops a hung or refusing receiver from freezing
//! publication until the service is restarted.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const WRITE_DEADLINE: Duration = Duration::from_secs(5);
const SURVIVE_GRACE: Duration = Duration::from_millis(200);
const HEARTBEAT: Duration = Duration::from_secs(30);
const RETRY: Duration = Duration::from_secs(3);

fn refusal(text: &str) -> bool {
    text.contains("Hive receiver refused request") || text.contains("Hive receiver rejected snapshot")
}

/// Delivery requires a finished write, a child that is still running, and no refusal.
fn delivered(write_finished: bool, child_running: bool, refused: bool) -> bool {
    write_finished && child_running && !refused
}

fn connect(host: &str, user: &str, identity: &str, known_hosts: &str) -> std::io::Result<Child> {
    Command::new("ssh")
        .args([
            "-F", "/dev/null",
            "-T",
            "-i",
            identity,
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ClearAllForwardings=yes",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ServerAliveInterval=10",
            "-o",
            "ServerAliveCountMax=2",
            "-o",
            "LogLevel=ERROR",
            "-o",
            &format!("UserKnownHostsFile={known_hosts}"),
            "-l",
            user,
            host,
            "hive-feed",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

struct Writer {
    tx: Sender<Option<(Vec<u8>, Sender<std::io::Result<()>>)>>,
}

impl Writer {
    fn spawn(stdin: ChildStdin) -> Self {
        let (tx, rx) = mpsc::channel::<Option<(Vec<u8>, Sender<std::io::Result<()>>)>>();
        thread::spawn(move || {
            let mut stdin = stdin;
            while let Ok(Some((line, reply))) = rx.recv() {
                let result = stdin.write_all(&line).and_then(|_| stdin.flush());
                let _ = reply.send(result);
            }
        });
        Writer { tx }
    }

    fn write(&self, line: Vec<u8>, deadline: Duration) -> std::io::Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Some((line, tx)))
            .map_err(|_| std::io::Error::other("writer gone"))?;
        match rx.recv_timeout(deadline) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "publish write stalled"))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(std::io::Error::other("writer dropped the write"))
            }
        }
    }
}

fn spawn_stderr(child: &mut Child) -> std::io::Result<(thread::JoinHandle<()>, Arc<Mutex<String>>)> {
    let stderr = child.stderr.take().ok_or_else(|| std::io::Error::other("stderr was not piped"))?;
    let buf = Arc::new(Mutex::new(String::new()));
    let share = Arc::clone(&buf);
    let handle = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let mut guard = share.lock().unwrap_or_else(|poison| poison.into_inner());
                    if guard.len() < 4096 {
                        guard.push_str(&line);
                    }
                }
                Err(_) => break,
            }
        }
    });
    Ok((handle, buf))
}

fn child_running(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}

fn survived(child: &mut Child, grace: Duration) -> bool {
    let start = Instant::now();
    loop {
        if !child_running(child) {
            return false;
        }
        if start.elapsed() >= grace {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn notes_of(buf: &Mutex<String>) -> String {
    buf.lock().unwrap_or_else(|poison| poison.into_inner()).clone()
}

fn snapshot_line(path: &Path) -> Option<(String, Vec<u8>)> {
    let meta = fs::metadata(path).ok()?;
    if meta.len() > 2 * 1024 * 1024 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let sequence = value.get("sequence")?.as_str()?.to_string();
    if sequence.is_empty() {
        return None;
    }
    let mut line = serde_json::to_vec(&value).expect("JSON value serializes");
    line.push(b'\n');
    Some((sequence, line))
}

/// One SSH session. Returns when the session must be torn down and retried.
fn run_session(ssh: &mut Child, writer: &Writer, stderr_buf: &Mutex<String>, path: &Path) {
    let mut sent = String::new();
    let mut last_beat = Instant::now();
    loop {
        if !child_running(ssh) {
            eprintln!("Hive transport interrupted; retrying");
            return;
        }
        if refusal(&notes_of(stderr_buf)) {
            eprintln!("hive-publish: receiver refused snapshot; reconnecting");
            return;
        }
        if last_beat.elapsed() >= HEARTBEAT {
            let shown = if sent.is_empty() { "(none)" } else { sent.as_str() };
            eprintln!("hive-publish: transport up, last sequence {shown}");
            last_beat = Instant::now();
        }
        if let Some((sequence, line)) = snapshot_line(path) {
            if sequence != sent {
                match writer.write(line, WRITE_DEADLINE) {
                    Ok(()) => {
                        let refused = refusal(&notes_of(stderr_buf));
                        let running = survived(ssh, SURVIVE_GRACE);
                        if delivered(true, running, refused) {
                            sent = sequence;
                            eprintln!("hive-publish: sent sequence {sent}");
                        } else if refused {
                            eprintln!("hive-publish: receiver refused snapshot; reconnecting");
                            return;
                        } else {
                            eprintln!("Hive transport interrupted; retrying");
                            return;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                        eprintln!("hive-publish: write stalled; reconnecting");
                        return;
                    }
                    Err(_) => {
                        eprintln!("Hive transport interrupted; retrying");
                        return;
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
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
            Err(_) => {
                eprintln!("Hive transport unavailable; retrying");
                thread::sleep(RETRY);
                continue;
            }
        };
        let stdin = match ssh.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = ssh.kill();
                let _ = ssh.wait();
                eprintln!("Hive transport interrupted; retrying");
                thread::sleep(RETRY);
                continue;
            }
        };
        let writer = Writer::spawn(stdin);
        let (stderr_thread, stderr_buf) = match spawn_stderr(&mut ssh) {
            Ok(pair) => pair,
            Err(_) => {
                let _ = ssh.kill();
                let _ = ssh.wait();
                eprintln!("Hive transport interrupted; retrying");
                thread::sleep(RETRY);
                continue;
            }
        };
        run_session(&mut ssh, &writer, &stderr_buf, path);
        // Kill before dropping the writer so a blocked write unblocks.
        let _ = ssh.kill();
        let _ = ssh.wait();
        drop(writer);
        let _ = stderr_thread.join();
        thread::sleep(RETRY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_matches_only_receiver_lines() {
        assert!(refusal("Hive receiver refused request\n"));
        assert!(refusal("prefix Hive receiver rejected snapshot\n"));
        assert!(!refusal("hive-publish: sent sequence abc"));
        assert!(!refusal(""));
    }

    #[test]
    fn delivery_requires_a_live_child_and_no_refusal() {
        assert!(delivered(true, true, false));
        assert!(!delivered(false, true, false));
        assert!(!delivered(true, false, false));
        assert!(!delivered(true, true, true));
    }
}
