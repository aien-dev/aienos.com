# allen-chat: the public Allen gateway

`https://www.aienos.com/allen/` lets anyone talk to Allen, my AI, on the live
Qwen3.6-35B-A3B FP8 that runs on my DGX Spark, run small programs in their own
browser, find what I have built, propose an issue or a project, and introduce
themselves with consent. This directory is the server half: one Python process
on the Spark (`allen_chat.py` plus `allen_spool.py`), its systemd unit, the hub
nginx route and the tests. The page half is `om/allen-page/` (Omega-generated
markup and JS).

```
browser --TLS--> hub :443 SNI router --PROXY protocol--> hub nginx www.aienos.com (127.0.0.1:4446)
        /allen/api/... --tailnet--> 100.64.0.2:18110 allen-chat (system unit, User=drakestapleton)
        chat      --> 127.0.0.1:18091 (shared model server, streamed, keep-alive)
        omega     --> bwrap { pinned omc (native check, then IR) -> pinned irwasm } -> .wasm to the browser
        submit    --> spool/in/<id>.json          (omega-projects-pusher files it; see below)
        introduce --> contacts-drop/<hex>.json    (allen-contacts-ingest stores it; see below)
```

## What public Allen cannot do

Public Allen cannot deploy anything: not to this site, not to any repository,
not to the hub and not to the Spark. This is enforced by structure, not by the
prompt, and each point is tested with a negative control (results in the PR):

1. **No capability in the gateway.** It exposes exactly these routes:
   `POST chat`, `POST omega`, `POST submit`, `GET status/<id>`,
   `POST introduce`, `POST forget`. It has no tools, no function calling, no
   shell, no git, gh or ssh, and no credential of any kind. Model output is
   relayed to the browser as text; the server never parses it for actions.
   A prompt asking Allen to deploy, push, commit or run a shell command produces
   text and nothing else.
2. **Sandboxed process.** `allen-chat.service` is a system unit running as
   drakestapleton (the user manager on this Spark silently skips mount
   namespacing, so a --user unit would not enforce these):
   `NoNewPrivileges`, empty `CapabilityBoundingSet`, `PrivateUsers`,
   `ProtectProc=invisible`, `ProtectSystem=strict`, `ProtectHome=tmpfs` with
   read-only binds of only its code dir and the pusher's status dir and
   read-write binds of only the two drop directories, `PrivateTmp`,
   `PrivateDevices`, `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
   AF_NETLINK` (netlink only so bwrap can bring up the sandbox's own
   loopback), `IPAddressDeny=any` with `IPAddressAllow=127.0.0.1 100.64.0.2
   100.64.0.3`, `LockPersonality`, `MemoryDenyWriteExecute`,
   `SystemCallFilter=@system-service @mount unshare setns`. Measured: it cannot
   read `~/.ssh`, `~/.config/gh`, `~/.omp`, the vault, the contact store or the
   gb10 lock (also not through `/proc/<pid>/root`), cannot `sudo`, and cannot
   connect to github.com, the internet or the LAN. The Spark firewall admits
   port 18110 only from the hub's tailnet address.
3. **Compiling is not running.** The Omega path runs only the pinned compiler
   and translator, inside bwrap with no network, a read-only root containing
   only the tool, an 8 MiB tmpfs, no `/proc`, cleared environment, no
   capabilities, its own pid namespace, RLIMIT_CPU 4 s, RLIMIT_AS 512 MiB,
   RLIMIT_FSIZE 4 MiB and a 5 s wall clock. Nothing the compiler produces is ever
   executed on the server; the `.wasm` runs only in the visitor's browser,
   without imports. If bwrap is missing or cannot build a sandbox the gateway
   refuses to start (no bare omc). Nothing is persisted.
4. **Web apps** run only in the visitor's `<iframe sandbox="allow-scripts">`
   with a `default-src 'none'` CSP: opaque origin, no cookies, no storage, no
   network, no top navigation.
5. **Proposals, not publication.** Allen can draft an issue or a project. Only
   after the visitor confirms the exact text does the gateway drop it into a
   spool it can write but not act on. A separate unit,
   `omega-projects-pusher` (path-activated, the only holder of GitHub write
   access, scoped in code to `aien-dev/omega-projects`), re-validates it,
   removes emails and phone numbers, and files exactly one issue (labels
   `from-allen`, `area:<repo>`) or opens one pull request on a
   `submissions/<slug>-<id>` branch. It never pushes to main and never merges;
   the repository's ruleset requires my approval with no bypass. Only my merge
   publishes anything.
6. **Introductions are private and consented.** Nothing is stored unless the
   visitor (or agent) sends `consent: true` after the notice. The gateway drops
   the record for `allen-contacts-ingest`, which appends it to a store that is
   not in the gateway's mount namespace, so the gateway, and therefore a
   prompt-injected Allen, can never read one visitor's details back to another.
   Deletion: the delete link (`#forget=<id>.<token>`, the token is only ever
   stored as a hash) or by email; I read the store with `allen-contacts`.
7. **Upstream auth ready.** The gateway reads `ALLEN_UPSTREAM_TOKEN` from
   `/home/drakestapleton/.config/allen-chat/env` (empty today) and sends it as
   a bearer to 18091, for the sovereign-core #771 cutover.

## What protects the shared model

| Guard | Where | Default |
|---|---|---|
| Public requests in flight upstream | gateway `--cap` | 2 of the server's 8 batch slots |
| Waiting queue, position streamed to the visitor | gateway `--queue`, `--queue-wait` | 6 deep, 90 s |
| Back-off while the server is busy | gateway `--busy-at` (reads `/health` `in_flight`) | no new public start at >= 6 |
| Per-address rate | nginx `limit_req` (chat 6/min burst 4, omega 12/min burst 6, submit/introduce 6/min burst 3) and gateway token buckets (chat burst 4 then one per 20 s; omega burst 6 then one per 10 s; submit and introduce 3 per hour) | both |
| Global submissions | gateway (40 per UTC day) and pusher (40 issues, 20 PRs per day) | both |
| Turn size | gateway | 24 messages, 4000 chars each, 16000 total, 1024 output tokens |
| Timeouts | gateway | 60 s to first token, 20 s between tokens, 150 s per answer |
| Connections per address | nginx `limit_conn` | 4 |

The system prompt is sent byte-identically on every request so the server's
prefix stays the same between turns. The hub keys limits on the PROXY protocol
address only inside `/allen/api/`; `$remote_addr` stays the loopback for the rest
of the site, so the site's own access log is unchanged, and `/allen/api/` is not
logged at all.

## Privacy

No prompt, reply, source, address or path is written anywhere: the handler's
access log is disabled, nginx has `access_log off` on these routes, and the
rate-limit keys are salted BLAKE2 hashes held in memory. The journal gets one
aggregate counter line every ten minutes. Introductions are the only personal
data kept, only with consent, as described above.

## Dependencies and the Omega path

Omega has no HTTP, TLS or socket words yet, so this gateway is host code, like
`crates/hive-feed` and `hive-receive`. Recorded per the AIEN dependency rule:

| Dependency | Version on the Spark | What it does here | Omega replacement |
|---|---|---|---|
| CPython standard library (`http.server`, `http.client`, `subprocess`) | 3.12.3 | HTTP/1.1 server, upstream client, SSE relay, sandbox launcher | An Omega gateway once host words for sockets, HTTP and process spawn land; the open host-primitive proposal is aien-dev/omega-mv#788 (bounded TLS HTTP/JSON host primitives). This hop is plain HTTP over the tailnet, so no TLS is needed in Omega for it. |
| bubblewrap | 0.9.0 | Namespace sandbox for omc and irwasm | Kept as an isolation tool around the compiler; the compiler and translator themselves are seeded Omega. |
| nginx (hub) | 1.28.3 | TLS, PROXY protocol, rate limits, routing | Existing edge; unchanged dependency. |

The compiler (`omc`, sha256 `734f0a39...6b61`) and the IR-to-WebAssembly
translator (`irwasm`, sha256 `2d235a70...82e6`) are seeded Omega binaries
(`om/hive-page/`); the gateway refuses to start on any other bytes.

## Deploy (Spark)

```sh
D=/home/drakestapleton/.local/lib/aienos-allen-chat
S=/home/drakestapleton/.local/state/allen-chat
install -d -m 700 "$D" "$S/spool/in" "$S/spool/status" "$S/contacts-drop" ~/.config/allen-chat
install -m 644 scripts/allen/allen_chat.py scripts/allen/allen_spool.py scripts/allen/system-prompt.txt "$D"/
install -m 755 om/hive-page/omc om/hive-page/irwasm "$D"/
printf 'ALLEN_UPSTREAM_TOKEN=\n' > ~/.config/allen-chat/env && chmod 600 ~/.config/allen-chat/env
sudo install -m 644 scripts/allen/allen-chat.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now allen-chat.service
# firewall: admit the hub only (persisted in /etc/nftables.conf)
sudo nft add rule inet atlas_ingress input iifname '"tailscale0"' ip saddr 100.64.0.3 tcp dport 18110 accept
```

Rollback: `sudo systemctl disable --now allen-chat.service`, delete the
18110 rule (`sudo nft -a list chain inet atlas_ingress input`, then
`delete rule ... handle N`) and restore `/etc/nftables.conf` from its
`.bak-allenchat-*` copy. Nothing else on the Spark changes.

## Deploy (hub nginx)

Files: `nginx-allen-zones.conf` -> `/etc/nginx/conf.d/aienos-allen.conf`,
`nginx-allen-location.conf` -> `/etc/nginx/snippets/aienos-allen.conf`
(included in the www.aienos.com server), a second listener
`listen 127.0.0.1:4446 ssl proxy_protocol;` in that server, and in
`/etc/nginx/stream.d/sni-router.conf`: `proxy_protocol on;` on the :443 server,
`aienos_web` -> 4446, and two strip shims (`listen 127.0.0.1:4453
proxy_protocol; proxy_protocol off; proxy_pass 127.0.0.1:4443;` and the same
4454 -> 4444) so headscale and interplane still receive plain TLS from
127.0.0.1. Applied in two reloads (listeners first, then the switch). Backup
and rollback: `/root/allenchat-backup-<UTC>` holds `stream.d`, `sites-*`,
`snippets` and `conf.d` as they were; copy them back, `nginx -t`, reload.

## Tests

```sh
python3 scripts/allen/test_sandbox.py              # compile sandbox escape tests, each with a control
python3 scripts/allen/test_gateway.py --omc om/hive-page/omc --irwasm om/hive-page/irwasm \
    --system-prompt scripts/allen/system-prompt.txt [--skip-model]
python3 scripts/allen/probe.py                     # run inside the unit's properties (systemd-run) and outside
```

`test_gateway.py` starts throwaway loopback gateways (never the production
unit): input caps, Omega refusals (including the native pre-check), per-address
429 with a control address, first-token and mid-stream timeouts against a
stalling stub, health back-off, submit validation (traversal, oversize, binary,
external script, secret-shaped text; valid control; dedupe; status URL
filtering), the introduce consent gate and drop format, the output cap, and a
10-client load test that records the peak of concurrent streams, queue
positions and the shared server's `/health` throughout. `dev_proxy.py` serves a
built site and forwards `/allen/api/` to a local gateway for browser tests; it
is never deployed.
