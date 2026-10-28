#!/bin/sh
# net.sh: loopback tests for the Agora network programs (aien-agora.om, agora-sync.om, agora-edge.om)
# and their mutants. Shell only coordinates; every program under test is Omega built here with the
# pinned omc. Loopback ephemeral ports only; the real model port 18091 is never contacted (the drainer
# always gets --upstream). Usage: sh test/net.sh <omc> <scratch dir>
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
build agora "$here" sha256.om http.om agora.om
build pow "$here" sha256.om test/pow.om
build aien "$here" sha256.om http.om json.om jsonw.om client.om aien-agora.om
build sync "$here" sha256.om http.om client.om agora-sync.om
build edge "$here" sha256.om http.om json.om jsonw.om client.om agora-edge.om
build fu "$here" http.om fake-upstream.om
build fp "$here" sha256.om http.om client.om test/fake-peer.om
ok "built agora pow aien sync edge fake-upstream fake-peer with omc $want"

# mutants: each is one source line changed in a copy of the tree; mut NAME FILE OLD NEW
mkdir -p "$t/m"
mut() {
	d="$t/m/$1"
	mkdir -p "$d/test"
	cp "$here"/*.om "$d/"
	cp "$here"/test/*.om "$d/test/"
	n=$(grep -cxF -- "$3" "$d/$2" || true)
	[ "$n" = 1 ] || fail "mutant $1: the line '$3' occurs $n times in $2"
	awk -v o="$3" -v r="$4" '$0 == o { print r; next } { print }' "$d/$2" > "$d/$2.new"
	mv "$d/$2.new" "$d/$2"
}
mut drain aien-agora.om 'if ok' 'if true'
mut origin agora-sync.om 'q = ac_cat ib q view part d f 16' 'q = sy_gnode ib q'
mut verify agora-sync.om 'vf = sy_verify view part l b sub be b nid' 'vf = true'
mut dedupe agora.om 'if lt efind org oc NE' 'if lt efind org oc 0'
build aien-drain "$t/m/drain" sha256.om http.om json.om jsonw.om client.om aien-agora.om
build sync-origin "$t/m/origin" sha256.om http.om client.om agora-sync.om
build sync-verify "$t/m/verify" sha256.om http.om client.om agora-sync.om
build agora-dedupe "$t/m/dedupe" sha256.om http.om agora.om
ok "built mutants drain origin verify dedupe"

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
