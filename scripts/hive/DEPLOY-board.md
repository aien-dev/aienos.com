# Hive edge board (proposed, not deployed)

Part 2 of #104, edge renderer and browser morph client. The Spark publication format and SSH boundary do not change.
The edge receiver validates and canonicalizes JSON, atomically installs `live.json`,
then runs the static AArch64 Omega renderer. Only exit 0 plus nonempty output can
replace `board.html`; a renderer failure logs to stderr and retains the old board.
An advisory std file lock spans reread/compare/install/render across receiver
processes; it is released before waiting for more stdin. Keep `.publication-lock`
in place: unlinking a live lock file would create two independent lock domains.
Neither this document nor a successful scratch run authorizes deployment.

## Build and gates on an AArch64 Linux host

Use the PR commit, not an unpinned checkout. The compiler is
`om/hive-page/omc`, SHA-256
`734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61`.
Language authority: omega-mv `55f9b0af47de70a3d06357985cf99ae3e45c016f`,
`docs/one_layer/OMEGESE.md` sections 9.4, 9.5 and 10. No host runtime or
foreign renderer is linked. Shell coordinates compilation; Python is an independent
oracle/measurement tool only. Existing Rust receiver glue is unfinished migration
until Omega fork/dupfd/execprog/waitpid orchestration replaces it (PR #678 host words).

```sh
sh om/hive-render/check.sh
(cd om/hive-render && sha256sum -c render.om.sha256)
nice -n 19 cargo test -j2 -p hive-feed --bin hive-receive
nice -n 19 cargo build -j2 --release -p hive-feed --bin hive-receive
python3 om/hive-render/test-receive.py target/release/hive-receive om/hive-render/hive-render
python3 om/hive-render/measure.py om/hive-render/hive-render "$SNAPSHOT" /tmp/hive-board-measurement.json
```

The measurement command reads the local snapshot, canonicalizes it exactly as the
receiver's sorted compact JSON, copies only the renderer into a unique `/tmp`
directory on `hub`, measures one execution there, compares full output hashes and
removes its scratch directory. It never writes a service, nginx config or web root.
`measurement.json` records an exercised real snapshot and single-run host timings,
not a latency distribution or a 1 Hz service qualification.

The renderer is schema-specific: receiver-canonical sorted keys, no whitespace
except one optional final newline, canonical escapes, bounded strings/arrays,
UTF-8, identifier and timestamp checks, unique ids, existing parents and acyclic
trees. Unknown/duplicate keys, unknown escapes and disallowed controls trap before
stdout. Noncanonical JSON is deliberately refused; never feed the collector's
pretty-printed JSON directly. The receiver canonicalizes it first.

Text is escaped with the same five replacements as Python `html.escape(quote=True)`.
`data-generated` is a rechecked timestamp; ids and id-derived links are rechecked
identifiers. `data-sequence` is `s-` followed by lowercase hex of the original
sequence's UTF-8 bytes, so even arbitrary receiver-accepted text becomes an
ident-shaped attribute; it has no authorization meaning. No payload text becomes
markup. All output is buffered before writing; two fixed output pools accommodate
the maximum input with worst-case escaping and repeated manifest text.

Closed native `<details>` expose manifests without JavaScript. The morph client
keeps the manifest storage container hidden and moves the selected card into the
manifest panel, preserving native disclosure state and keyed row identity.
Elapsed time and observation lag use only the source's timestamps, never render time.
The no-script document explicitly discloses its inability to infer connection
freshness and refreshes every 15 seconds. A frozen source remains visibly timestamped.
Install and verify the edge board in the authorized window **before** merging or
deploying the browser cutover. Until `/hive/board/` exists, the new client correctly
reports telemetry unavailable; keeping this PR draft prevents an accidental gap.

## Install on hub only after current operator authorization

Before starting, set `PR_COMMIT`, `RECEIVER_DIR`, `TLS_SITE` and `PUBLISH_USER` from
the existing installation. `RECEIVER_DIR` must be the already-configured receiver
directory, not a new path from an SSH request. Inspect `authorized_keys`'s existing
forced command and nginx's existing TLS site to obtain the values. Do not copy
private addresses or keys into this repository.

1. Pin the checked checkout to `PR_COMMIT`; run the gates above. Record
   `sha256sum target/release/hive-receive om/hive-render/hive-render` and compare the
   renderer digest with `om/hive-render/hive-render.sha256`.
2. Copy both binaries and `scripts/hive/nginx-location.conf` to a unique hub scratch
   directory. Back up `/usr/local/bin/hive-receive`, any existing
   `/usr/local/bin/hive-render`, `$TLS_SITE`, the included location file and any
   existing `$RECEIVER_DIR/board.html` to an operator-owned timestamped directory.
   Record which files did not previously exist.
3. Install each binary atomically: `sudo install -m755 NEW /usr/local/bin/NAME.next`
   followed by `sudo mv /usr/local/bin/NAME.next /usr/local/bin/NAME`. The default
   renderer path is `/usr/local/bin/hive-render`. If a different trusted executable
   is necessary, set `HIVE_RENDER` in the operator-controlled receiver environment,
   never in `SSH_ORIGINAL_COMMAND`. No shell arguments are accepted as a renderer path.
4. Prime the board as the existing publish user (not root):

   ```sh
   sudo -u "$PUBLISH_USER" sh -c '
     set -eu
     d=$1
     /usr/local/bin/hive-render < "$d/live.json" > "$d/.board-prime"
     test -s "$d/.board-prime"
     mv "$d/.board-prime" "$d/board.html"
   ' sh "$RECEIVER_DIR"
   ```

   This input must be the receiver's canonical file. On refusal remove only the
   `.board-prime` you just created, investigate stderr and keep the previous board.
5. Install the exact-route nginx include inside the existing www TLS server,
   replacing the old live.json-only include. Its aliases assume the existing
   `/var/www/aienos-hive` directory; preserve the real installation's alias if it
   differs. Do not create a prefix route or directory listing. The board route has
   strict CSP, no JavaScript and no tracking; it bypasses static-site head enrichment
   and must remain excluded from the consent pixel's global enrichment.
6. `sudo nginx -t`; on failure restore the nginx backup and do not reload. On
   success, and only within the authorized deployment, `sudo systemctl reload nginx`.
   Receiver SSH processes are per connection: existing streams keep their old
   executable until a newly authorized publisher reconnect. Do not restart a live
   publisher merely to force this without authority.
7. Verify from an external client:

   ```sh
   curl -fsSI https://www.aienos.com/hive/board/
   curl -fsS https://www.aienos.com/hive/board/ -o /tmp/board.html
   curl -fsS https://www.aienos.com/hive/live.json -o /tmp/live.json
   ```

   Require 200, `text/html`, no-store, nosniff, HSTS, no-referrer and the exact board
   CSP in the template. Check `#hive-board` data-generated matches the JSON snapshot
   (capture coherent files server-side when publication races a request). Read a
   later observation to ensure both timestamps advance. Open the board with JS
   disabled; expand a branch and a manifest, read messages, and check the source
   timestamp and refresh disclosure. Verify partial/failing renderer retains the
   old board in a disposable directory with `test-receive.py`, never by replacing
   the live renderer with a fake. Check receiver stderr in the publisher journal
   for `Hive board retained previous observation`.

## Proposed /hive/ CSP, separate from the board

Raw production `/hive/` and generated `dist/hive/index.html` were inspected on
2026-10-26. Both contain two external first-party executable scripts and one inline
`application/ld+json` data block. They do **not** have zero inline script elements;
they have zero inline executable scripts. Keep the distinction in deployment claims.
The page's decorative inline style and style attributes also remain; do not set
`default-src 'none'` on that page without a separate style migration.

Without the separately proposed consent pixel, the proposed policy is:

```nginx
add_header Content-Security-Policy "script-src 'self' 'wasm-unsafe-eval'; object-src 'none'; base-uri 'none'" always;
```

The concurrent X logbook change adds only external `/js/logbook.js` and, **after
consent**, `https://static.ads-twitter.com/uwt.js`. Coordination with its author
identified `analytics.x.com`, `analytics.twitter.com` and `https://t.co/` in the
fetched official pixel. If that change is installed, use this proposed policy
instead, after checking the exact installed script and browser requests again:

```nginx
add_header Content-Security-Policy "script-src 'self' 'wasm-unsafe-eval' https://static.ads-twitter.com; connect-src 'self' https://analytics.x.com https://analytics.twitter.com https://t.co; img-src 'self' data: https://analytics.x.com https://analytics.twitter.com https://t.co; object-src 'none'; base-uri 'none'" always;
```
The explicitly authorized `wasm-unsafe-eval` keyword permits WebAssembly
compilation for the seeded freshness unit; it does **not** permit JavaScript
`eval`, `Function` or string-based timers. Without it Chromium would deliberately
take the existing JavaScript fallback. Retain that fallback and the visible
`data-fresh-engine` observation even when the proposed policy enables WASM.
This remains tighter than the currently inspected main page's lack of CSP.

CSP permits hosts statically; consent is enforced by the first-party consent code,
not by CSP. Check denial makes no third-party requests and consent enables only
these hosts. This policy is a proposal, not installed by the board include. Check
JSON-LD behavior and CSP console reports in the actual deployment before enforcing.
The board policy **never** gets pixel hosts or scripts.

## Rollback

Restore both binaries atomically from the recorded backups (remove hive-render only
if the pre-install record says it did not exist), restore the prior nginx include
and TLS site, run `sudo nginx -t`, then reload only on success. Restore board.html
from its backup; if it did not exist, remove only this newly installed board.html
and remove the exact board route. Preserve live.json and the unchanged publication
key. New receiver connections use the restored binary. Confirm live.json continues
to advance, the old /hive/ works and no scratch file is publicly reachable. Do not
merge, deploy, restart live services or change authorization as part of a scratch run.

## Arithmetic oracle

`WolframLanguageEvaluator: Integrate[x^2,{x,0,1}] -> 1/3` and
`WolframAlpha: sqrt(27)/2 -> (3 sqrt(3))/2` verified the mounted calculator.
The Gregorian formula used by `seconds` was checked with:

```wolfram
leap[y_]:=Mod[y,4]==0&&(Mod[y,100]!=0||Mod[y,400]==0);
months[y_]:={31,If[leap[y],29,28],31,30,31,30,31,31,30,31,30,31};
ord[y_,m_,d_]:=365 (y-1)+Floor[(y-1)/4]-Floor[(y-1)/100]+Floor[(y-1)/400]+Total[Take[months[y],m-1]]+d;
sec[y_,m_,d_,h_,mi_,s_]:=((ord[y,m,d] 24+h) 60+mi) 60+s;
{Table[ord[y,3,1]-ord[y,2,28],{y,{1900,2000,2024,2026}}],
 sec[2026,10,26,0,0,1]-sec[2026,10,25,23,59,59],
 Floor[(1000 sec[2026,10,26,0,0,1]+100-(1000 sec[2026,10,25,23,59,59]+900))/1000]}
```

Result: `{{1,2,2,1},2,1}` (calendar days, seconds, floored seconds respectively).
The implementation shifts years by one Gregorian 400-year cycle to keep year 0000
unsigned; differences are unchanged. The Python date oracle checks independent
calendar boundaries; it is test-only, not a production dependency.
