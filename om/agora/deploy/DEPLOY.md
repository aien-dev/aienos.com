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

### Firewall preflight

The tcp/18132 accept rule is not yet approved or applied. Run this preflight
before starting the unit. Do not add the rule. On the Spark, nftables table
`inet atlas_ingress` handles input before the unit's systemd IP filter. Its
input chain policy is drop, and port 18132 is not accepted there. Adding the
rule needs Drake's explicit approval. This preflight does not insert it, does
not authorize a temporary rule, and does not write `/etc/nftables.conf`.

Back up only table `inet atlas_ingress`. The file is one atomic restore: the
line `table inet atlas_ingress`, then `delete table inet atlas_ingress`, then
the saved table from `nft list table inet atlas_ingress`. Record the path.
The file is root-owned, mode 0600.

```sh
backup=$(sudo mktemp /root/aien-agora-nft.XXXXXX)
sudo sh -c 'printf "%s\n" "table inet atlas_ingress" "delete table inet atlas_ingress" > "$1"' sh "$backup"
sudo nft list table inet atlas_ingress | sudo tee -a "$backup" >/dev/null
sudo chown root:root "$backup"
sudo chmod 0600 "$backup"
sudo stat -c '%u %g %a %F %s %n' "$backup"
sudo awk 'NR==1 && $0!="table inet atlas_ingress" { exit 2 }
          NR==2 && $0!="delete table inet atlas_ingress" { exit 2 }
          NR==3 && $0!="table inet atlas_ingress {" { exit 2 }
          END { if (NR<3) exit 2 }' "$backup" || exit 1
```

PASS: uid 0, gid 0, mode 600, type `regular file`, size greater than 0, and
the three opening lines are `table inet atlas_ingress`,
`delete table inet atlas_ingress`, and `table inet atlas_ingress {`.
FAIL: any other owner, group, mode, type, or an empty or short file. Stop.
Do not start the unit.

List the input chain. `nft` must exit 0. The first printed number is the
input chain header: the line must start with optional whitespace, then
`type filter hook input`, and end with `policy drop;`. A comment that only
contains `policy drop;` does not match. The second number is how many lines
contain `dport 18132`. The third is how many lines contain the exact rule.
A trailing `# handle N` may follow the rule on the listed line.

```sh
list=$(sudo nft -a list chain inet atlas_ingress input) || exit 1
printf '%s\n' "$list"
printf '%s\n' "$list" | grep -E -c '^[[:space:]]*type filter hook input .*policy drop;[[:space:]]*$' || true
rule='iifname "tailscale0" ip saddr 100.64.0.3 tcp dport 18132 accept comment "aien-agora, hub nginx only"'
printf '%s\n' "$list" | grep -F -c 'dport 18132' || true
printf '%s\n' "$list" | grep -F -c -- "$rule" || true
```

The expression is the listed form of the 18110 rule in
`scripts/allen/README.md`: table `inet atlas_ingress`, chain `input`,
`iifname "tailscale0"`, source `100.64.0.3`, then `tcp dport` and `accept`.
That README records the 18110 rule with shell quoting `iifname '"tailscale0"'`,
which `nft` lists as `iifname "tailscale0"`. The 18110 line has no comment.
The 18132 line is that same listed form with this port and comment. The 18110
rule lives in `/etc/nftables.conf`.

PASS, once Drake has approved and the rule is applied: the header count is 1,
exactly one line contains `dport 18132`, and the exact rule count is 1.
A `dport 18132` count of 0 and an exact count of 0, with the header count at
1, is the current host. The rule is not yet approved or applied. That is a
failed preflight for starting the listener on `100.64.0.2:18132`. Leave the
chain unchanged.
FAIL: `nft` does not exit 0, the chain is missing, the header count is not 1,
the `dport 18132` count is greater than 1, the exact count is greater than 1,
or the `dport 18132` count is 1 while the exact count is not 1. A broader
`tcp dport 18132 accept` with no source address is that last failure. Stop.
Do not start the unit for hub ingress while either rule count is 0. A drop in
this chain never reaches the systemd IP filter, so unit IP accounting stays 0.

### Persist the rule in nftables.conf

The tcp/18132 accept rule is not yet approved or applied. Do not run this
step before Drake's explicit approval. It is the file change for after that
approval, matching the 18110 rule's home in `/etc/nftables.conf`
(`scripts/allen/README.md`). `nft -c -f` checks the file and does not load
it. This guide does not apply the rule.

Back up `/etc/nftables.conf` to a `.bak` copy, insert the exact rule once
after the `tcp dport 18110 accept` line, and validate the installed file.
The insert copies every other line unchanged, including the output chain.
Refuse the edit when any `dport 18132` line is already present, or when the
18110 line is missing.

```sh
if sudo grep -F -q 'dport 18132' /etc/nftables.conf; then
  exit 1
fi
stamp=$(date -u +%Y%m%dT%H%M%SZ)
confbak="/etc/nftables.conf.bak-aien-agora-$stamp"
sudo cp -a -- /etc/nftables.conf "$confbak"
rule='iifname "tailscale0" ip saddr 100.64.0.3 tcp dport 18132 accept comment "aien-agora, hub nginx only"'
tmp=$(sudo mktemp /root/aien-agora-nftables.XXXXXX)
sudo awk -v rule="$rule" '
  { print }
  index($0, "tcp dport 18110 accept") && !seen { print "\t\t" rule; seen=1 }
  END { if (!seen) exit 2 }
' /etc/nftables.conf | sudo tee "$tmp" >/dev/null
sudo test "$(sudo grep -F -c -- "$rule" "$tmp")" = 1 || exit 1
sudo test "$(sudo grep -F -c 'dport 18132' "$tmp")" = 1 || exit 1
sudo test "$(sudo grep -F -c 'tcp dport 18110 accept' "$tmp")" -ge 1 || exit 1
sudo nft -c -f "$tmp" || exit 1
mode=$(sudo stat -c '%a' /etc/nftables.conf)
sudo install -o root -g root -m "$mode" "$tmp" /etc/nftables.conf
sudo nft -c -f /etc/nftables.conf || exit 1
sudo rm -- "$tmp"
```

PASS: `$confbak` exists, `nft -c -f /etc/nftables.conf` exits 0, the file
contains the exact rule once, and it still contains `tcp dport 18110 accept`.
FAIL: any test exits non-zero. Leave `/etc/nftables.conf` as it was, using
`$confbak` if the install already happened, and stop.

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

### Hub path gate

Run this only after the startup identity barrier shows the same PID listening
on `100.64.0.2:18132`, and only when the firewall preflight found the exact
rule once. That rule is not yet approved or applied, so this gate cannot pass
on the current host. Do not add the rule to make it pass. No temporary
firewall rule is authorized.

A SYN from the hub to `100.64.0.2:18132` must be answered by a SYN-ACK. A
denied source must still get none. Record unit IP accounting immediately
before and after each probe. One probe per source. Do not retry a failed
probe as a pass.

On the Spark, start the capture first and leave it running until the probe
finishes. tcpdump exit 0 or 124 ends a complete capture. Any other exit is
FAIL. Read the counters before the probe:

```sh
systemctl show -p IPIngressPackets -p IPIngressBytes -p IPEgressPackets -p IPEgressBytes aien-agora.service
sudo timeout 20 tcpdump -ni tailscale0 -nn -tt -S -l \
  'tcp port 18132 and host 100.64.0.3 and (tcp[tcpflags] & tcp-syn != 0)'
```

On the hub, whose tailnet address is `100.64.0.3`:

```sh
curl --silent --show-error --max-time 5 --interface 100.64.0.3 \
  --output /dev/null --write-out '%{http_code}\n' \
  http://100.64.0.2:18132/agora/api/challenge
```

After the hub probe, read the counters again:

```sh
systemctl show -p IPIngressPackets -p IPIngressBytes -p IPEgressPackets -p IPEgressBytes aien-agora.service
```

tcpdump prints `Flags [S]` for a SYN and `Flags [S.]` for a SYN-ACK. Extra
flag letters may appear beside those. A SYN-ACK is a packet whose flags
contain both `S` and `.`. A SYN counted below must contain `S` and must not
contain `.`.

PASS: the capture has at least one SYN from `100.64.0.3` to
`100.64.0.2.18132`, and at least one reply from `100.64.0.2.18132` to
`100.64.0.3` that is a SYN-ACK. The after reading shows `IPIngressPackets`
greater than the before reading. The firewall verdict is the SYN-ACK. Record
the HTTP code beside it.
FAIL: a hub SYN and no SYN-ACK to `100.64.0.3`. FAIL: the after reading shows
`IPIngressPackets` unchanged after a hub SYN. The 2026-10-30 capture was 5
SYNs, 0 SYN-ACKs, and unit IP accounting at 0. FAIL: no hub SYN in the
capture. The probe did not arrive, so the gate did not run.

Denied source. Set `DENIED` to an address other than `100.64.0.3` and other
than `100.64.0.2` that can already place a packet on `tailscale0` toward
`100.64.0.2`. Do not add a firewall rule, a route, or a systemd address
allowance to create that path. Start a new capture on the Spark, then run
one curl on the denied host.

Read the counters before this probe:

```sh
systemctl show -p IPIngressPackets -p IPIngressBytes -p IPEgressPackets -p IPEgressBytes aien-agora.service
sudo timeout 20 tcpdump -ni tailscale0 -nn -tt -S -l \
  "tcp port 18132 and host ${DENIED} and (tcp[tcpflags] & tcp-syn != 0)"
```

```sh
curl --silent --show-error --max-time 5 --interface "$DENIED" \
  --output /dev/null --write-out '%{http_code}\n' \
  http://100.64.0.2:18132/agora/api/challenge
```

After the denied probe, read the counters again:

```sh
systemctl show -p IPIngressPackets -p IPIngressBytes -p IPEgressPackets -p IPEgressBytes aien-agora.service
```

PASS: at least one SYN from `DENIED` to `100.64.0.2.18132`, and no SYN-ACK
from `100.64.0.2.18132` to `DENIED`. The deciding test is that the denied
source gets no SYN-ACK. Other traffic can move `IPIngressPackets`, so the
after reading is a record, not the verdict.
FAIL: any SYN-ACK to `DENIED`. FAIL: no SYN from `DENIED`. Denial was not
observed.
A failed gate stops activation. It does not approve a firewall change.

### Firewall rollback

Deleting the live rule by its handle is the normal path. List the chain:

```sh
sudo nft -a list chain inet atlas_ingress input
```

Find the one line that contains this exact expression:

```
iifname "tailscale0" ip saddr 100.64.0.3 tcp dport 18132 accept comment "aien-agora, hub nginx only"
```

Read `N` from the `# handle N` at the end of that line. Do not use the handle
on the 18110 line (`tcp dport 18110 accept`). If the expression occurs zero
times, there is nothing to delete in the live chain. If it occurs more than
once, do not pick a handle. Use the table file below instead.

```sh
sudo nft delete rule inet atlas_ingress input handle N
sudo nft -a list chain inet atlas_ingress input
```

PASS: the 18132 expression is gone, the input chain header still ends with
`policy drop;`, and this 18110 line is still present:
`iifname "tailscale0" ip saddr 100.64.0.3 tcp dport 18110 accept`.
FAIL: the 18132 expression remains, or the 18110 line is missing. Stop and
use the table file below.

Also remove the same rule from `/etc/nftables.conf`. The 18110 rule lives in
that file, and a reboot or an nftables reload would load a 18132 line left
behind. The rule is not yet approved or applied. If the exact line is already
absent, record that and leave the file in place.

```sh
rule='iifname "tailscale0" ip saddr 100.64.0.3 tcp dport 18132 accept comment "aien-agora, hub nginx only"'
if sudo grep -F -q -- "$rule" /etc/nftables.conf; then
  stamp=$(date -u +%Y%m%dT%H%M%SZ)
  confbak="/etc/nftables.conf.bak-aien-agora-$stamp"
  sudo cp -a -- /etc/nftables.conf "$confbak"
  tmp=$(sudo mktemp /root/aien-agora-nftables.XXXXXX)
  sudo grep -F -v -- "$rule" /etc/nftables.conf | sudo tee "$tmp" >/dev/null
  sudo nft -c -f "$tmp" || exit 1
  mode=$(sudo stat -c '%a' /etc/nftables.conf)
  sudo install -o root -g root -m "$mode" "$tmp" /etc/nftables.conf
  sudo nft -c -f /etc/nftables.conf || exit 1
  sudo rm -- "$tmp"
fi
sudo grep -F -c -- "$rule" /etc/nftables.conf || true
sudo grep -F -n 'tcp dport 18110 accept' /etc/nftables.conf
```

PASS: the exact rule count in the file is 0, and `tcp dport 18110 accept` is
still present.
FAIL: the 18110 line is missing, or `nft -c -f` fails. Copy `$confbak` back
over `/etc/nftables.conf` when that backup was taken, and stop.

The other path is the preflight file, and only when the handle cannot be
used. `$backup` is that root-owned mode 0600 file. It saves and restores only
`inet atlas_ingress`: `table inet atlas_ingress`, then
`delete table inet atlas_ingress`, then the saved table. Check it with
`nft -c -f` before `nft -f`. One `nft -f` is one transaction. This keeps the
relay mailer's output chain and the 18110 rule. `scripts/allen/relay/start`
describes that output chain: it lets the allen-relay user reach only DNS on
loopback and TCP 465. No other table is in the file, so those tables stay as
they are. A preflight file taken while the 18132 rule is absent restores a
table without it.

```sh
sudo test "$(sudo stat -c '%u %g %a' "$backup")" = '0 0 600' || exit 1
sudo nft -c -f "$backup" || exit 1
sudo nft -f "$backup"
sudo nft -a list chain inet atlas_ingress input
```

If the mode test or `nft -c -f` fails, do not load the file. After the load,
the input chain must match the saved table, including the 18110 accept rule,
and the output chain must still be the saved one. Then remove the 18132 line
from `/etc/nftables.conf` with the commands above, so a reboot cannot bring
it back. Leave the 18110 rule in place. It is documented in
`scripts/allen/README.md` and is not part of this registration unit.

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

`activation-rehearsal.evidence.json` records the completed 2026-10-28 isolated
sequence at source `9caa5598159a8589f8dfd0c4c36b5b568b84786e`: all 17
conductor gates and seven real-browser gates passed, including durable readback,
lost-acknowledgement refusal and injected rollback. The earlier interrupted
rehearsals remain failed or incomplete. This receipt is not a production retry,
deployment authorization, or evidence that production routing is enabled.

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

The tcp/18132 accept rule is not yet approved or applied. The normal live
rollback is delete-by-handle, under [Firewall rollback](#firewall-rollback).
Do not delete the 18110 accept rule in `inet atlas_ingress` input. That rule
is documented in `scripts/allen/README.md` and lives in `/etc/nftables.conf`.

Also remove the exact 18132 line from `/etc/nftables.conf`, or a reboot or an
nftables reload brings it back. If the line is already absent, record that
and leave the file in place.

```sh
rule='iifname "tailscale0" ip saddr 100.64.0.3 tcp dport 18132 accept comment "aien-agora, hub nginx only"'
if sudo grep -F -q -- "$rule" /etc/nftables.conf; then
  stamp=$(date -u +%Y%m%dT%H%M%SZ)
  confbak="/etc/nftables.conf.bak-aien-agora-$stamp"
  sudo cp -a -- /etc/nftables.conf "$confbak"
  tmp=$(sudo mktemp /root/aien-agora-nftables.XXXXXX)
  sudo grep -F -v -- "$rule" /etc/nftables.conf | sudo tee "$tmp" >/dev/null
  sudo nft -c -f "$tmp" || exit 1
  mode=$(sudo stat -c '%a' /etc/nftables.conf)
  sudo install -o root -g root -m "$mode" "$tmp" /etc/nftables.conf
  sudo nft -c -f /etc/nftables.conf || exit 1
  sudo rm -- "$tmp"
fi
sudo grep -F -c -- "$rule" /etc/nftables.conf || true
sudo grep -F -n 'tcp dport 18110 accept' /etc/nftables.conf
```

PASS: the exact rule count in the file is 0, and `tcp dport 18110 accept` is
still present.
FAIL: the 18110 line is missing, or `nft -c -f` fails. Copy `$confbak` back
over `/etc/nftables.conf` when that backup was taken, and stop.

## History

### 2026-10-28 rollback record

The 2026-10-28 activation was rolled back after an immediate post-reload public
challenge returned the Pages 404. Replaying the complete captured hub
configuration returned 200 through both the PROXY listener and the SNI router.
Reloading from the preceding configuration reproduced an immediate old-worker
Pages 404 followed by 200 after handover. This establishes the reload race
mechanism, not a retrospective proof of the production cause: historical
worker lifecycle and per-request upstream counts were not captured. No
registration location change follows from that observation.

### 2026-10-30

The 2026-10-30 hello-door activation was rolled back at 09:00 CT. The Spark
nftables table `inet atlas_ingress` has its input chain on policy drop, and
port 18132 is not accepted. Hub SYNs were dropped before they reached the
systemd IP filter: 5 SYNs, 0 SYN-ACKs, and unit IP accounting at 0. The
accept rule for tcp/18132 is not yet approved or applied. Adding it needs
Drake's explicit approval. The earlier public-routing rollback is the
[2026-10-28 rollback record](#2026-10-28-rollback-record).

## Remaining boundary

Per-client JSON-edge quotas remain unimplemented at this compiler pin. The
registration flow uses the text API directly and does not activate that edge.
A transport peer-name primitive alone would not authenticate original client
headers behind a proxy. Broader Agora deployment requires its own review and
authorization.
