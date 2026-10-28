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

### Reload adoption barrier

`nginx -t`, `nginx -T`, and a successful reload command do not prove that the
running master has adopted the candidate. Reload sends a signal asynchronously.
A fresh TLS connection can still reach an old worker and fall through to Pages.
Do not change the location hierarchy or add a fixed sleep to mask that race.

Before reload, record the actual master PID, executable, command line and its
worker PIDs. Confirm the service uses the same executable/configuration that
passed `nginx -t`; an alternate `-c` or `-p` must be accounted for. After reload,
use a bounded 30-second observation loop: require at least one new worker PID
and require every captured old worker PID to have exited. Record both sets.
Signal-command completion alone is not the barrier. A timeout is FAIL and
requires rollback, not an extension of the deadline. Long-lived old connections
can prevent this deliberately strict retirement condition from passing.

Only after worker handover, send one non-mutating GET challenge through the
actual public TLS/SNI path. Require HTTP 200, `Cache-Control: no-store`, and
exactly one independently observed backend request. Configure that observation
before the request without logging bodies, keys or visitor identities. No
temporary firewall rules are authorized by this guide; use an explicitly
approved observation mechanism. A missing observation or unexpected response
is FAIL. Only then begin the remaining identity, route, privacy and browser
gates. Never retry a failed acceptance request as though it had passed.

The local oracle includes a missing-reload negative control, actual old-worker
retirement, and a post-barrier challenge with exact upstream count. When
diagnosing a host-specific failure, also replay the complete `nginx -T`
configuration in isolation, including its SNI/PROXY topology and sibling
locations; replace only host paths, ports, certificates and external upstreams
with local fixtures. Do not publish private captured configuration.


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
