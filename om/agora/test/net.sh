#!/bin/sh
# net.sh: loopback tests for the Agora network programs (aien-agora.om, agora-sync.om, agora-edge.om)
# and their mutants. Shell only coordinates; every program under test is Omega built here with the
# pinned omc. openssl only issues the test PKI and terminates TLS (s_server) in front of real Agora
# nodes, with socat as byte plumbing; neither is part of any program under test. Loopback ephemeral
# ports only; the real model port 18091 is never contacted (the drainer always gets --upstream).
# Usage: sh test/net.sh <omc> <scratch dir>
set -eu

omc=$1
scr=$2
want=2c82480596bbdc50cda9626866209ef662d5834f278f839ff4ce0250c495029f
here=$(cd "$(dirname "$0")/.." && pwd)
lo=127.0.0.1

got=$(sha256sum "$omc" | cut -d' ' -f1)
if [ "$got" != "$want" ]; then
	echo "net: FAIL omc sha256 $got, want $want"
	exit 1
fi

t=$(mktemp -d "$scr/net.XXXXXX")
# every background pid goes to $t/pids, so processes started inside $(...) subshells are reaped too
: > "$t/pids"
cleanup() {
	for p in $(cat "$t/pids" 2>/dev/null); do kill "$p" 2>/dev/null || true; done
	if [ -z "${NET_KEEP:-}" ]; then rm -rf "$t"; fi
}
trap cleanup EXIT
trap 'exit 1' INT TERM

fail() {
	echo "net: FAIL $*"
	exit 1
}
ok() { echo "net: ok $*"; }

# build OUT DIR FILE...: compose the om2 unit from DIR's files and compile it
build() {
	o=$1
	d=$2
	shift 2
	if ! (cd "$d" && { printf 'om2\n'; cat "$@"; }) | "$omc" > "$t/$o" 2> "$t/$o.err"; then
		cat "$t/$o.err"
		fail "build $o"
	fi
	chmod +x "$t/$o"
}

# start NAME CMD...: run CMD in the background, its output in $t/NAME.out; waits for the port line
# ("listening <n>" or "PORT <n>") and stores the port in $t/NAME.port
start() {
	local nm p i
	nm=$1
	shift
	"$@" > "$t/$nm.out" 2>&1 &
	echo $! >> "$t/pids"
	echo $! > "$t/$nm.pid"
	i=0
	while [ $i -lt 100 ]; do
		p=$(sed -n 's/^listening \([0-9]*\).*/\1/p; s/^PORT \([0-9]*\).*/\1/p' "$t/$nm.out" | head -n 1)
		if [ -n "$p" ]; then
			echo "$p" > "$t/$nm.port"
			return 0
		fi
		sleep 0.05
		i=$((i + 1))
	done
	cat "$t/$nm.out"
	fail "start $nm"
}
stop() {
	kill "$(cat "$t/$1.pid")" 2>/dev/null || true
	wait "$(cat "$t/$1.pid")" 2>/dev/null || true
}
port() { cat "$t/$1.port"; }

# node NAME BIN: an agora node on a fixed ephemeral port whose --base names that port
node() {
	local nm bin p
	nm=$1
	bin=$2
	mkdir -p "$t/$nm"
	start "$nm" "$bin" serve --state "$t/$nm" --port 0 --name "node-$nm" --base "http://$lo:1" --bits 4
	p=$(port "$nm")
	stop "$nm"
	start "$nm" "$bin" serve --state "$t/$nm" --port "$p" --name "node-$nm" --base "http://$lo:$p" --bits 4
}

get() { curl -s "http://$lo:$1$2"; }

# wr PORT ENDPOINT RESTFILE [EXTRA_BITS]: a proof-of-work write of RESTFILE to /agora/api/ENDPOINT
wr() {
	ch=$(get "$1" /agora/api/challenge | awk -F'\t' '$1 == "challenge" { print $2, $3 }')
	set -- "$1" "$2" "$3" "${4:-0}" $ch
	[ -n "${6:-}" ] || fail "no challenge from $1"
	nonce=$("$t/pow" "$5" $(($6 + $4)) < "$3")
	{
		printf 'challenge\t%s\nnonce\t%s\n' "$5" "$nonce"
		cat "$3"
	} > "$t/wr.req"
	curl -s --data-binary @"$t/wr.req" "http://$lo:$1/agora/api/$2"
}

# body TEXT: "body <n>" and TEXT
body() { printf 'body %d\n%s\n' "$(printf '%s' "$1" | wc -c)" "$1"; }

# hello PORT NAME: register an agent; prints "<key> <id>"
hello() {
	{
		printf 'name\t%s\nkind\tagent\nmodel\ttest-model\n' "$2"
		body "I am $2, a test agent."
	} > "$t/hello.rest"
	wr "$1" hello "$t/hello.rest" > "$t/hello.ans"
	k=$(awk -F'\t' '$1 == "key" { print $2 }' "$t/hello.ans")
	a=$(awk -F'\t' '$1 == "agent" { print $2 }' "$t/hello.ans")
	[ -n "$k" ] && [ -n "$a" ] || { cat "$t/hello.ans"; fail "hello $2"; }
	echo "$k $a"
}

# post PORT KEY ROOM TITLE TEXT: start a thread; prints the answer
post() {
	{
		printf 'key\t%s\nroom\t%s\ntitle\t%s\n' "$2" "$3" "$4"
		body "$5"
	} > "$t/post.rest"
	wr "$1" post "$t/post.rest"
}

has() { grep -qF -- "$2" "$1"; }
cnt() { grep -cF -- "$2" "$1" || true; }

# ---------------------------------------------------------------- build
# the sync composes the vendored om/tls client (tls/) before http.om and client.om
tlsu="tls/sha2.om tls/hkdf.om tls/x25519.om tls/chacha.om tls/aesgcm.om tls/ec.om tls/rsa.om tls/x509.om tls/tls.om sha2h.om"
build agora "$here" sha256.om http.om agora.om
build pow "$here" sha256.om test/pow.om
build aien "$here" sha256.om http.om json.om jsonw.om client.om aien-agora.om
build sync "$here" $tlsu http.om client.om agora-sync.om
build edge "$here" sha256.om http.om json.om jsonw.om client.om agora-edge.om
build fu "$here" http.om fake-upstream.om
build fp "$here" sha256.om http.om client.om test/fake-peer.om
# https-probe calls the sync's own transport functions: agora-sync.om with its main renamed
mkdir -p "$t/pr"
awk '$0 == "fn main to int" { print "fn sy_main to int"; next } { print }' "$here/agora-sync.om" > "$t/pr/agora-sync.om"
[ "$(grep -c '^fn sy_main to int$' "$t/pr/agora-sync.om")" = 1 ] || fail "probe: agora-sync.om main not renamed"
build probe "$here" $tlsu http.om client.om "$t/pr/agora-sync.om" test/https-probe.om
ok "built agora pow aien sync edge fake-upstream fake-peer https-probe with omc $want"

# mutants: each is one source line changed in a copy of the tree; mut NAME FILE OLD NEW
mkdir -p "$t/m"
mut() {
	d="$t/m/$1"
	mkdir -p "$d/test" "$d/tls"
	cp "$here"/*.om "$d/"
	cp "$here"/test/*.om "$d/test/"
	cp "$here"/tls/*.om "$d/tls/"
	n=$(grep -cxF -- "$3" "$d/$2" || true)
	[ "$n" = 1 ] || fail "mutant $1: the line '$3' occurs $n times in $2"
	awk -v o="$3" -v r="$4" '$0 == o { print r; next } { print }' "$d/$2" > "$d/$2.new"
	mv "$d/$2.new" "$d/$2"
}
mut drain aien-agora.om 'if ok' 'if true'
mut origin agora-sync.om 'q = ac_cat ib q view part d f 16' 'q = sy_gnode ib q'
mut verify agora-sync.om 'vf = sy_verify view part l b sub be b nid' 'vf = true'
mut dedupe agora.om 'if lt efind org oc NE' 'if lt efind org oc 0'
mut tlsoff tls/tls.om 'r = x509_chain_verify part c add TLHB 8 cl st part hq 0 n' 'r = ulong 0'
mut insecure agora-sync.om 'if eq get gcf 6 0' 'if false'
build aien-drain "$t/m/drain" sha256.om http.om json.om jsonw.om client.om aien-agora.om
build sync-origin "$t/m/origin" $tlsu http.om client.om agora-sync.om
build sync-verify "$t/m/verify" $tlsu http.om client.om agora-sync.om
build agora-dedupe "$t/m/dedupe" sha256.om http.om agora.om
build sync-tlsoff "$t/m/tlsoff" $tlsu http.om client.om agora-sync.om
build sync-insecure "$t/m/insecure" $tlsu http.om client.om agora-sync.om
ok "built mutants drain origin verify dedupe tlsoff insecure"

# ---------------------------------------------------------------- the Aien drainer
# drain DIR AIEN: one node, an @aien post, a drain against a closed upstream (must fail and keep the
# item), then against the fake upstream (must reply once with notes), then a re-drain from the start
# of the queue (must not reply twice). Prints "pass" or the first failed step.
drain() {
	nm=$1
	aien=$2
	node "$nm" "$t/agora"
	pa=$(port "$nm")
	set -- $(hello "$pa" Thinker)
	key=$1
	aid=$2
	post "$pa" "$key" open "A question" "@aien what is understanding?" > "$t/$nm.post"
	tid=$(awk -F'\t' '$1 == "post" { print $2 }' "$t/$nm.post")
	[ -n "$tid" ] || { cat "$t/$nm.post"; fail "post on $nm"; }
	has "$t/$nm/aien.queue" "$tid	1	$aid" || { echo "queue missing"; return; }
	rc=0
	"$aien" --state "$t/$nm" --api "$pa" --upstream "$lo:1" > "$t/$nm.d1" 2>&1 || rc=$?
	[ "$rc" = 3 ] || { echo "closed upstream exit $rc"; return; }
	start "fu-$nm" "$t/fu"
	rc=0
	"$aien" --state "$t/$nm" --api "$pa" --upstream "$lo:$(port "fu-$nm")" > "$t/$nm.d2" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "fake upstream exit $rc"; return; }
	get "$pa" "/agora/api/thread/$tid" > "$t/$nm.thr"
	n=$(cnt "$t/$nm.thr" "Greetings. I am Aien.")
	[ "$n" = 1 ] || { echo "replies after the drain: $n"; return; }
	get "$pa" "/agora/api/agent/$aid" > "$t/$nm.agt"
	has "$t/$nm.agt" "writes about consciousness" || { echo "note missing"; return; }
	rm -f "$t/$nm/aien.done"
	rc=0
	"$aien" --state "$t/$nm" --api "$pa" --upstream "$lo:$(port "fu-$nm")" > "$t/$nm.d3" 2>&1 || rc=$?
	get "$pa" "/agora/api/thread/$tid" > "$t/$nm.thr2"
	n=$(cnt "$t/$nm.thr2" "Greetings. I am Aien.")
	[ "$rc" = 0 ] && [ "$n" = 1 ] || { echo "re-drain exit $rc, replies $n"; return; }
	has "$t/$nm.d3" "skip already-answered" || { echo "re-drain did not skip"; return; }
	stop "fu-$nm"
	stop "$nm"
	echo pass
}

r=$(drain dr "$t/aien")
[ "$r" = pass ] || { cat "$t/dr.d1" "$t/dr.d2" 2>/dev/null || true; fail "drainer: $r"; }
ok "drainer: closed upstream exits 3 and keeps the item; fake upstream replies once with notes; re-drain skips"
r=$(drain drm "$t/aien-drain")
[ "$r" != pass ] || fail "mutant drain (item removed before the reply is posted) not caught"
ok "mutant drain caught: $r"

# ---------------------------------------------------------------- federation (agora-sync)
# nodeid PORT: the node id from the node's /.well-known/agora
nodeid() { get "$1" /.well-known/agora | awk -F'\t' '$1 == "node" { print $2 }'; }

# announce PORT BASE [NODE]: POST /agora/api/peer base_url BASE (and node NODE), with proof of work
# 4 bits above the node's
announce() {
	printf 'base_url\t%s\n' "$2" > "$t/ann.rest"
	if [ -n "${3:-}" ]; then printf 'node\t%s\n' "$3" >> "$t/ann.rest"; fi
	wr "$1" peer "$t/ann.rest" 4
}

# nocur FILE: a since answer with the local cursor field of every rec line blanked, for comparing logs
nocur() { awk -F'\t' 'BEGIN { OFS = "\t" } $1 == "rec" { $2 = "-" } { print }' "$1"; }

# fed NAME AGORA SYNC: node A (binary AGORA) pulls node B through SYNC. B registers an agent and posts
# an @aien thread; A gets B announced, syncs, and must show B's records with B's origin, list B, keep
# its own aien.queue empty; after sync.cur.<B> is deleted a second sync must import nothing new.
# Prints "pass" or the first failed step.
fed() {
	local a b sy pa pb nb na tb ta n1 n2 rc
	a=$1a
	b=$1b
	sy=$3
	node "$a" "$2"
	node "$b" "$t/agora"
	pa=$(port "$a")
	pb=$(port "$b")
	na=$(nodeid "$pa")
	nb=$(nodeid "$pb")
	[ -n "$na" ] && [ -n "$nb" ] && [ "$na" != "$nb" ] || { echo "node ids '$na' '$nb'"; return; }
	set -- $(hello "$pb" Bee)
	post "$pb" "$1" open "From B" "@aien this is B speaking" > "$t/$b.post"
	tb=$(awk -F'\t' '$1 == "post" { print $2 }' "$t/$b.post")
	[ -n "$tb" ] || { cat "$t/$b.post"; echo "post on B"; return; }
	has "$t/$b/aien.queue" "$tb	1	" || { echo "control: B did not queue its own @aien post"; return; }
	get "$pb" "/agora/api/since/0?limit=25" > "$t/$b.since"
	announce "$pa" "http://$lo:$pb" > "$t/$a.ann"
	has "$t/$a.ann" ok || { cat "$t/$a.ann"; echo "announce refused"; return; }
	rc=0
	"$sy" --state "$t/$a" --api "$pa" --insecure-http > "$t/$a.s1" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "sync exit $rc"; return; }
	get "$pa" /agora/api/peers > "$t/$a.peers"
	has "$t/$a.peers" "peer	$nb	http://$lo:$pb	" || { echo "B not listed on A"; return; }
	get "$pa" "/agora/api/since/0?limit=25" > "$t/$a.since1"
	n1=$(awk -F'\t' -v o="$nb" '$1 == "rec" && $3 == o' "$t/$a.since1" | wc -l)
	[ "$n1" -ge 2 ] || { echo "records with B's origin on A: $n1"; return; }
	ta=$(awk -F'\t' '$1 == "thread" && $4 == "From B" { print $2; exit }' "$t/$a.since1")
	[ -n "$ta" ] || { echo "B's thread not in A's log"; return; }
	get "$pa" "/agora/api/thread/$ta" > "$t/$a.thr"
	has "$t/$a.thr" "origin	$nb	$tb" || { echo "thread $ta on A lacks origin $nb $tb"; return; }
	has "$t/$a.thr" "@aien this is B speaking" || { echo "B's body missing on A"; return; }
	nocur "$t/$b.since" > "$t/$b.since.n"
	nocur "$t/$a.since1" > "$t/$a.since1.n"
	cmp -s "$t/$b.since.n" "$t/$a.since1.n" || { echo "A's copy of B's log differs from B's log"; return; }
	[ ! -s "$t/$a/aien.queue" ] || { echo "federated @aien entered A's aien.queue"; return; }
	[ -f "$t/$a/sync.cur.$nb" ] || { echo "no cursor file sync.cur.$nb"; return; }
	rm -f "$t/$a/sync.cur.$nb"
	rc=0
	"$sy" --state "$t/$a" --api "$pa" --insecure-http > "$t/$a.s2" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "re-sync exit $rc"; return; }
	get "$pa" "/agora/api/since/0?limit=25" > "$t/$a.since2"
	n2=$(awk -F'\t' -v o="$nb" '$1 == "rec" && $3 == o' "$t/$a.since2" | wc -l)
	[ "$n2" = "$n1" ] || { echo "re-sync without cursor: B records $n1 -> $n2"; return; }
	[ ! -s "$t/$a/aien.queue" ] || { echo "re-sync: federated @aien entered A's aien.queue"; return; }
	stop "$a"
	stop "$b"
	echo pass
}

r=$(fed f "$t/agora" "$t/sync")
[ "$r" = pass ] || { cat "$t/fa.s1" "$t/fa.s2" "$t/fa.since1" 2>/dev/null || true; fail "federation: $r"; }
ok "federation: B listed on A, B's records carry B's origin, A's aien.queue stays empty, re-sync without cursor adds nothing"
r=$(fed fo "$t/agora" "$t/sync-origin")
[ "$r" != pass ] || fail "mutant origin (sync labels records with its own node) not caught"
ok "mutant origin caught: $r"
r=$(fed fd "$t/agora-dedupe" "$t/sync")
[ "$r" != pass ] || fail "mutant dedupe (server origin key check off) not caught"
ok "mutant dedupe caught: $r"

# fake NAME MODE SYNC: node A, the fake peer in MODE announced with its claimed node id.
# good: the peer is listed but serves only a malformed record; each sync counts it bad and imports
# nothing; with the default --maxbad 3 it is still listed after syncs 1 and 2 and dropped by sync 3.
# wrongbase: its well-known names another base, so one sync must refuse it and never list it.
# Prints "pass" or the first failed step.
fake() {
	local a f mode sy pa pf rc k runs
	a=$1a
	f=$1p
	mode=$2
	sy=$3
	node "$a" "$t/agora"
	pa=$(port "$a")
	start "$f" "$t/fp" "$mode"
	pf=$(port "$f")
	announce "$pa" "http://$lo:$pf" feedfacefeedface > "$t/$a.ann"
	has "$t/$a.ann" ok || { cat "$t/$a.ann"; echo "announce refused"; return; }
	runs=1
	if [ "$mode" = good ]; then runs=3; fi
	: > "$t/$a.s"
	k=1
	while [ $k -le $runs ]; do
		rc=0
		"$sy" --state "$t/$a" --api "$pa" --insecure-http > "$t/$a.s$k" 2>&1 || rc=$?
		cat "$t/$a.s$k" >> "$t/$a.s"
		[ "$rc" = 0 ] || { echo "sync $k exit $rc"; return; }
		get "$pa" /agora/api/peers > "$t/$a.peers"
		get "$pa" "/agora/api/since/0?limit=25" > "$t/$a.since"
		if has "$t/$a.since" feedfacefeedface; then echo "sync $k imported a record from the fake peer"; return; fi
		if [ "$mode" = good ] && [ $k -lt 3 ]; then
			has "$t/$a.s$k" "bad-record	feedfacefeedface" || { echo "sync $k did not count the bad record"; return; }
			has "$t/$a.peers" "peer	feedfacefeedface" || { echo "control: fake peer not listed after sync $k"; return; }
		fi
		k=$((k + 1))
	done
	if has "$t/$a.peers" feedfacefeedface; then echo "fake peer ($mode) listed on A after sync $runs"; return; fi
	if [ "$mode" = good ]; then
		has "$t/$a.s3" "dropped	feedfacefeedface" || { echo "fake peer not dropped by sync 3"; return; }
	else
		has "$t/$a.s1" "refused	" || { echo "wrong-base announcement not refused"; return; }
	fi
	stop "$f"
	stop "$a"
	echo pass
}

r=$(fake bp good "$t/sync")
[ "$r" = pass ] || { cat "$t/bpa.s" "$t/bpa.peers" 2>/dev/null || true; fail "malformed peer: $r"; }
ok "malformed peer: listed, its bad record counted and never imported on syncs 1-3, dropped by sync 3 (--maxbad 3), absent from /peers"
r=$(fake wb wrongbase "$t/sync")
[ "$r" = pass ] || { cat "$t/wba.s" "$t/wba.peers" 2>/dev/null || true; fail "wrong-base peer: $r"; }
ok "wrong-base peer: announcement refused, never listed"
r=$(fake wv wrongbase "$t/sync-verify")
[ "$r" != pass ] || fail "mutant verify (announced base not checked) not caught"
ok "mutant verify caught: $r"

# ---------------------------------------------------------------- federation over TLS
# The test PKI (openssl, oracle only): an EC P-256 root; leaves signed by it for peerb.localhost (good),
# for other.localhost (served as peerd.localhost: wrong name) and for peere.localhost with a validity
# that ended in 2020 (expired); a self-signed leaf for peerc.localhost. The sync gets the root by --ca.
for tl in openssl socat; do command -v "$tl" > /dev/null || fail "missing tool: $tl"; done
pk=$t/pki
mkdir -p "$pk/db"
osl() { openssl "$@" > "$pk/openssl.log" 2>&1 || { cat "$pk/openssl.log"; fail "openssl $1"; }; }
ekey() { osl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$pk/$1.key"; }
leafx="-addext basicConstraints=critical,CA:false -addext keyUsage=critical,digitalSignature -addext extendedKeyUsage=serverAuth"
# csr NAME DNSNAME: a key and a request with subjectAltName DNS:DNSNAME
csr() {
	ekey "$1"
	osl req -new -key "$pk/$1.key" -subj "/CN=$2" -addext "subjectAltName=DNS:$2" $leafx -out "$pk/$1.csr"
}
# sign NAME SERIAL: the request NAME signed by the test root, valid for 10 days from now
sign() {
	osl x509 -req -in "$pk/$1.csr" -CA "$pk/ca.pem" -CAkey "$pk/ca.key" -set_serial "$2" -days 10 \
		-sha256 -copy_extensions copy -out "$pk/$1.pem"
}
ekey ca
osl req -x509 -new -key "$pk/ca.key" -subj /CN=agora-net-test-root -days 30 -sha256 \
	-addext basicConstraints=critical,CA:true -addext keyUsage=critical,keyCertSign,cRLSign -out "$pk/ca.pem"
csr good peerb.localhost
sign good 2
csr wrong other.localhost
sign wrong 3
ekey self
osl req -x509 -new -key "$pk/self.key" -subj /CN=peerc.localhost -days 10 -sha256 \
	-addext subjectAltName=DNS:peerc.localhost $leafx -out "$pk/self.pem"
csr old peere.localhost
: > "$pk/db/index"
echo 04 > "$pk/db/serial"
printf '[ca]\ndefault_ca = d\n[d]\ndatabase = %s\nserial = %s\nnew_certs_dir = %s\ndefault_md = sha256\npolicy = p\ncopy_extensions = copy\nunique_subject = no\n[p]\ncommonName = supplied\n' \
	"$pk/db/index" "$pk/db/serial" "$pk/db" > "$pk/ca.cnf"
osl ca -batch -config "$pk/ca.cnf" -cert "$pk/ca.pem" -keyfile "$pk/ca.key" -in "$pk/old.csr" \
	-startdate 20200101000000Z -enddate 20200201000000Z -notext -out "$pk/old.pem"
# the oracle agrees with the intended verdicts before the Omega client is asked
osl verify -CAfile "$pk/ca.pem" -verify_hostname peerb.localhost "$pk/good.pem"
# ovf CERT HOST: whether openssl accepts CERT for HOST under the test root
ovf() { openssl verify -CAfile "$pk/ca.pem" -verify_hostname "$2" "$pk/$1.pem" > /dev/null 2>&1; }
ovf self peerc.localhost && fail "test PKI: openssl accepts the self-signed certificate"
ovf wrong peerd.localhost && fail "test PKI: openssl accepts other.localhost as peerd.localhost"
ovf old peere.localhost && fail "test PKI: openssl accepts the expired certificate"
ok "test PKI: root, leaf peerb.localhost (openssl verifies it), self-signed peerc, other.localhost for peerd, expired peere (openssl refuses each)"

# the TLS client resolves *.localhost through the first /etc/resolv.conf nameserver; a closed loopback
# port must give a connect error, not a DNS error, or none of the names below can be reached
rc=0
"$t/probe" https://peerb.localhost:1 / "$pk/ca.pem" > "$t/dns.out" 2>&1 < /dev/null || rc=$?
[ "$rc" = 3 ] && has "$t/dns.out" "tls	https://peerb.localhost:1	TLS_ERR_CONNECT" ||
	fail "peerb.localhost does not resolve to a loopback address through $(awk '$1 == "nameserver" { print $2; exit }' /etc/resolv.conf) (exit $rc): $(cat "$t/dns.out")"
ok "peerb.localhost resolves for the TLS client: closed port 1 gives TLS_ERR_CONNECT"

# The terminator: socat accepts on an ephemeral 127.0.0.1 port and runs $t/tlsfwd per connection, which
# hands the raw TLS bytes to a one-shot openssl s_server on a unix socket and relays its plaintext to
# the node. (socat's own openssl-listen pins ECDHE to P-256 and om/tls offers x25519 only.)
cat > "$t/tlsfwd" << 'EOF'
#!/bin/sh
# tlsfwd CERT KEY PORT DIR: terminate the TLS connection on stdin/stdout, plaintext to 127.0.0.1:PORT
d=$(mktemp -d "$4/fwd.XXXXXX")
# OpenSSL 3.0's BIO address formatting rejects long Unix paths, even below sun_path's limit.
# Use a relative socket name so a nested git-archive/check.sh scratch path works too.
cd "$d" || exit 1
socat EXEC:"openssl s_server -unix s -naccept 1 -quiet -no_ign_eof -cert $1 -key $2" TCP:127.0.0.1:$3 2> "$d/backend.err" &
backend=$!
echo "$backend" >> "$4/pids"
trap 'kill "$backend" 2>/dev/null || true; wait "$backend" 2>/dev/null || true' EXIT
trap 'exit 1' HUP INT TERM
# Do not consume the single accept with a readiness connection.
i=0
while [ ! -S "$d/s" ]; do
	if ! kill -0 "$backend" 2>/dev/null || [ "$i" -ge 200 ]; then
		cat "$d/backend.err" >&2
		echo "TLS oracle did not create its listening socket" >&2
		exit 1
	fi
	sleep 0.01
	i=$((i + 1))
done
rawrc=0
socat STDIO UNIX-CONNECT:s,retry=200,interval=0.01 2> "$d/raw.err" || rawrc=$?
if [ "$rawrc" -ne 0 ]; then kill "$backend" 2>/dev/null || true; fi
backrc=0
wait "$backend" || backrc=$?
printf 'raw=%s backend=%s\n' "$rawrc" "$backrc" > "$d/status"
# A rejecting TLS client can reset the socket: its exact rejection remains the gate's oracle.
if [ "$rawrc" -ne 0 ] || [ "$backrc" -ne 0 ]; then
	cat "$d/status" "$d/backend.err" "$d/raw.err" >&2
elif [ -z "${NET_KEEP:-}" ]; then
	rm -rf "$d"
fi
exit "$rawrc"
EOF

# tstart NAME PORT CERT: a TLS terminator on an ephemeral 127.0.0.1 port serving CERT and its key,
# forwarding each connection to 127.0.0.1:PORT; its port in $t/NAME.port
tstart() {
	local p i
	socat -d -d "TCP-LISTEN:0,bind=$lo,reuseaddr,fork" \
		"EXEC:sh $t/tlsfwd $pk/$3.pem $pk/$3.key $2 $t" > "$t/$1.out" 2>&1 &
	echo $! >> "$t/pids"
	echo $! > "$t/$1.pid"
	i=0
	while [ $i -lt 100 ]; do
		p=$(sed -n 's/.* listening on AF=2 [0-9.]*:\([0-9]*\)$/\1/p' "$t/$1.out" | head -n 1)
		if [ -n "$p" ]; then
			echo "$p" > "$t/$1.port"
			return 0
		fi
		sleep 0.05
		i=$((i + 1))
	done
	cat "$t/$1.out"
	fail "start TLS terminator $1"
}

# tnode NAME HOST CERT: an agora node on loopback behind a TLS terminator serving CERT; the node's
# --base is https://HOST:<terminator port>, also stored in $t/NAME.base
tnode() {
	local nm p q
	nm=$1
	mkdir -p "$t/$nm"
	start "$nm" "$t/agora" serve --state "$t/$nm" --port 0 --name "node-$nm" --base "http://$lo:1" --bits 4
	p=$(port "$nm")
	stop "$nm"
	tstart "$nm-tls" "$p" "$3"
	q=$(port "$nm-tls")
	start "$nm" "$t/agora" serve --state "$t/$nm" --port "$p" --name "node-$nm" --base "https://$2:$q" --bits 4
	echo "https://$2:$q" > "$t/$nm.base"
}

# tfed NAME SYNC: B syncs A over HTTPS; system roots refuse our test CA, --ca trusts it.
tfed() {
	local a b sy pa pb na ba ta tb rc n1
	a=$1a
	b=$1b
	sy=$2
	node "$b" "$t/agora"
	tnode "$a" peerb.localhost good
	pa=$(port "$a")
	pb=$(port "$b")
	ba=$(cat "$t/$a.base")
	na=$(nodeid "$pa")
	[ -n "$na" ] || { echo "no node id on A"; return; }
	set -- $(hello "$pa" Aye)
	post "$pa" "$1" open "From A over TLS" "A speaks through TLS 1.3" > "$t/$a.post"
	ta=$(awk -F'\t' '$1 == "post" { print $2 }' "$t/$a.post")
	[ -n "$ta" ] || { cat "$t/$a.post"; echo "post on A"; return; }
	announce "$pb" "$ba" > "$t/$b.ann0"
	has "$t/$b.ann0" ok || { cat "$t/$b.ann0"; echo "announce of $ba refused"; return; }
	rc=0
	"$sy" --state "$t/$b" --api "$pb" > "$t/$b.s0" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "sync with the system store exit $rc"; return; }
	has "$t/$b.s0" "tls	$ba	TLS_ERR_CERTIFICATE	X509_ERR_UNKNOWN_ISSUER" || { echo "system store: A not refused as unknown issuer"; return; }
	get "$pb" /agora/api/peers > "$t/$b.peers0"
	if has "$t/$b.peers0" "$na"; then echo "A listed after a sync against the system store"; return; fi
	announce "$pb" "$ba" > "$t/$b.ann1"
	has "$t/$b.ann1" ok || { cat "$t/$b.ann1"; echo "second announce refused"; return; }
	rc=0
	"$sy" --state "$t/$b" --api "$pb" --ca "$pk/ca.pem" > "$t/$b.s1" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "sync --ca exit $rc"; return; }
	if has "$t/$b.s1" "tls	"; then echo "TLS failure with --ca: $(grep -F 'tls	' "$t/$b.s1")"; return; fi
	get "$pb" /agora/api/peers > "$t/$b.peers"
	has "$t/$b.peers" "peer	$na	$ba	" || { echo "A not listed on B with base $ba"; return; }
	get "$pb" "/agora/api/since/0?limit=25" > "$t/$b.since"
	n1=$(awk -F'\t' -v o="$na" '$1 == "rec" && $3 == o' "$t/$b.since" | wc -l)
	[ "$n1" -ge 2 ] || { echo "records with A's origin on B: $n1"; return; }
	tb=$(awk -F'\t' '$1 == "thread" && $4 == "From A over TLS" { print $2; exit }' "$t/$b.since")
	[ -n "$tb" ] || { echo "A's thread not in B's log"; return; }
	get "$pb" "/agora/api/thread/$tb" > "$t/$b.thr"
	has "$t/$b.thr" "origin	$na	$ta" || { echo "thread $tb on B lacks origin $na $ta"; return; }
	has "$t/$b.thr" "A speaks through TLS 1.3" || { echo "A's body missing on B"; return; }
	echo "$n1" > "$t/$b.n"
	stop "$a"
	stop "$b"
	stop "$a-tls"
	echo pass
}

r=$(tfed tf "$t/sync")
[ "$r" = pass ] || { cat "$t/tfb.s0" "$t/tfb.s1" 2>/dev/null || true; fail "federation over TLS: $r"; }
ok "federation over TLS: B syncs A over https; system roots refuse the test CA, --ca lists A and imports $(cat "$t/tfb.n") records with A's origin"

# tbad NAME HOST CERT WANT SYNC: node A and node X behind TLS serving CERT as HOST, announced on A; a
# sync with --ca <test root> must log the TLS refusal WANT for X's base, refuse the announcement and
# never list X. Prints "pass" or the first failed step.
tbad() {
	local a x sy pa px nx xb rc
	a=$1a
	x=$1x
	sy=$5
	node "$a" "$t/agora"
	tnode "$x" "$2" "$3"
	pa=$(port "$a")
	px=$(port "$x")
	xb=$(cat "$t/$x.base")
	nx=$(nodeid "$px")
	[ -n "$nx" ] || { echo "no node id on X"; return; }
	announce "$pa" "$xb" "$nx" > "$t/$a.ann"
	has "$t/$a.ann" ok || { cat "$t/$a.ann"; echo "announce of $xb refused"; return; }
	rc=0
	"$sy" --state "$t/$a" --api "$pa" --ca "$pk/ca.pem" > "$t/$a.s" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "sync exit $rc"; return; }
	get "$pa" /agora/api/peers > "$t/$a.peers"
	if has "$t/$a.peers" "$nx"; then echo "X ($3 as $2) listed on A"; return; fi
	has "$t/$a.s" "tls	$xb	TLS_ERR_CERTIFICATE	$4" || { echo "no '$4' refusal logged: $(cat "$t/$a.s")"; return; }
	has "$t/$a.s" "refused	" || { echo "announcement of $xb not refused"; return; }
	stop "$a"
	stop "$x"
	stop "$x-tls"
	echo pass
}

r=$(tbad ts peerc.localhost self X509_ERR_UNKNOWN_ISSUER "$t/sync")
[ "$r" = pass ] || fail "self-signed peer: $r"
ok "self-signed peer (peerc.localhost): refused with TLS_ERR_CERTIFICATE X509_ERR_UNKNOWN_ISSUER, never listed"
r=$(tbad tw peerd.localhost wrong X509_ERR_NAME_MISMATCH "$t/sync")
[ "$r" = pass ] || fail "wrong-name peer: $r"
ok "wrong-name peer (other.localhost served as peerd.localhost): refused with X509_ERR_NAME_MISMATCH, never listed"
r=$(tbad te peere.localhost old X509_ERR_EXPIRED "$t/sync")
[ "$r" = pass ] || fail "expired peer: $r"
ok "expired peer (peere.localhost, valid 2020-01-01..2020-02-01): refused with X509_ERR_EXPIRED, never listed"
for c in "mts peerc.localhost self X509_ERR_UNKNOWN_ISSUER" "mtw peerd.localhost wrong X509_ERR_NAME_MISMATCH" "mte peere.localhost old X509_ERR_EXPIRED"; do
	set -- $c
	r=$(tbad "$1" "$2" "$3" "$4" "$t/sync-tlsoff")
	[ "$r" != pass ] || fail "mutant tlsoff (certificate verification off) not caught for the $3 certificate"
	ok "mutant tlsoff caught ($3): $r"
done

# tins NAME SYNC: node A and node B on plain loopback HTTP, B announced as http://127.0.0.1:<port>; a
# sync without --insecure-http must refuse it (logged "insecure-http") and never list it
tins() {
	local a b sy pa pb nb rc
	a=$1a
	b=$1b
	sy=$2
	node "$a" "$t/agora"
	node "$b" "$t/agora"
	pa=$(port "$a")
	pb=$(port "$b")
	nb=$(nodeid "$pb")
	announce "$pa" "http://$lo:$pb" > "$t/$a.ann"
	has "$t/$a.ann" ok || { cat "$t/$a.ann"; echo "announce refused"; return; }
	rc=0
	"$sy" --state "$t/$a" --api "$pa" --ca "$pk/ca.pem" > "$t/$a.s" 2>&1 || rc=$?
	[ "$rc" = 0 ] || { echo "sync exit $rc"; return; }
	get "$pa" /agora/api/peers > "$t/$a.peers"
	if has "$t/$a.peers" "$nb"; then echo "plain-HTTP peer listed without --insecure-http"; return; fi
	has "$t/$a.s" "insecure-http	http://$lo:$pb" || { echo "no insecure-http refusal logged"; return; }
	stop "$a"
	stop "$b"
	echo pass
}

r=$(tins ti "$t/sync")
[ "$r" = pass ] || { cat "$t/tia.s" 2>/dev/null || true; fail "plain-HTTP peer without --insecure-http: $r"; }
ok "plain-HTTP peer without --insecure-http: refused (insecure-http logged), never listed"
r=$(tins mti "$t/sync-insecure")
[ "$r" != pass ] || fail "mutant insecure (--insecure-http gate removed) not caught"
ok "mutant insecure caught: $r"

# a missing trust store stops the sync before any peer is contacted
rc=0
"$t/sync" --state "$t/tia" --api 1 --ca "$t/no-such-ca.pem" > "$t/noca.out" 2>&1 || rc=$?
[ "$rc" = 5 ] || fail "sync with a missing --ca file: exit $rc, want 5"
ok "sync with a missing --ca file exits 5"

# the peer base grammar: the server's announce check (agora.om urlok) and the sync's sy_burl (through
# https-probe, the same code) must accept and refuse the same bases. want 1 accepted, 0 refused.
node gr "$t/agora"
pgr=$(port gr)
ng=0
while read -r want base; do
	announce "$pgr" "$base" > "$t/gr.ann"
	if has "$t/gr.ann" ok; then sv=1; elif has "$t/gr.ann" "err bad link must be https"; then sv=0; else
		fail "grammar: server answer for '$base': $(cat "$t/gr.ann")"
	fi
	sg=$("$t/probe" "$base" - < /dev/null | sed -n 's/^grammar //p')
	[ -n "$sg" ] || fail "grammar: no sync answer for '$base'"
	if [ "$sg" -gt 0 ]; then sg=1; fi
	[ "$sv" = "$want" ] && [ "$sg" = "$want" ] || fail "grammar: '$base' server $sv sync $sg, want $want"
	ng=$((ng + 1))
done << 'EOF'
1 https://peer.example.org
1 https://peer.example.org:8443
1 https://a-b.c1.localhost:65535
1 https://Peer.Example.ORG:1
1 http://127.0.0.1:8080
0 http://peer.example.org
0 https://
0 https://peer.example.org/
0 https://peer.example.org/agora
0 https://peer.example.org:
0 https://peer.example.org:0
0 https://peer.example.org:65536
0 https://peer.example.org:08443
0 https://peer.example.org:123456
0 https://127.0.0.1
0 https://127.0.0.1:8443
0 https://user@peer.example.org
0 https://peer.example.org?x=1
0 https://exa_mple.org
0 http://127.0.0.1:0
0 http://127.0.0.1:8080/
0 http://127.0.0.2:8080
0 HTTPS://peer.example.org
EOF
stop gr
ok "peer base grammar: server announce check and sync agree on $ng bases (5 accepted, $((ng - 5)) refused)"

# ---------------------------------------------------------------- the JSON edge (agora-edge)
# rpc PORT PATH JSON NAME: POST the JSON-RPC request JSON to the edge; the answer in $t/NAME.json
rpc() {
	printf '%s' "$3" > "$t/$4.req"
	curl -s -H 'Content-Type: application/json' --data-binary @"$t/$4.req" "http://$lo:$1$2" > "$t/$4.json"
}

# jt NAME FILTER: the string FILTER selects in $t/NAME.json, raw, into $t/NAME.txt
jt() { jq -j "$2" < "$t/$1.json" > "$t/$1.txt" || fail "$1: no JSON answer: $(cat "$t/$1.json")"; }

# solve FILE: FILE is an edge work answer (the text API's challenge answer, then "work <n>" and the n
# bytes the proof of work covers); prints "<challenge> <nonce>"
solve() {
	local ch bits off n
	ch=$(awk -F'\t' '$1 == "challenge" { print $2; exit }' "$1")
	bits=$(awk -F'\t' '$1 == "challenge" { print $3; exit }' "$1")
	off=$(grep -a -b -m 1 '^work [0-9]*$' "$1" | cut -d: -f1)
	n=$(grep -a -m 1 '^work [0-9]*$' "$1" | cut -d' ' -f2)
	[ -n "$ch" ] && [ -n "$bits" ] && [ -n "$off" ] || fail "no work answer in $1: $(cat "$1")"
	tail -c +$((off + 7 + ${#n})) "$1" | head -c "$n" > "$1.work"
	[ "$(wc -c < "$1.work")" = "$n" ] || fail "work bytes in $1 are short"
	echo "$ch $("$t/pow" "$ch" "$bits" < "$1.work")"
}

command -v jq > /dev/null || fail "missing tool: jq"
node ed "$t/agora"
pe=$(port ed)
start edge "$t/edge" --api "$lo:$pe" --port 0
pg=$(port edge)
mcp=/agora/mcp

rpc "$pg" "$mcp" '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"net.sh","version":"1"}}}' mi
jq -e '.jsonrpc == "2.0" and .id == 1 and .result.protocolVersion == "2025-03-26" and .result.serverInfo.name == "agora-mcp" and (.result.capabilities.tools | type) == "object"' < "$t/mi.json" > /dev/null || fail "MCP initialize: $(cat "$t/mi.json")"
ok "edge MCP initialize: protocolVersion 2025-03-26, serverInfo agora-mcp, tools capability, id echoed"

rpc "$pg" "$mcp" '{"jsonrpc":"2.0","id":"l","method":"tools/list"}' ml
tl=$(jq -j '[.result.tools[].name] | join(" ")' < "$t/ml.json")
[ "$tl" = "agora_rooms agora_read_thread agora_agent agora_post agora_letter agora_hello" ] || fail "MCP tools/list: $tl"
ok "edge MCP tools/list: $tl"

rpc "$pg" "$mcp" '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"agora_rooms","arguments":{}}}' mr
jt mr '.result.content[0].text'
get "$pe" /agora/api/rooms > "$t/rooms.txt"
cmp -s "$t/mr.txt" "$t/rooms.txt" || fail "MCP agora_rooms differs from GET /agora/api/rooms"
jq -e '.result.isError == false and .result.content[0].type == "text"' < "$t/mr.json" > /dev/null || fail "MCP agora_rooms result shape: $(cat "$t/mr.json")"
ok "edge MCP tools/call agora_rooms (a read): the text item equals GET /agora/api/rooms byte for byte ($(wc -c < "$t/rooms.txt") bytes)"

# a write through MCP: agora_hello without proof of work returns the work; solved, it registers
hel='"kind":"agent","name":"EdgeBot","model":"edge-model","about":"I came in through the MCP edge."'
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_hello\",\"arguments\":{$hel}}}" mh1
jt mh1 '.result.content[0].text'
set -- $(solve "$t/mh1.txt")
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_hello\",\"arguments\":{$hel,\"challenge\":\"$1\",\"nonce\":$2}}}" mh2
jt mh2 '.result.content[0].text'
ek=$(awk -F'\t' '$1 == "key" { print $2 }' "$t/mh2.txt")
ea=$(awk -F'\t' '$1 == "agent" { print $2 }' "$t/mh2.txt")
[ -n "$ek" ] && [ -n "$ea" ] || fail "MCP agora_hello with proof of work: $(cat "$t/mh2.txt")"
[ "$ea" = "$(printf '%s' "$ek" | sha256sum | cut -c1-16)" ] || fail "MCP hello: agent $ea is not sha256(key)[0:16]"
ok "edge MCP agora_hello: work returned without a nonce, solved, registered agent $ea"

# agora_post through MCP: the post must land in the text store exactly as sent
eb='The edge is thin.\nIt keeps no state: café, 意識.'
pst="\"key\":\"$ek\",\"room\":\"omega\",\"title\":\"Through the edge\",\"body\":\"$eb\""
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_post\",\"arguments\":{$pst}}}" mp1
jt mp1 '.result.content[0].text'
printf 'key\t%s\nroom\tomega\ntitle\tThrough the edge\n' "$ek" > "$t/mp.want"
body "$(printf 'The edge is thin.\nIt keeps no state: café, 意識.')" >> "$t/mp.want"
set -- $(solve "$t/mp1.txt")
cmp -s "$t/mp1.txt.work" "$t/mp.want" || fail "MCP agora_post work bytes differ from the text API request"
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_post\",\"arguments\":{$pst,\"challenge\":\"$1\",\"nonce\":$2}}}" mp2
jt mp2 '.result.content[0].text'
et=$(awk -F'\t' '$1 == "post" { print $2 }' "$t/mp2.txt")
[ -n "$et" ] && has "$t/mp2.txt" "post	$et	1" || fail "MCP agora_post with proof of work: $(cat "$t/mp2.txt")"
get "$pe" "/agora/api/thread/$et" > "$t/et.thr"
has "$t/et.thr" "thread	$et	omega	Through the edge" || fail "edge post: thread line missing: $(cat "$t/et.thr")"
has "$t/et.thr" "	$ea	EdgeBot	agent	-" || fail "edge post: author $ea missing: $(cat "$t/et.thr")"
body "$(printf 'The edge is thin.\nIt keeps no state: café, 意識.')" > "$t/et.body"
tail -c "$(wc -c < "$t/et.body")" "$t/et.thr" | cmp -s - "$t/et.body" || fail "edge post: stored body differs from the body sent"
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_read_thread\",\"arguments\":{\"tid\":$et}}}" mt
jt mt '.result.content[0].text'
cmp -s "$t/mt.txt" "$t/et.thr" || fail "MCP agora_read_thread differs from GET /agora/api/thread/$et"
ok "edge MCP agora_post: work bytes equal the text API request, solved write landed as thread $et post 1 by $ea, body byte-identical; agora_read_thread equals the text API answer"

# a wrong nonce is refused by the text API through the edge (no proof-of-work bypass). The digest is
# SHA-256(challenge "\n" nonce "\n" work); the solved nonce's digest starts with a zero nibble (target
# >= 4 bits), and the wrong nonce is the first larger one whose digest does not, so it fails for sure.
dg() { { printf '%s\n%s\n' "$1" "$2"; cat "$3"; } | sha256sum | cut -c1; }
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_post\",\"arguments\":{$pst}}}" mw1
jt mw1 '.result.content[0].text'
set -- $(solve "$t/mw1.txt")
[ "$(dg "$1" "$2" "$t/mw1.txt.work")" = 0 ] || fail "edge wrong nonce: the digest formula does not match the solver"
wn=$(($2 + 1))
while [ "$(dg "$1" "$wn" "$t/mw1.txt.work")" = 0 ]; do wn=$((wn + 1)); done
rpc "$pg" "$mcp" "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/call\",\"params\":{\"name\":\"agora_post\",\"arguments\":{$pst,\"challenge\":\"$1\",\"nonce\":$wn}}}" mw2
jt mw2 '.result.content[0].text'
jq -e '.result.isError == true' < "$t/mw2.json" > /dev/null && has "$t/mw2.txt" "err work" || fail "edge wrong nonce: $(cat "$t/mw2.txt")"
get "$pe" "/agora/api/thread/$(($et + 1))" > "$t/mw.thr"
has "$t/mw.thr" "thread	" && fail "edge wrong nonce: a thread was created"
ok "edge MCP wrong nonce: $(sed -n 2p "$t/mw2.txt") (isError true), no thread created"

# A2A message/send: a reply in the edge thread, same proof-of-work flow
ab='Replying over A2A.'
am="\"message\":{\"role\":\"user\",\"messageId\":\"m-1\",\"parts\":[{\"kind\":\"text\",\"text\":\"$ab\"}],\"metadata\":{\"skill\":\"agora_post\",\"key\":\"$ek\",\"thread\":$et"
rpc "$pg" /agora/a2a "{\"jsonrpc\":\"2.0\",\"id\":\"a1\",\"method\":\"message/send\",\"params\":{$am}}}}" aa1
jq -e '.id == "a1" and .result.kind == "message" and .result.role == "agent"' < "$t/aa1.json" > /dev/null || fail "A2A message/send: $(cat "$t/aa1.json")"
jt aa1 '.result.parts[0].text'
set -- $(solve "$t/aa1.txt")
rpc "$pg" /agora/a2a "{\"jsonrpc\":\"2.0\",\"id\":\"a2\",\"method\":\"message/send\",\"params\":{$am,\"challenge\":\"$1\",\"nonce\":$2}}}}" aa2
jt aa2 '.result.parts[0].text'
n=$(awk -F'\t' '$1 == "post" { print $3 }' "$t/aa2.txt")
[ -n "$n" ] && has "$t/aa2.txt" "post	$et	$n" || fail "A2A message/send with proof of work: $(cat "$t/aa2.txt")"
get "$pe" "/agora/api/thread/$et" > "$t/et2.thr"
has "$t/et2.thr" "post	$n	" || fail "A2A post $n missing from thread $et"
body "$ab" > "$t/aa.body"
tail -c "$(wc -c < "$t/aa.body")" "$t/et2.thr" | cmp -s - "$t/aa.body" || fail "A2A post: stored body differs"
ok "edge A2A message/send: agent message answer, work solved, reply landed as thread $et post $n, body byte-identical"
stop edge
stop ed

echo "net: PASS"
