#!/usr/bin/env python3
# Run hard20.jsonl against an aien-math binary and judge each row with the corpus's own
# tests/oracle_common.verdict (no second matcher). Usage: run_hard20.py ENGINE_ROOT [BIN]
import json, os, subprocess, sys
root = sys.argv[1]
binp = sys.argv[2] if len(sys.argv) > 2 else os.path.join(root, "build", "aien-math")
sys.path.insert(0, os.path.join(root, "tests"))
from oracle_common import load, verdict, expected  # noqa: E402
here = os.path.dirname(os.path.abspath(__file__))
cases = load(os.path.join(here, "hard20.jsonl"))
out, tally = [], {}
for c in cases:
    try:
        p = subprocess.run([binp, "eval", "-e", c["code"]], capture_output=True, text=True, timeout=60)
        got, rc = p.stdout.rstrip("\n"), p.returncode
    except subprocess.TimeoutExpired:
        got, rc = "<timeout>", -1
    v = verdict(c, got, rc)
    tally[v] = tally.get(v, 0) + 1
    out.append({"id": c["id"], "topic": c["topic"], "code": c["code"], "want": expected(c),
                "compare": c.get("compare", "exact"), "source": c["source"],
                "oracle_row": c["oracle_row"], "observed": got, "rc": rc, "verdict": v})
    print(f"{c['id']:3s} {v:12s} rc={rc} {c['code'][:70]}")
json.dump({"binary": binp, "results": out, "tally": tally}, open(os.path.join(here, "hard20-results.json"), "w"), indent=1)
print("tally", tally)
sys.exit(0 if tally.get("pass", 0) == len(cases) else 1)
