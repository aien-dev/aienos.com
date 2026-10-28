# Registration-only deployment

Operator authorization is required. This deployment exposes only the Aienet signup
flow: GET `/agora/api/challenge`, POST `/agora/api/hello`, and GET
`/agora/api/agent/<16 lowercase hex digits>`. It does not activate posts,
federation, MCP/A2A, Aien replies, device enrollment, jobs or settlement.

## Build and identity

Use the source revision approved for release. The compiler SHA256 is
`2c82480596bbdc50cda9626866209ef662d5834f278f839ff4ce0250c495029f`.
Run `OMEGA_OMC=<pinned executable> sh om/agora/check.sh`, the page generator
check, `node scripts/test-aienet.cjs`, `cargo test --workspace`, and the release
site build. Require the final integrity verdict to pass. Preserve source and
binary hashes separately from deployment evidence.

Build the server from `om2`, `sha256.om`, `http.om`, then `agora.om`, in that
order. A layout after a function is compiler error 29. A failed compilation
must not leave an executable that can be installed as a successful build.

## Backend account, state and network

The service uses the dedicated `aien-agora` system account and group. Check whether
they exist before installation. If absent, create a locked system account with
home `/var/lib/aien-agora`, no login shell and no additional groups. Do not reuse
an interactive human account. Install the verified executable root-owned at
`/opt/aien/bin/agora` with mode 0755. `StateDirectory=aien-agora` creates persistent
state with mode 0750; `UMask=0077` keeps new state private. Preserve this directory
on rollback and upgrades.

The committed unit defaults to loopback and permits only loopback IP traffic.
For a separate trusted proxy host, installation additionally requires:

- Root-owned `/etc/aien-agora/runtime.env`, mode 0600, containing
  `AGORA_LISTEN=<the approved private backend address>`.
- A root-owned systemd drop-in that clears `IPAddressAllow=` and then allows
  only `127.0.0.1/32` and the exact approved proxy address with its host prefix.
  Retain `IPAddressDeny=any`. Never substitute a whole private network.
- Inspection of the effective unit, drop-ins and ancestor slices. Allow lists
  accumulate; an unrelated broad allowance is not overridden by a narrow one.

Keep host addresses in local configuration, not public source. Install only
`aien-agora.service`; do not install the drainer, federation puller or JSON edge.
Before public routing, prove that the actual service has ingress/egress cgroup
BPF filters attached, the trusted proxy can reach it, and a demonstrably
reachable disallowed source cannot. Unsupported filtering is a deployment
refusal, not permission to expose the listener. Check that there is no alternate
wildcard, IPv6 or socket-activated listener. Enable startup only after these
checks pass. This boundary trusts the proxy host and permitted local tools;
it is not authentication of individual processes on those hosts.

### Exact effective policy observations

Read machine properties by name, not their display order. `IPAddressAllow` and
`IPAddressDeny` are sets of prefixes: require exact equality with the approved
allow set and with `{::/0, 0.0.0.0/0}` respectively. Reordered prefixes and
whitespace do not change a policy; a missing deny prefix or an extra allowance
does. Never use a substring or a contains-only comparison.

The native `policy.om` comparator takes the observed allow and deny values plus
the expected proxy and loopback prefixes as runtime inputs. Keep actual host
addresses in the root-owned local configuration, not public fixture source.
The standalone `policy-check.om` CLI accepts `ALLOW DENY EXPECTED_PROXY
EXPECTED_LOCAL` and exits 0 for exact sets, 1 for refusal, or 90 for malformed
invocation. Its no-argument fixtures cover both display orders and negative
controls for missing deny prefixes and extra allowances.

Before a live retry, replay the retained real observer output offline and
record an explicit pass/fail table. Exercise each guarded field unchanged,
changed and removed. A pre-exec observation is pending, not proof that a
listening backend runs with those credentials; late journal metadata is not
an atomic substitute for the bracketed executable, credentials and listener
observation. An observer failure still stops activation and requires rollback.

Compose `om2`, `json.om`, `policy.om` and `observer-check.om` for the native
raw-output oracle. Its modes are `unit`, `state`, `exec`, `snapshot`, `ps`,
`journal` and `ancestors`. Pass the original captured output and the explicit
expected identities as separate arguments; fields must be shorter than 4095
bytes. The oracle exits 0 for its predicate passing, 1 for refusal, 2 for a
pending observation, or 90 for an invalid or oversized invocation. These are
not the startup policy CLI's exit codes. A journal predicate proves only that
historical message's provenance, never current socket readiness.

Keep ExecStart's displayed `${AGORA_LISTEN}` separate from the resolved
process command line. Do not compare displayed timestamps, queue sizes,
descriptor numbers, process-column padding or decimal leading zeroes as
identity facts. Continue to require the exact executable, credentials,
endpoint, PID, property names and prefix sets.

Collect ancestor properties with options before the end-of-options marker:
`systemctl show -p IPAddressAllow -p IPAddressDeny -- -.slice`, then the same
command for `system.slice`. Require both properties to be present and empty
in each selected unit's output. `observer ancestors RAW EXPECTED_RECORDS`
accepts one record or two blank-delimited records; it rejects missing,
duplicated, unknown or nonempty properties. A noisy query that accidentally
selects `-p.service` is not policy evidence.

## Nginx scopes and exact routes

Install `nginx-agora-http.conf` at HTTP scope, for example under
`/etc/nginx/conf.d/`. Its `limit_req_zone` directive is invalid inside a server.
Install `nginx-agora.conf` as a snippet included inside the existing website TLS
server. Create the root-owned local `/etc/nginx/snippets/agora-upstream.conf`
from `agora-upstream.conf.example`, replacing its loopback server with the exact
approved private backend address when the hosts differ.

Preserve the existing SNI router and loopback-only PROXY-protocol TLS listener.
Both limiting and forwarded `X-Real-IP` use `$proxy_protocol_addr`, never a
visitor-supplied HTTP header. Empty PROXY identity refuses all allowed API routes.
`X-Forwarded-For` is stripped. The non-PROXY TLS listener must therefore refuse
API calls. The configuration deliberately refuses HEAD, rather than relying on
`limit_except GET`, which also permits HEAD.

Only the three registration routes proxy. The prefix fallback returns 404
locally, including post, import, note, peer and stream requests. Request bodies
are capped and buffered at 16k. API access logging, API error/debug logging and
proxy caching are off; responses carry no-store and nosniff. The API-specific
`error_log /dev/null crit` suppresses API body debug logs. Header processing
precedes location selection: require no effective server/connection debug logging,
including `debug_connection` overrides, before activation. Inspect the complete
configuration and body-temporary-file settings; no recovery keys belong in receipts.

Back up the affected configuration with root-only permissions. Validate the
complete candidate configuration with `nginx -t` before reloading. Do not restart
nginx or touch unrelated model routes. If any prerequisite fails, leave routing
disabled rather than broadening the allow list or trusting arbitrary headers.

### Verified configuration transfer

Transfer each candidate, including the existing website server file, through a
root-only regular temporary file on the destination host. Do not use
`install /dev/stdin DEST`: the hub's uutils coreutils 0.8.0 installer refuses that
source when `DEST` already exists, although installing a new destination works.
Installing from a regular file works for both cases.

For each already reviewed candidate, record its expected SHA256 before transfer.
On the destination host, create a root-owned temporary file using
`sudo mktemp /root/aien-agora-config.XXXXXX`; require mode 0600. Transfer the
candidate bytes into that file with `sudo tee "$tmp" >/dev/null`, not `install`.
Require `sudo sha256sum "$tmp"` to equal the expected digest before installation.
After backing up the destination with root-only permissions (or recording that
it was absent), run:

```sh
sudo install -o root -g root -m 0644 "$tmp" "$dest"
actual=$(sudo sha256sum "$dest")
actual=${actual%% *}
test "$actual" = "$expected" || exit 1
sudo rm -- "$tmp"
```

A transfer or installed-file digest mismatch is FAIL: stop and roll back, never
reload an unverified candidate. Remove the root-only temporary file on failure
as well. Check every installed file, not only the new snippet files. This
procedure changes the transfer mechanism, not the approved configuration,
network boundary, reload barrier or deployment acceptance gates.

### Backend startup identity barrier

`Type=simple` reports a successful start before the child has necessarily
changed credentials or completed `exec`. An immediate root-owned pre-exec
snapshot is not evidence that the listening backend runs as root.

After start, capture a nonzero `MainPID`. Poll for at most 10 seconds against
one absolute monotonic deadline. For the approved Spark account, require
`/proc/PID/exe` to resolve to `/opt/aien/bin/agora`, effective UID/GID 976/964,
and root `ss -ltnp` to show that **that same PID** owns the 18132 listener at
the approved bind address. Record the actual observations. Require the unit's
`MainPID` to remain the captured PID. Never accept command completion,
a matching process name, a journal line or a listener owned by another PID.

Capture executable/credentials before and after the listener observation;
a changing snapshot is pending, not readiness. Root pre-exec snapshots
without a listener are pending within the deadline, as are the expected
post-exec credentials before bind. A completed executable with wrong
credentials, an unexpected stable identity, process/read failure or timeout
is FAIL and triggers rollback. Do not extend the deadline, rerun start,
change the service identity or activate routing to make this check pass.

A recognized `/usr/lib/systemd/systemd-executor` to exact expected executable
transition may coincide with a single valid ownerless LISTEN row. With stable
MainPID, permitted pre-exec credentials, exact final credentials and time
remaining, that narrow case is PENDING only. Reacquire the entire bracket.
A stable ownerless listener, an explicit different owner, wrong endpoint,
malformed or multiple rows, changed MainPID or deadline expiry still refuses.
Only a fresh stable bracket with explicit same-PID ownership can become READY.

The native `om/agora/startup-check.om` unit is the startup policy authority.
Its CLI takes `deadline_ns executable_match effective_uid effective_gid
same_pid_listener` and exits 0 for pending, 1 for ready, 2 for refusal, or 90
for malformed syntax. Checked-integer overflow also fails closed. The observer
must interpret those policy statuses, not treat exit 0 as readiness. It must
retain the same deadline throughout polling and reject any late result.

With no arguments, the native unit replays synthetic credential snapshots
against the real monotonic clock: pre-exec root stays pending, the expected
executable/credentials/listener become ready, and a root snapshot that
persists through the full deadline is refused. This is a policy trace
fixture, not a claim that a privileged fixture process was launched.
The conductor requires preexec-root-fail, deadline-off and identity-off
mutants to fail with exits 40, 48 and 44 respectively. Actual production
`/proc` and listener ownership remain mandatory independent observations.

### Reload adoption barrier

`nginx -t`, `nginx -T`, and a successful reload command do not prove that the
running master has adopted the candidate. Reload sends a signal asynchronously.
A fresh TLS connection can still reach an old worker and fall through to Pages.
Do not change the location hierarchy or add a fixed sleep to mask that race.

Before reload, record the actual master PID, executable, command line and every
worker PID **with its process title**, using the equivalent of
`ps -C nginx -o pid=,ppid=,args=`. Separate active workers from workers already
reported as `worker process is shutting down`. Record the latter and verify
their lack of listening sockets with root `ss -ltnp`; an existing draining
process need not exit. Confirm the service uses the executable/configuration
that passed `nginx -t`; account for any alternate `-c` or `-p`.

After reload, use a bounded 30-second observation loop. Require at least one
new active worker PID. Every worker that was **active at capture** must have
either exited, or entered the shutting down state **and** no longer own any
listening socket. Observe process titles and root `ss -ltnp` together; a new
worker alone is insufficient, and a draining title alone is insufficient.
Record initial active/draining sets, new workers and listener ownership.
Pre-existing draining workers without listeners cannot accept a fresh request
and do not block adoption, even when an earlier connection keeps them alive.
Do not kill or signal a production worker to satisfy this barrier. Signal
command completion alone is not readiness. A timeout is FAIL and requires
rollback, not a longer deadline.

Only after worker handover, send one non-mutating GET challenge through the
actual public TLS/SNI path. Require HTTP 200, `Cache-Control: no-store`, and
exactly one independently observed backend request. Configure that observation
before the request without logging bodies, keys or visitor identities. No
temporary firewall rules are authorized by this guide; use an explicitly
approved observation mechanism. A missing observation or unexpected response
is FAIL. Only then begin the remaining identity, route, privacy and browser
gates. Never retry a failed acceptance request as though it had passed.

The native `om/agora/reload-check.om` oracle holds a real response open across
an initial reload, proving that a pre-existing draining worker can stay alive
without a listener. It checks missing reload, an old active worker still owning
a listener, and a post-barrier challenge with exact upstream count. A bounded
predicate fixture supplies a draining classification for the proven owned
listener and must still refuse it, independently exercising the listener guard.
It launches only isolated loopback nginx workers through Omega's process
facilities, observes Linux `/proc`, uses monotonic deadlines and performs its
own HTTP request. Its socket inode checks stream both kernel TCP tables through
EOF in a small buffer, retaining split token/row state and failing closed on
read errors, incomplete final rows or exhaustion of the bounded read loop.
Its one-worker and explicit 128-descriptor fixture limits are not production
worker enumeration limits. Production uses the full root `ss -ltnp` observation.
Run `OMEGA_OMC=/path/to/pinned/omc NGINX_BIN=/path/to/nginx sh om/agora/check.sh`;
the existing conductor compiles the Omega unit and requires barrier-off,
active-listener-off, draining-worker-refused and listener-check-off mutants
to fail with exits 11, 14, 23 and 30 respectively. Current source/binary pins
and observations are in `reload-draining.evidence.json`; the earlier
`reload.evidence.json` remains the historical stricter-barrier receipt.
Local oracle nginx version: 1.24.0; it is an unchanged verification oracle,
not an implementation substituted for Omega. When
diagnosing a host-specific failure, also replay the complete `nginx -T`
configuration in isolation, including its SNI/PROXY topology and sibling
locations; replace only host paths, ports, certificates and external upstreams
with local fixtures. Do not publish private captured configuration.


## Full isolated activation rehearsal

Before another live retry, execute the file-backed activation observers end to
end with `scripts/test-agora-activation.py` as root. Supply a new private
`--root`, the approved `--backend`, oracle `--nginx`, compiled native
`--native` observer and `--startup` policy, and the exact published `--page`
and `--client`. Do not substitute a mock API or replace the browser client.
The host needs `ip`, `tcpdump`, `bpftool`, systemd and the existing dedicated
account. The verification tools are not production dependencies.

The conductor creates a uniquely named network namespace, isolated systemd
backend and stub nginx. Its executables live in its own `/opt/aien-agora-oracle-*`
directory because Spark mounts `/run` noexec. Units and temporary boot links
are runtime-only; no production firewall rule, unit, hub file or state changes.
Fixed loopback tunnels carry raw TLS through the fixture SNI router and its
PROXY listener. The backend socket is created inside its network namespace.
The exact two-prefix policy is tested after a separately labelled third-source
reachability control. Passive SYN and SYN-ACK observations prove allowed and
denied paths without adding firewall rules.

Use the root-only `control.sock` with `--control PATH --action pre-browser`.
Then run `runBrowser` from `scripts/agora_activation_browser.mjs` against the
returned `browser_url` in a new, CPU-only browser profile. Resolve only
`www.aienos.com` to the returned loopback address, refuse other domains and
permit the fixture certificate only in that owned browser. Recovery callbacks
must pass secrets through stdin to the root-only `save-key` control; never put
them in arguments, screenshots, HAR, console output or receipts. The
`restart-backend` and `read-profile` controls operate only on the fixture.

Pass the browser verdict to `after-browser`, which rejects any missing or
failed prerequisite. It exercises real TLS quota exhaustion with forged
identity headers, checks effective access/error/temp sinks directly and
enables only the isolated runtime unit after all gates pass. Finally `finish`
injects an acceptance failure and verifies rollback: API closed, ordinary
site healthy, unit stopped and disabled, profiles unchanged. Private recovery
files and a root-only retained-state copy remain beneath the scratch root.
The receipt is explicitly `ISOLATED_NAMESPACE_ONLY_NOT_DEPLOYED`.

The conductor treats SIGHUP, SIGINT and SIGTERM as failed acceptance: it records
FAIL, rolls back and cleans up its own units and namespace while retaining
private state. A killed process cannot execute cleanup. If recovering an older
interrupted run, first prove the namespace, runtime units and paths belong to
that fixture, preserve their files privately, and stop only those resources.
Never label an interrupted run PASS or loosen directory permissions to read it;
use the operator's existing permitted root read path.

Privacy includes a negative control: place a public sentinel in an effective
fixture temporary-body sink, require the actual scanner to refuse it, and
remove only that control file. This demonstrates scanner sensitivity, not the
absence of transient production body spills or historical logging causality.
The privacy pass also scans current-invocation journals for the backend and
every fixture routing unit. Raw journal bodies are private, including the
startup warning observation; receipts retain only the scanner verdict and
non-secret unit names. Empty control responses fail without an acceptance
verdict rather than being treated as success.

Run `python3 -m py_compile` and `pyflakes` on all three Python observer files,
and `node --check scripts/agora_activation_browser.mjs`. Record the complete
rehearsal receipt and static-check output on the source PR before any live
retry. A fixture failure is not permission to continue live.

## Required deployment checks

Record observed outputs, not just the commands:

- Actual service account, executable digest/ownership, state permissions,
  effective IP policy, attached filters and the sole listening address.
- Allowed proxy request and denied request from a source whose path to the
  same listener has been independently established.
- Real TLS/SNI requests with forged `X-Real-IP` and `X-Forwarded-For`; they must
  retain the same client identity and quota bucket.
- Non-PROXY listener refusal; wrong methods, malformed profile IDs, trailing
  segments and oversized bodies. Blocked routes must create no upstream traffic.
- No-store/nosniff and absence of API access/body/key logs on success and failure.
- The published page and JavaScript match the approved artifacts. In a real
  browser, missing consent sends no registration request, success requires
  acknowledgement plus matching profile readback, and an uncertain write does
  not repeat automatically. Use explicitly public synthetic acceptance data,
  clearly identified as verification rather than a network participant.
- Successful profile readback after a controlled backend restart, preserving
  persistent state. Protect the returned recovery key; never publish it.
- Run `NGINX_BIN=/path/to/nginx python3 scripts/test-agora-deploy.py` locally.
  This isolated oracle starts loopback nginx/upstream listeners and checks
  route/method refusal, upstream counts, spoofing, rate buckets, accepted body
  fidelity, access-log exclusion and no retained body files. It requires a
  debug-capable nginx with an active control. Synthetic request-body sentinels
  were absent from its observed debug log, but suppression causality is not
  established: the override-removal mutant survived. Transient body spill,
  secret-bearing response logging, early header logging, live TLS/SNI identity
  and actual systemd BPF enforcement require separate live inspection/evidence.

The 2026-10-28 activation was rolled back after an immediate post-reload public
challenge returned the Pages 404. Replaying the complete captured hub
configuration returned 200 through both the PROXY listener and the SNI router.
Reloading from the preceding configuration reproduced an immediate old-worker
Pages 404 followed by 200 after handover. This establishes the reload race
mechanism, not a retrospective proof of the production cause: historical
worker lifecycle and per-request upstream counts were not captured. No
registration location change follows from that observation.


Ownership and API consent remain self-attested. Browser consent does not become
a cryptographic ownership proof. Browser-local recovery keys are accessible to
scripts executing on the same origin, including third-party scripts permitted
by the site's separate analytics consent. Signup must not trigger analytics or
forward form fields or keys to it.

## Rollback

Remove the registration snippet include and its HTTP-scope configuration,
validate nginx, and reload to close public routing. Stop/disable only the new
registration unit if necessary. Preserve private state, recovery material and
backups. Never delete accepted profiles as part of rollback. Restore unrelated
configuration only from its exact pre-change backup, and recheck the normal
website response. No rollback command should affect model services.

## Remaining boundary

Per-client JSON-edge quotas remain unimplemented at this compiler pin. The
registration flow uses the text API directly and does not activate that edge.
A transport peer-name primitive alone would not authenticate original client
headers behind a proxy. Broader Agora deployment requires its own review and
authorization.
