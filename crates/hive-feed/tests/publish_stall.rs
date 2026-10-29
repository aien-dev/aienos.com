//! The publisher must not treat a local write as delivery, and a receiver
//! that never reads must not freeze the process.
use std::fs;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_hive-publish")
}

fn write_ssh(dir: &std::path::Path) {
    let path = dir.join("ssh");
    fs::write(
        &path,
        "#!/bin/sh\n\
         echo start >> \"$HIVE_FAKE_LOG\"\n\
         if [ \"$HIVE_FAKE_MODE\" = hang ]; then\n\
           exec sleep 120\n\
         fi\n\
         if [ \"$HIVE_FAKE_MODE\" = refuse ]; then\n\
           echo 'Hive receiver rejected snapshot' >&2\n\
           exit 1\n\
         fi\n\
         if [ \"$HIVE_FAKE_MODE\" = ok ]; then\n\
           exec cat >/dev/null\n\
         fi\n\
         exit 1\n",
    )
    .unwrap();
    let mut perm = fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perm.set_mode(0o755);
    }
    fs::set_permissions(&path, perm).unwrap();
}

fn snapshot(path: &std::path::Path, sequence: &str, pad: usize) {
    let body = "x".repeat(pad);
    let text = format!("{{\"sequence\":{sequence:?},\"body\":{body:?}}}");
    fs::write(path, text).unwrap();
}

struct Publisher {
    child: std::process::Child,
    dir: std::path::PathBuf,
}

impl Publisher {
    fn start(mode: &str, sequence: &str, pad: usize) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "aienosqueue-hive-{}-{}",
            mode,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_ssh(&dir);
        let snap = dir.join("live.json");
        snapshot(&snap, sequence, pad);
        fs::write(dir.join("key"), b"not-a-key").unwrap();
        fs::write(dir.join("known"), b"").unwrap();
        let log = dir.join("ssh.log");
        fs::write(&log, b"").unwrap();
        let mut prefixed = dir.as_os_str().to_owned();
        prefixed.push(":");
        if let Some(path) = std::env::var_os("PATH") {
            prefixed.push(path);
        }
        let child = Command::new(bin())
            .args([
                snap.to_str().unwrap(),
                "edge.example",
                "hive",
                dir.join("key").to_str().unwrap(),
                dir.join("known").to_str().unwrap(),
            ])
            .env("PATH", prefixed)
            .env("HIVE_FAKE_MODE", mode)
            .env("HIVE_FAKE_LOG", &log)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        Publisher { child, dir }
    }

    fn stops(&mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut buf = String::new();
        if let Some(mut err) = self.child.stderr.take() {
            let _ = err.read_to_string(&mut buf);
        }
        buf
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn wait_until(mut pred: impl FnMut() -> bool, limit: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if pred() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    pred()
}

#[test]
fn usage_exits_2() {
    let status = Command::new(bin()).status().unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn hung_receiver_is_killed_and_retried() {
    let mut pub_ = Publisher::start("hang", "seq-hang-1", 80_000);
    let log = pub_.dir.join("ssh.log");
    let retried = wait_until(
        || fs::read_to_string(&log).unwrap_or_default().matches("start").count() >= 2,
        Duration::from_secs(12),
    );
    let starts = fs::read_to_string(&log).unwrap_or_default().matches("start").count();
    let err = pub_.stops();
    assert!(retried, "publisher stayed on one hung ssh; log starts={starts}, stderr={err}");
    assert!(err.contains("hive-publish: write stalled; reconnecting"), "{err}");
    assert!(!err.contains("sent sequence"), "a stalled write must not be recorded as sent: {err}");
    assert!(!err.contains("xxxxxxxx"), "snapshot body must not be logged: {err}");
}

#[test]
fn refusing_receiver_is_not_recorded_as_sent() {
    let mut pub_ = Publisher::start("refuse", "seq-refuse-1", 0);
    let log = pub_.dir.join("ssh.log");
    let retried = wait_until(
        || fs::read_to_string(&log).unwrap_or_default().matches("start").count() >= 2,
        Duration::from_secs(8),
    );
    let err = pub_.stops();
    assert!(retried, "publisher did not reconnect after refusal; stderr={err}");
    assert!(
        err.contains("receiver refused snapshot") || err.contains("Hive transport interrupted"),
        "{err}"
    );
    assert!(!err.contains("sent sequence"), "a refusal must not be recorded as sent: {err}");
}

#[test]
fn live_receiver_records_one_send_and_stays_up() {
    let mut pub_ = Publisher::start("ok", "seq-ok-1", 0);
    let log = pub_.dir.join("ssh.log");
    let up = wait_until(
        || {
            fs::read_to_string(&log).unwrap_or_default().matches("start").count() == 1
                && pub_.child.try_wait().ok().flatten().is_none()
        },
        Duration::from_secs(3),
    );
    assert!(up, "healthy ssh did not stay up");
    thread::sleep(Duration::from_secs(2));
    let starts = fs::read_to_string(&log).unwrap_or_default().matches("start").count();
    let err = pub_.stops();
    assert_eq!(starts, 1, "a live receiver must not be torn down; stderr={err}");
    assert!(err.contains("hive-publish: sent sequence seq-ok-1"), "{err}");
    assert!(!err.contains("write stalled"), "{err}");
}
