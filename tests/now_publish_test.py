#!/usr/bin/env python3
"""Behavioral tests for the Spark Pages dispatcher using isolated fake git/gh."""
import json
import os
import pathlib
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
PUBLISHER = ROOT / "scripts" / "now-publish"
MAIN = "a" * 40


class PublisherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="now-publish-test-")
        self.root = pathlib.Path(self.temp.name)
        self.home = self.root / "home"
        self.bin = self.root / "bin"
        self.repo = self.home / ".local/state/now-publish/repo"
        self.state = self.home / ".local/state/now-publish"
        self.ledger = self.home / "handoffs/LEDGER.tsv"
        self.bin.mkdir(parents=True)
        self.repo.mkdir(parents=True)
        (self.repo / ".git").mkdir()
        self.ledger.parent.mkdir(parents=True)
        self.ledger.write_text("ledger-v1\n")
        (self.repo / "now.tsv").write_text("ledger-v0\n")
        self.runs = self.root / "runs.json"
        self.runs.write_text("[]")
        self.dispatches = self.root / "dispatches"
        self.capture = self.root / "pushed-ledger.tsv"
        self.ledger_status = self.root / "ledger-status"
        self.ledger_status.write_text("0")
        self._write_exe(self.bin / "git", '''#!/bin/bash
set -eu
if [[ "$*" == *"ls-remote"* ]]; then
  [[ "${FAKE_MAIN_READ_FAIL:-0}" == 0 ]] || exit 17
  printf '%s refs/heads/main\\n' "${FAKE_MAIN_SHA:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
  exit 0
fi
if [[ "$*" == *"diff --quiet -- now.tsv"* ]]; then
  [[ "$(cat "$FAKE_LEDGER_STATUS")" == 0 ]]
  exit $?
fi
if [[ "$*" == *"push"* ]]; then
  cp "$FAKE_REPO/now.tsv" "$FAKE_PUSH_CAPTURE"
  printf 0 > "$FAKE_LEDGER_STATUS"
fi
if [[ "$*" == *"rev-parse HEAD"* ]]; then
  printf '%s\\n' "${FAKE_DATA_SHA:-cccccccccccccccccccccccccccccccccccccccc}"
fi
exit 0
''')
        self._write_exe(self.bin / "gh", '''#!/bin/bash
set -eu
if [[ "${FAKE_GH_DELAY:-0}" != 0 ]]; then sleep "$FAKE_GH_DELAY"; fi
if [[ "$1 $2" == "run list" ]]; then
  [[ "${FAKE_GH_READ_FAIL:-0}" == 0 ]] || exit 22
  cat "$FAKE_RUNS"
  exit 0
fi
if [[ "$1 $2" == "workflow run" ]]; then
  printf 'dispatch\\n' >> "$FAKE_DISPATCHES"
  [[ "${FAKE_DISPATCH_FAIL:-0}" == 0 ]] || exit 23
  exit 0
fi
exit 90
''')
        self.env = os.environ.copy()
        self.env.update({
            "HOME": str(self.home), "PATH": f"{self.bin}:{os.environ['PATH']}",
            "LEDGER": str(self.ledger), "FAKE_REPO": str(self.repo),
            "FAKE_RUNS": str(self.runs), "FAKE_DISPATCHES": str(self.dispatches),
            "FAKE_PUSH_CAPTURE": str(self.capture), "FAKE_LEDGER_STATUS": str(self.ledger_status),
            "FAKE_MAIN_SHA": MAIN, "FAKE_MAIN_READ_FAIL": "0",
            "FAKE_GH_READ_FAIL": "0", "FAKE_DISPATCH_FAIL": "0",
        })

    def tearDown(self):
        self.temp.cleanup()

    @staticmethod
    def _write_exe(path, text):
        path.write_text(text)
        path.chmod(0o755)

    def pending(self, main=MAIN):
        self.state.mkdir(parents=True, exist_ok=True)
        (self.state / "pages.pending").write_text(main + "\n")

    def runs_with(self, status, conclusion=None, sha=MAIN):
        self.runs.write_text(json.dumps([{
            "status": status, "conclusion": conclusion, "headSha": sha, "event": "workflow_dispatch"
        }]))

    def idle_runs(self):
        self.runs_with("completed", "success", "d" * 40)

    def run_publisher(self, env=None, check=True, args=()):
        merged = self.env.copy()
        if env:
            merged.update(env)
        result = subprocess.run([str(PUBLISHER), *args], env=merged, text=True, capture_output=True, timeout=15)
        if check:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def dispatch_count(self):
        return len(self.dispatches.read_text().splitlines()) if self.dispatches.exists() else 0

    def test_installed_metrics_hook_is_preserved_and_published(self):
        collector = self.home / '.local/lib/renda-public/renda_metrics.py'
        collector.parent.mkdir(parents=True)
        collector.write_text('import pathlib,sys\npathlib.Path(sys.argv[2]).write_text(\'{"measured":true}\\n\')\n')
        self.ledger_status.write_text('1')
        self.runs_with('in_progress')
        self.run_publisher()
        self.assertEqual(json.loads((self.repo / 'renda-metrics.json').read_text()), {'measured': True})
        self.assertEqual(self.dispatch_count(), 0)

    def test_active_or_queued_run_is_not_cancelled_or_redundantly_dispatched(self):
        self.pending()
        for status in ("queued", "in_progress", "waiting", "pending"):
            with self.subTest(status=status):
                self.runs_with(status)
                self.run_publisher()
                self.assertEqual(self.dispatch_count(), 0)
                self.assertTrue((self.state / "pages.pending").exists())

    def test_failed_or_malformed_remote_reads_fail_closed_and_keep_pending(self):
        self.pending()
        for content, env in (("not-json", {}), ("[]", {}), ("[]", {"FAKE_GH_READ_FAIL": "1"}),
                             ('[{"status":"in_progress"}]', {}),
                             ("[]", {"FAKE_MAIN_READ_FAIL": "1"})):
            with self.subTest(content=content, env=env):
                self.runs.write_text(content)
                self.run_publisher(env)
                self.assertEqual(self.dispatch_count(), 0)
                self.assertTrue((self.state / "pages.pending").exists())

    def test_same_main_failure_waits_but_changed_main_is_eligible(self):
        self.pending()
        self.runs_with("completed", "failure", MAIN)
        self.ledger_status.write_text("1")
        self.ledger.write_text("updated-but-same-main-failed\n")
        self.run_publisher()
        self.assertEqual(self.dispatch_count(), 0)
        self.assertTrue((self.state / "pages.pending").exists())
        self.idle_runs()
        self.run_publisher()
        self.assertEqual(self.dispatch_count(), 0, "remembered same-main failure must survive run-list rotation")
        self.assertTrue((self.state / "pages.pending").exists())
        newer = "b" * 40
        self.run_publisher({"FAKE_MAIN_SHA": newer})
        self.assertEqual(self.dispatch_count(), 1)
        self.assertFalse((self.state / "pages.pending").exists())

    def test_force_is_explicit_operator_intent_after_same_main_failure(self):
        self.pending()
        self.runs_with("completed", "failure", MAIN)
        self.run_publisher()
        self.assertEqual(self.dispatch_count(), 0)
        self.run_publisher(args=("--force",))
        self.assertEqual(self.dispatch_count(), 1)
        self.assertFalse((self.state / "pages.pending").exists())

    def test_dispatch_failure_retains_pending_for_a_later_no_ledger_change_tick(self):
        self.pending()
        self.idle_runs()
        self.run_publisher({"FAKE_DISPATCH_FAIL": "1"})
        self.assertEqual(self.dispatch_count(), 1)
        self.assertTrue((self.state / "pages.pending").exists())
        self.run_publisher()
        self.assertEqual(self.dispatch_count(), 2)
        self.assertFalse((self.state / "pages.pending").exists())

    def test_pending_work_flushes_when_ledger_is_unchanged(self):
        self.pending()
        self.idle_runs()
        self.run_publisher()
        self.assertEqual(self.dispatch_count(), 1)
        self.assertFalse((self.state / "pages.pending").exists())

    def test_latest_ledger_snapshot_is_pushed_while_dispatch_waits_for_active_run(self):
        self.ledger_status.write_text("1")
        self.ledger.write_text("latest-ledger\n")
        self.runs_with("in_progress")
        self.run_publisher()
        self.assertEqual(self.capture.read_text(), "latest-ledger\n")
        self.assertTrue((self.state / "pages.pending").exists())
        self.ledger.write_text("newer-ledger\n")
        self.ledger_status.write_text("1")
        self.run_publisher()
        self.assertEqual(self.capture.read_text(), "newer-ledger\n")
        self.idle_runs()
        self.run_publisher()
        self.assertEqual(self.dispatch_count(), 1)
        self.assertFalse((self.state / "pages.pending").exists())

    def test_serialized_invocations_do_not_dispatch_twice(self):
        self.idle_runs()
        self.ledger_status.write_text("1")
        self.env["FAKE_GH_DELAY"] = "0.4"
        first = subprocess.Popen([str(PUBLISHER)], env=self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        time.sleep(0.05)
        second = subprocess.Popen([str(PUBLISHER)], env=self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        out1, err1 = first.communicate(timeout=15)
        out2, err2 = second.communicate(timeout=15)
        self.assertEqual(first.returncode, 0, out1 + err1)
        self.assertEqual(second.returncode, 0, out2 + err2)
        self.assertEqual(self.dispatch_count(), 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
