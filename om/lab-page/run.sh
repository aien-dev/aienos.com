#!/bin/sh
# The /lab/ kernel on your own machine, from a git clone of aien-dev/aienos.com:
#   git clone --depth 1 https://github.com/aien-dev/aienos.com && cd aienos.com && sh om/lab-page/run.sh
# Optional: sh om/lab-page/run.sh A N   (1 <= A <= 1000, 1 <= N <= 4194304; defaults 3 1048576)
# It checks the pinned compiler's sha256, builds axpy.om + harness.om into a Linux program,
# runs it, checks the checksum against awk, proves the check can fail (add turned into sub),
# and writes the WebAssembly the browser runs to om/lab-page/out/. Nothing is downloaded.
set -eu
cd "$(dirname "$0")"
a=${1:-3}
n=${2:-1048576}
case "$a$n" in *[!0-9]*|'') echo 'A and N must be whole numbers' >&2; exit 2;; esac
[ "$a" -ge 1 ] && [ "$a" -le 1000 ] && [ "$n" -ge 1 ] && [ "$n" -le 4194304 ] ||
    { echo 'need 1 <= A <= 1000 and 1 <= N <= 4194304 (keeps the awk reference exact)' >&2; exit 2; }
if [ "$(uname -s) $(uname -m)" != "Linux aarch64" ]; then
    echo "NOT_RUN: the pinned omc is a Linux aarch64 program and this is $(uname -s) $(uname -m)."
    echo 'The same kernel runs in your browser at https://www.aienos.com/lab/'
    exit 0
fi
omc=../hive-page/omc
irwasm=../hive-page/irwasm
for f in omc irwasm; do
    want=$(tr -d ' \n' < "../hive-page/$f.sha256")
    got=$(sha256sum "../hive-page/$f" | cut -d' ' -f1)
    [ "$got" = "$want" ] || { echo "$f sha256 $got is not the pin $want" >&2; exit 3; }
    echo "pin     $f sha256 $got"
done
mkdir -p out
cat axpy.om harness.om > out/prog.om
s=$(date +%s%N); "$omc" < out/prog.om > out/axpy; e=$(date +%s%N)
chmod +x out/axpy
echo "build   omc: axpy.om + harness.om -> out/axpy in $(( (e - s) / 1000 )) us"
s=$(date +%s%N); got=$(echo "$a $n" | ./out/axpy); e=$(date +%s%N)
echo "run     out/axpy a=$a n=$n -> $got in $(( (e - s) / 1000 )) us (whole process)"
ref=$(awk -v a="$a" -v n="$n" 'BEGIN{s=0;for(i=0;i<n;i++)s+=a*i+2*i+1;printf "%.0f\n",s}')
echo "check   awk reference $ref"
[ "$got" = "$ref" ] || { echo 'FAIL: checksum differs from the reference' >&2; exit 1; }
sed 's/^ret add mul a x y$/ret sub mul a x y/' out/prog.om > out/mutant.om
"$omc" < out/mutant.om > out/mutant; chmod +x out/mutant
bad=$(echo "$a $n" | ./out/mutant || true)
[ "$bad" != "$ref" ] || { echo 'FAIL: the mutant (add -> sub) also matched, so the check proves nothing' >&2; exit 1; }
echo "control add -> sub gives $bad: mismatch, so the check can fail"
{ printf 'ir\n'; cat axpy.om; } | "$omc" > out/axpy.ir
"$irwasm" < out/axpy.ir > out/axpy.wasm
echo "wasm    out/axpy.wasm $(wc -c < out/axpy.wasm) bytes, sha256 $(sha256sum out/axpy.wasm | cut -d' ' -f1)"
echo 'PASS'
