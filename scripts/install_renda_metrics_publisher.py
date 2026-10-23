#!/usr/bin/env python3
"""Add sanitized telemetry to the existing publisher; compare before replacing."""
from pathlib import Path
import datetime,subprocess,hashlib
h=Path.home();live=h/'.local/bin/now-publish';original=h/'handoffs/now-publish-dashboard-original';old=original.read_bytes()
assert live.read_bytes()==old,'Publisher changed; reconcile before installation'
s=old.decode()
s=s.replace('mkdir -p "$STATE"','mkdir -p "$STATE"\nexec 9>"$STATE/publish.lock"\nflock -n 9 || exit 0',1)
s=s.replace('if [ "${1:-}" != "--force" ] && git diff --quiet -- now.tsv; then','python3 "$HOME/.local/lib/renda-public/renda_metrics.py" --output renda-metrics.json\n\nif [ "${1:-}" != "--force" ] && git diff --quiet -- now.tsv renda-metrics.json && [ -z "$(git ls-files --others --exclude-standard -- renda-metrics.json)" ]; then',1)
s=s.replace('git add now.tsv now-pulse.tsv','git add now.tsv now-pulse.tsv renda-metrics.json',1)
assert s!=old.decode()
candidate=live.with_name('now-publish.dashboard-candidate');candidate.write_text(s);candidate.chmod(0o755)
subprocess.run(['bash','-n',str(candidate)],check=True)
backup=live.with_name('now-publish.bak-dashboard-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ'));backup.write_bytes(old);backup.chmod(0o755)
assert live.read_bytes()==old
candidate.replace(live)
print('installed',hashlib.sha256(live.read_bytes()).hexdigest(),'backup',backup)
