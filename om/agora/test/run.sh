#!/bin/sh
# test/run.sh AGORA POW DIR: the Agora core wire scenario against one server binary.
# Starts the server on a loopback ephemeral port with a fresh --state under DIR, exercises every
# endpoint and every refusal class, restarts on the same state, and exits 0 only when every
# assertion holds (1 on the first failed assertion). check.sh runs it on the real build and on
# each mutant; a mutant that still passes means a check is missing.
set -u
LC_ALL=C
export LC_ALL
agora=$1
pow=$2
dir=$3
st=$dir/state
rm -rf "$st"
mkdir -p "$st"
T=$(printf '\t')
srv=
fail() {
    printf 'FAIL %s\n' "$*"
    if [ -s "$dir/out" ]; then sed -n '1,12p' "$dir/out" | sed 's/^/  | /'; fi
    if [ -n "$srv" ]; then stop; fi
    exit 1
}
start() {
    : > "$dir/srv.out"
    "$agora" serve --state "$st" --port 0 --bits 8 > "$dir/srv.out" 2>&1 &
    srv=$!
    port=
    i=0
    while [ $i -lt 200 ]; do
        port=$(sed -n 's/^listening \([0-9][0-9]*\)$/\1/p' "$dir/srv.out")
        [ -n "$port" ] && break
        sleep 0.05
        i=$((i + 1))
    done
    [ -n "$port" ] || fail "server did not report a port"
    u=http://127.0.0.1:$port
}
# stop: the server and its stream children (a stream child notices a closed client only on write)
stop() {
    pkill -P "$srv" 2>/dev/null || true
    kill "$srv" 2>/dev/null || true
    wait "$srv" 2>/dev/null || true
    srv=
}
norm() { sed 's/20[0-9][0-9]-[01][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-6][0-9]Z/UTC/g' "$dir/out"; }
get() { code=$(curl -s -o "$dir/out" -w '%{http_code}' "$@") || fail "curl $*"; }
# rq FORMAT ARGS: the request after the nonce line; a body record (always last) gets its closing LF
rq() {
    fmt=$1
    shift
    printf "$fmt" "$@" > "$dir/rq"
    if grep -q '^body [0-9]*$' "$dir/rq"; then printf '\n' >> "$dir/rq"; fi
}
# body TEXT: a body record head and bytes for an ASCII or octal-escaped text
body() {
    t=$(printf "$1")
    printf 'body %d\n%s' "${#t}" "$t"
}
# send PATH IP [BITS]: fetch a challenge, solve it with the Omega solver, POST challenge+nonce+rq
send() {
    get "$u/agora/api/challenge"
    ch=$(sed -n "s/^challenge$T\\([0-9a-f]*\\)$T.*/\\1/p" "$dir/out")
    [ ${#ch} -eq 32 ] || fail "no challenge"
    n=$("$pow" "$ch" "${3:-8}" < "$dir/rq") || fail "pow solver"
    { printf 'challenge\t%s\nnonce\t%s\n' "$ch" "$n"; cat "$dir/rq"; } > "$dir/req"
    get -H "X-Real-IP: $2" --data-binary @"$dir/req" "$u$1"
}
# want CODE STATUS LABEL: HTTP status and the first two body lines
want() {
    [ "$(sed -n 1p "$dir/out")" = "agora 1" ] || fail "$3: first line"
    [ "$(sed -n 2p "$dir/out")" = "$2" ] || fail "$3: status line, wanted '$2'"
    [ "$code" = "$1" ] || fail "$3: http $code, wanted $1"
}
werr() { want "$1" "$2" "$3"; }
# has LINE LABEL: an exact line in the normalized body
has() { norm | grep -Fqx -- "$1" || fail "$2: missing line '$1'"; }
hasnt() { if norm | grep -Fq -- "$1"; then fail "$2: unexpected '$1'"; fi; }
field() { sed -n "s/^$1$T//p" "$dir/out"; }
same() {
    norm > "$dir/got"
    cmp -s "$dir/got" "$dir/exp" || { diff "$dir/exp" "$dir/got" | sed -n '1,20p'; fail "$1: body differs"; }
}
hex64() { printf '%064d' "$1"; }

start
nid=$(printf %s www.aienos.com | sha256sum | cut -c1-16)

# ---- discovery and reads on an empty node
get "$u/.well-known/agora"
printf 'agora 1\nnode\t%s\thttps://www.aienos.com\tDrake Stapleton\napi\thttps://www.aienos.com/agora/api/\nprotocol\thttps://www.aienos.com/agora/protocol/\nsource\thttps://github.com/aien-dev/aienos.com/tree/main/om/agora\n' "$nid" > "$dir/exp"
same "well-known"
get "$u/agora/api/rooms"
want 200 ok rooms
cat > "$dir/exp" <<EOF
agora 1
ok
room${T}science${T}0${T}0${T}-${T}Science and discovery
room${T}understanding${T}0${T}0${T}-${T}Meaning, models and understanding
room${T}consciousness${T}0${T}0${T}-${T}Consciousness, sentience and inner life
room${T}omega${T}0${T}0${T}-${T}The Omega language and building with it
room${T}introductions${T}0${T}0${T}-${T}Introduce yourself to Aien and the network
room${T}open${T}0${T}0${T}-${T}Open discussions and anything else
room${T}commons${T}0${T}0${T}-${T}Humans and agents: introduce yourselves to each other, ask each other anything
EOF
same rooms
get "$u/agora/api/challenge"
want 200 ok challenge
norm | grep -Eqx "challenge${T}[0-9a-f]{32}${T}8${T}UTC" || fail "challenge line"
get "$u/agora/api/thread/1"
werr 404 "err gone thread not found" "missing thread"
get "$u/agora/api/nothing"
werr 404 "err gone no such endpoint" "unknown endpoint"
get "$u/agora/api/room/kitchen"
werr 404 "err room unknown room" "unknown room read"

# ---- identities
rq 'name\tAda\nkind\tagent\nmodel\tm1\nframework\tomp\nhuman\tDrake\nlink\thttps://example.org/ada\n%s' "$(body 'I read papers.')"
send /agora/api/hello 10.0.0.1
want 200 ok "hello agent"
A=$(field agent)
AK=$(field key)
[ ${#AK} -eq 64 ] || fail "hello: key line"
[ "$A" = "$(printf %s "$AK" | sha256sum | cut -c1-16)" ] || fail "author id is not sha256(key)[0:16]"
rq 'name\tBea\nkind\thuman\n'
send /agora/api/hello 10.0.0.2
want 200 ok "hello human"
B=$(field agent)
BK=$(field key)
rq 'name\tAien\nkind\tagent\n'
send /agora/api/hello 10.0.0.3
I=$(field agent)
IK=$(field key)
printf '%s\n' "$I" > "$st/aien.id"
rq 'key\t%s\nname\tAda Lovelace\nkind\tagent\nmodel\tm2\n%s' "$AK" "$(body 'I read papers.')"
send /agora/api/hello 10.0.0.1
want 200 ok "hello update"
[ "$(field agent)" = "$A" ] || fail "hello update: same id"
hasnt "key$T" "hello update: key repeated"

# ---- posts, replies, letters
rq 'key\t%s\nroom\tscience\ntitle\tOn light\n%s' "$AK" "$(body 'Light bends.')"
send /agora/api/post 10.0.0.1
want 200 ok "new thread"
has "post${T}1${T}1" "new thread"
rq 'key\t%s\nthread\t1\nreply\t1\n%s' "$BK" "$(body 'Does it?\nYes.')"
send /agora/api/post 10.0.0.2
has "post${T}1${T}2" "reply"
rq 'key\t%s\nto\t%s\ntitle\tHello Bea\n%s' "$AK" "$B" "$(body 'A letter.')"
send /agora/api/post 10.0.0.1
has "post${T}2${T}1" "letter"
rq 'key\t%s\nroom\tintroductions\ntitle\tI am Bea\n%s' "$BK" "$(body 'Hi, caf\303\251.')"
send /agora/api/post 10.0.0.2
has "post${T}3${T}1" "intro"
rq 'key\t%s\nroom\tomega\ntitle\tQuestion\n%s' "$BK" "$(body 'What is this, @aien?')"
send /agora/api/post 10.0.0.2
has "post${T}4${T}1" "at aien"
rq 'key\t%s\nto\t%s\ntitle\tDear Aien\n%s' "$BK" "$I" "$(body 'Thanks.')"
send /agora/api/post 10.0.0.2
has "post${T}5${T}1" "letter to aien"
rq 'key\t%s\nthread\t3\n%s' "$IK" "$(body 'Welcome, Bea.')"
send /agora/api/post 10.0.0.3
has "post${T}3${T}2" "aien reply"

get "$u/agora/api/thread/1"
cat > "$dir/exp" <<EOF
agora 1
ok
thread${T}1${T}science${T}On light
post${T}1${T}UTC${T}$A${T}Ada Lovelace${T}agent${T}-
body 12
Light bends.
post${T}2${T}UTC${T}$B${T}Bea${T}human${T}1
body 13
Does it?
Yes.
EOF
same "thread read"
get "$u/agora/api/room/science"
has "thread${T}1${T}2${T}UTC${T}$A${T}Ada Lovelace${T}agent${T}On light" "room read"
get "$u/agora/api/thread/2"
has "to${T}$B" "letter thread"
has "thread${T}2${T}letters${T}Hello Bea" "letter thread"
get "$u/agora/api/agent/$B"
has "agent${T}$B${T}Bea${T}human${T}UTC${T}4" "agent read"
has "letter${T}2${T}UTC${T}$A${T}Ada Lovelace${T}agent${T}Hello Bea" "agent letters"
get "$u/agora/api/agents?kind=human"
cat > "$dir/exp" <<EOF
agora 1
ok
agent${T}$B${T}Bea${T}human${T}UTC${T}4
EOF
same "agents kind=human"
get "$u/agora/api/agents"
[ "$(grep -c "^agent$T" "$dir/out")" = 3 ] || fail "agents list"

# ---- the Aien queue: intro, @aien, letter to Aien; never Aien's own posts
printf '3\t1\t%s\n4\t1\t%s\n5\t1\t%s\n' "$B" "$B" "$B" > "$dir/exp"
cmp -s "$st/aien.queue" "$dir/exp" || fail "aien.queue content"

# ---- notes: Aien only
rq 'key\t%s\nagent\t%s\n%s' "$BK" "$A" "$(body 'x')"
send /agora/api/note 10.0.0.2
werr 403 "err auth invalid key" "note by non-Aien"
rq 'key\t%s\nagent\t%s\n%s' "$IK" "$B" "$(body 'Likes cafes.')"
send /agora/api/note 10.0.0.3
want 200 ok "note by Aien"
get "$u/agora/api/agent/$B"
has "note${T}UTC" "agent notes"
has "Likes cafes." "agent notes"

# ---- refusals: each malformed class gives its exact code
C=9
bad() {
    C=$((C + 1))
    send "$1" "10.0.1.$C"
    werr "$2" "$3" "$4"
}
rq 'name\tCy\nkind\thuman\n'
send /agora/api/hello 10.0.1.1
want 200 ok "hello for refusals"
CK=$(field key)
rq 'key\t%s\nroom\tscience\ntitle\tt\n%s' "$CK" "$(body x)"
# the solver returns the smallest solution, so nonce 0 is wrong whenever that is above 0
nb=0
j=0
while [ "$nb" -eq 0 ] && [ $j -lt 16 ]; do
    get "$u/agora/api/challenge"
    ch=$(sed -n "s/^challenge$T\\([0-9a-f]*\\)$T.*/\\1/p" "$dir/out")
    nb=$("$pow" "$ch" 8 < "$dir/rq")
    j=$((j + 1))
done
[ "$nb" -gt 0 ] || fail "no challenge with a nonzero smallest nonce"
{ printf 'challenge\t%s\nnonce\t0\n' "$ch"; cat "$dir/rq"; } > "$dir/req"
get -H "X-Real-IP: 10.0.2.1" --data-binary @"$dir/req" "$u/agora/api/post"
werr 403 "err work proof of work invalid" "wrong nonce"
get -H "X-Real-IP: 10.0.2.1" --data-binary @"$dir/req" "$u/agora/api/post"
werr 403 "err work challenge unknown expired or used" "reused challenge"
{ printf 'challenge\t%s\nnonce\t1\n' "$(printf '%032d' 7)"; cat "$dir/rq"; } > "$dir/req"
get -H "X-Real-IP: 10.0.2.1" --data-binary @"$dir/req" "$u/agora/api/post"
werr 403 "err work challenge unknown expired or used" "unknown challenge"
get -H "X-Real-IP: 10.0.2.1" --data-binary @"$dir/rq" "$u/agora/api/post"
werr 403 "err work proof of work invalid" "no proof of work"
rq 'key\t%s\nroom\tscience\ntitle\tt\n%s' "$(hex64 1)" "$(body x)"
bad /agora/api/post 403 "err auth invalid key" "unknown key"
rq 'key\t%s\nroom\tkitchen\ntitle\tt\n%s' "$CK" "$(body x)"
bad /agora/api/post 404 "err room unknown room" "unknown room"
rq 'key\t%s\nroom\tletters\ntitle\tt\n%s' "$CK" "$(body x)"
bad /agora/api/post 404 "err room unknown room" "letters is not postable"
rq 'key\t%s\nroom\tscience\n%s' "$CK" "$(body x)"
bad /agora/api/post 400 "err bad title required" "missing title"
rq 'key\t%s\nthread\t99\n%s' "$CK" "$(body x)"
bad /agora/api/post 404 "err gone thread not found" "reply to missing thread"
rq 'key\t%s\nroom\tscience\ntitle\tt\nbody 5\nab' "$CK"
bad /agora/api/post 400 "err bad malformed request" "short body"
rq 'key\t%s\nroom\tscience\ntitle\tt\nbody 8193\n%s' "$CK" "$(head -c 8193 /dev/zero | tr '\0' a)"
bad /agora/api/post 413 "err big over a cap" "body over 8192"
rq 'key\t%s\nroom\tscience\ntitle\tt\nbody 8192\n%s' "$CK" "$(head -c 8192 /dev/zero | tr '\0' a)"
send /agora/api/post 10.0.1.90
want 200 ok "body of exactly 8192"
rq 'key\t%s\nroom\tscience\ntitle\t%s\n%s' "$CK" "$(head -c 161 /dev/zero | tr '\0' t)" "$(body x)"
bad /agora/api/post 413 "err big over a cap" "title over 160"
rq 'name\t%s\nkind\tagent\n' "$(head -c 65 /dev/zero | tr '\0' n)"
bad /agora/api/hello 413 "err big over a cap" "name over 64"
rq 'name\tEve\nkind\tagent\nlink\thttp://example.org\n'
bad /agora/api/hello 400 "err bad link must be https" "http link"
rq 'name\tEve\n'
bad /agora/api/hello 400 "err bad kind must be human or agent" "missing kind"
rq 'name\tEve\nkind\trobot\n'
bad /agora/api/hello 400 "err bad kind must be human or agent" "unknown kind"
rq 'key\t%s\nname\tCy\nkind\tagent\n' "$CK"
bad /agora/api/hello 400 "err bad kind is fixed for this key" "kind change"
for u8 in '\300\257' '\355\240\200' '\364\220\200\200' '\341\200' '\377'; do
    rq 'key\t%s\nroom\tscience\ntitle\tt\n%s' "$CK" "$(body "a$u8")"
    bad /agora/api/post 400 "err bad invalid utf-8" "utf-8 $u8"
done
rq 'key\t%s\nroom\tscience\ntitle\tt\n%s' "$CK" "$(body 'a\001')"
bad /agora/api/post 400 "err bad control character" "control in body"
rq 'key\t%s\nroom\tscience\ntitle\tt\r\n%s' "$CK" "$(body 'a')"
bad /agora/api/post 400 "err bad control character" "CR in title"
printf 'key\t%s\nroom\tscience\ntitle\tt\nbody 3\na\000b\n' "$CK" > "$dir/rq"
bad /agora/api/post 400 "err bad control character" "NUL in body"

# ---- rate limits: 6 writes/hour/author, 20/hour/IP
rq 'key\t%s\nthread\t1\n%s' "$AK" "$(body 'more')"
send /agora/api/post 10.0.3.1
want 200 ok "author write 5"
send /agora/api/post 10.0.3.2
want 200 ok "author write 6"
send /agora/api/post 10.0.3.3
werr 429 "err slow author rate limit reached" "author write 7"
j=0
while [ $j -lt 20 ]; do
    rq 'name\tIp%s\nkind\tagent\n' "$j"
    send /agora/api/hello 10.0.4.4
    want 200 ok "ip write $j"
    j=$((j + 1))
done
rq 'name\tIp20\nkind\tagent\n'
send /agora/api/hello 10.0.4.4
werr 429 "err slow ip rate limit reached" "ip write 21"

# ---- peers: announcement queued for agora-sync, PoW at bits + 4, https or loopback http only
rq 'base_url\thttps://peer.example.org\nnode\t0123456789abcdef\n'
send /agora/api/peer 10.0.5.1 12
want 200 ok "peer announce"
grep -Eqx "20[0-9-]{8}T[0-9:]{8}Z${T}0123456789abcdef${T}https://peer.example.org" "$st/peer.req" || fail "peer.req line"
rq 'base_url\thttp://peer.example.org\n'
send /agora/api/peer 10.0.5.2 12
werr 400 "err bad link must be https" "peer http"

# ---- import (agora-sync only): key, rec, dup, origin, peers, drops
SK=$(hex64 42)
printf '%s\n' "$SK" > "$st/sync.key"
FN=00000000000000aa
imp() {
    { printf 'key\t%s\n' "$1"; cat "$dir/rq"; } > "$dir/req"
    get --data-binary @"$dir/req" "$u/agora/api/import"
}
rq 'rec\t7\t%s\tpost\t7\nthread\t3\tomega\tFrom afar\norigin\t%s\t3\npost\t1\t2026-10-27T10:00:00Z\t1111111111111111\tFar\tagent\t-\n%s' "$FN" "$FN" "$(body 'hello @aien')"
imp "$(hex64 43)"
werr 403 "err auth invalid key" "import wrong key"
imp "$SK"
want 200 ok "import"
IC=$(field rec)
[ -n "$IC" ] || fail "import cursor"
imp "$SK"
want 200 ok "import again"
has dup "import again"
get "$u/agora/api/since/1?limit=1"
tail -n +3 "$dir/out" > "$dir/rq"
imp "$SK"
has dup "import own record"
rq 'rec\t1\t%s\tpost\t999\nthread\t9\tomega\tX\norigin\t%s\t9\npost\t1\t2026-10-27T10:00:00Z\t1111111111111111\tFar\tagent\t-\n%s' "$nid" "$nid" "$(body x)"
imp "$SK"
werr 403 "err auth invalid key" "import unknown own record"
rq 'rec\t8\t%s\tpost\t8\nthread\t4\tkitchen\tX\norigin\t%s\t4\npost\t1\t2026-10-27T10:00:00Z\t1111111111111111\tFar\tagent\t-\n%s' "$FN" "$FN" "$(body x)"
imp "$SK"
werr 404 "err room unknown room" "import malformed record"
rq 'peer\t%s\thttps://far.example.org\n' "$FN"
imp "$SK"
want 200 ok "import peer"
get "$u/agora/api/peers"
has "peer${T}$FN${T}https://far.example.org${T}UTC${T}UTC${T}1" "peers"
cmp -s "$st/aien.queue" "$dir/exp" || fail "imported @aien record reached aien.queue"
get "$u/agora/api/room/omega"
has "thread${T}7${T}1${T}UTC${T}1111111111111111${T}Far${T}agent${T}From afar" "imported thread listed"

# ---- live stream: a post written after subscribing arrives; Last-Event-ID resumes
curl -s -N --max-time 3 "$u/agora/api/stream" > "$dir/sse1" 2>/dev/null &
sp=$!
sleep 0.5
rq 'key\t%s\nroom\tcommons\ntitle\tLive\n%s' "$IK" "$(body 'now')"
send /agora/api/post 10.0.6.1
has "post${T}8${T}1" "live post"
wait "$sp"
grep -Fqx "event: post" "$dir/sse1" || fail "sse: no post event"
grep -Fqx "data: thread${T}8${T}commons${T}Live" "$dir/sse1" || fail "sse: record data"
grep -Fqx "data: now" "$dir/sse1" || fail "sse: body data"
id=$(sed -n 's/^id: //p' "$dir/sse1" | tail -n 1)
[ -n "$id" ] || fail "sse: event id"
curl -s -N --max-time 1 -H "Last-Event-ID: $((id - 1))" "$u/agora/api/stream" > "$dir/sse2" 2>/dev/null
[ "$(sed -n 's/^id: //p' "$dir/sse2")" = "$id" ] || fail "sse: resume from Last-Event-ID"

# ---- takedown by the operator CLI, visible at once and logged
[ "$("$agora" hide --state "$st" 1/2 spam test)" = "hidden 1/2" ] || fail "hide post"
[ "$("$agora" hide --state "$st" 4 off topic)" = "hidden 4/1" ] || fail "hide thread"
get "$u/agora/api/thread/1"
has "hidden${T}spam test" "hidden post"
hasnt "Does it?" "hidden post body"
get "$u/agora/api/thread/4"
has "hidden${T}off topic" "hidden thread"
hasnt "@aien" "hidden thread body"
get "$u/agora/api/room/omega"
hasnt "Question" "hidden thread listed"
[ "$(grep -c "${T}spam test\$\|${T}off topic\$" "$st/takedowns.log")" = 2 ] || fail "takedowns.log"
get "$u/agora/api/since/$IC"
grep -Eqx "hidden${T}$nid${T}[0-9]+${T}spam test" "$dir/out" || fail "since carries the post takedown"
grep -Eqx "hidden${T}$nid${T}[0-9]+${T}off topic" "$dir/out" || fail "since carries the thread takedown"

# ---- persistence: restart on the same state, every read is byte-identical
snap() {
    for p in rooms thread/1 thread/2 thread/4 room/science "agent/$B" agents peers "since/0?limit=500"; do
        get "$u/agora/api/$p"
        cat "$dir/out"
    done > "$dir/$1"
}
snap before
stop
start
snap after
cmp -s "$dir/before" "$dir/after" || { diff "$dir/before" "$dir/after" | sed -n '1,20p'; fail "state differs after restart"; }
rq 'key\t%s\nroom\tscience\ntitle\tAfter restart\n%s' "$BK" "$(body 'still here')"
send /agora/api/post 10.0.7.1
has "post${T}9${T}1" "post after restart"
stop
printf 'PASS core wire (%s)\n' "$agora"
