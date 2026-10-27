#!/bin/sh
# Records the /lab/ captures on Drake's DGX Spark (Linux aarch64). Host orchestration only:
# it runs the pinned Omega compiler and the read-only oracles (objdump, spirv-val, spirv-dis,
# PoCL, nvcc, cuobjdump) and writes what they printed, with the command, pins and load.
# Nothing here runs on the GPU. Run it by hand, then `sh check.sh` with OMEGA_ACCEPT_CHANGE=1.
#
# Inputs (environment):
#   OMEGA_MV       omega-mv git directory (default ~/workspace/omega), read with git show only
#   OMEGA_MV_REV   omega-mv revision for irspirv, opencl-host and the GB10 SASS (default below)
#   SPIRV_ROOT     extracted SPIRV-Tools + OpenCL ICD root (default /tmp/spirv-tools-root)
#   NVCC           CUDA compiler (default /usr/local/cuda/bin/nvcc)
set -eu
cd "$(dirname "$0")"
here=$(pwd)
P=../hive-page
omc=$P/omc
pin=$(tr -d ' \n' < $P/omc.sha256)
[ "$(sha256sum "$omc" | cut -d' ' -f1)" = "$pin" ] || { echo 'Compiler pin mismatch' >&2; exit 3; }
wpin=$(tr -d ' \n' < $P/irwasm.sha256)
[ "$(sha256sum $P/irwasm | cut -d' ' -f1)" = "$wpin" ] || { echo 'irwasm pin mismatch' >&2; exit 3; }
mv=${OMEGA_MV:-$HOME/workspace/omega}
rev=${OMEGA_MV_REV:-369dbc96daa1684db2d7c2c7085bbf16b3fd77ee}
sroot=${SPIRV_ROOT:-/tmp/spirv-tools-root}
nvcc=${NVCC:-/usr/local/cuda/bin/nvcc}
[ "$(uname -s) $(uname -m)" = "Linux aarch64" ] || { echo 'record.sh runs on Linux aarch64 only' >&2; exit 2; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
run() { nice -n 19 taskset -c 8-9 "$@"; }
now() { date +%s%N; }
# median of 7 wall-clock runs of "$@" in microseconds; stdin from $in, stdout discarded
in=/dev/null
median() {
    : > "$work/t"
    for k in 1 2 3 4 5 6 7; do
        s=$(now); run "$@" < "$in" > /dev/null; e=$(now)
        echo $(( (e - s) / 1000 )) >> "$work/t"
    done
    sort -n "$work/t" | sed -n 4p
}
ms() { printf '%d.%03d ms' $(( $1 / 1000 )) $(( $1 % 1000 )); }
load() { cut -d' ' -f1 /proc/loadavg; }
stamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
ksha=$(sha256sum axpy.om | cut -d' ' -f1)

# 1. AArch64: the kernel alone, as raw instruction words, disassembled by objdump.
host="$(uname -sm), $(nproc) of $(nproc --all) CPU cores allowed, DGX Spark (GB10)"
in=axpy.om; t_omc=$(median "$omc"); l_omc=$(load)
"$omc" < axpy.om > "$work/axpy.bin"
cat axpy.om harness.om > "$work/prog.om"
in="$work/prog.om"; t_prog=$(median "$omc"); l_prog=$(load)
"$omc" < "$work/prog.om" > "$work/prog"; chmod +x "$work/prog"
echo '3 1048576' > "$work/args"
got=$("$work/prog" < "$work/args")
want=$(awk 'BEGIN{s=0;for(i=0;i<1048576;i++)s+=3*i+2*i+1;printf "%.0f\n",s}')
[ "$got" = "$want" ] || { echo "native run $got != reference $want" >&2; exit 6; }
# Negative control: the same program with the kernel's add turned into sub must not match.
sed 's/^ret add mul a x y$/ret sub mul a x y/' "$work/prog.om" > "$work/mutant.om"
! cmp -s "$work/prog.om" "$work/mutant.om" || { echo 'mutant edit did not apply' >&2; exit 6; }
"$omc" < "$work/mutant.om" > "$work/mutant"; chmod +x "$work/mutant"
bad=$("$work/mutant" < "$work/args" || true)
[ "$bad" != "$want" ] || { echo 'mutant matched the reference' >&2; exit 6; }
in="$work/args"; t_run=$(median "$work/prog"); l_run=$(load)
{
    printf 'compiler   omc sha256 %s (the site pin)\n' "$pin"
    printf 'command    omc < axpy.om > axpy.bin\n'
    printf 'compile    %s, median of 7, %s, load1 %s, %s\n' "$(ms "$t_omc")" "$host" "$l_omc" "$stamp"
    printf 'output     %s bytes of raw AArch64 code, sha256 %s\n' "$(wc -c < "$work/axpy.bin")" "$(sha256sum "$work/axpy.bin" | cut -d' ' -f1)"
    printf 'program    axpy.om + harness.om into one Linux program: %s, median of 7, load1 %s\n' "$(ms "$t_prog")" "$l_prog"
    printf 'run        a=3, n=1048576: checksum %s, awk reference %s, MATCH\n' "$got" "$want"
    printf 'control    add changed to sub in the kernel: %s, MISMATCH as it should be\n' "${bad:-no output}"
    printf 'run time   %s per process (start, 1048576 elements, print, exit), median of 7, load1 %s\n' "$(ms "$t_run")" "$l_run"
    printf 'disasm     objdump -D -b binary -m aarch64 -M no-aliases (%s)\n\n' "$(objdump --version | head -1)"
    objdump -D -z -b binary -m aarch64 -M no-aliases "$work/axpy.bin" | sed -n '/^ *[0-9a-f]*:/p' | sed 's/^ *//'
} > captures/aarch64.txt

# 2. WebAssembly: IR mode, then the pinned irwasm. The page serves this exact file.
in=axpy.om
{ printf 'ir\n'; cat axpy.om; } > "$work/ir.om"
in="$work/ir.om"; t_ir=$(median "$omc")
"$omc" < "$work/ir.om" > "$work/axpy.ir"
in="$work/axpy.ir"; t_w=$(median $P/irwasm)
$P/irwasm < "$work/axpy.ir" > ../../public/lab/axpy.wasm
wsha=$(sha256sum ../../public/lab/axpy.wasm | cut -d' ' -f1)

# 3. SPIR-V: irspirv built from omega-mv with that revision's own omc, validated, run on PoCL (CPU).
git -C "$mv" show "$rev:seed/out/omc" > "$work/omc-mv"
git -C "$mv" show "$rev:seed/spirv/irspirv.om" > "$work/irspirv.om"
git -C "$mv" show "$rev:seed/spirv/opencl-host.c" > "$work/opencl-host.c"
chmod +x "$work/omc-mv"
mvsha=$(sha256sum "$work/omc-mv" | cut -d' ' -f1)
"$work/omc-mv" < "$work/irspirv.om" > "$work/irspirv"; chmod +x "$work/irspirv"
"$work/omc-mv" < "$work/ir.om" > "$work/axpy-mv.ir"
in="$work/axpy-mv.ir"; t_s=$(median "$work/irspirv")
"$work/irspirv" < "$work/axpy-mv.ir" > "$work/axpy.spv"
lib=$sroot/root/usr/lib/aarch64-linux-gnu
pocl=$(ls "$sroot"/pocl/lib/libpocl.so.*.*.* | head -1)
LD_LIBRARY_PATH=$lib "$sroot/root/usr/bin/spirv-val" --target-env opencl2.1 "$work/axpy.spv"
run cc -std=c11 -O2 -Wall -Wextra -Werror -DIRSPIRV_POCL_LIBRARY="\"$pocl\"" -I"$sroot/root/usr/include" \
    -o "$work/opencl-host" "$work/opencl-host.c" -L"$lib" -Wl,-rpath,"$lib" -lOpenCL
pocl_out=$(run "$work/opencl-host" "$work/axpy.spv" run 3 0 1000 2> "$work/pocl.err")
[ "$pocl_out" = "2498500 0" ] || { echo "PoCL run printed $pocl_out" >&2; exit 7; }
dev=$(sed -n 's/.*device=\([^ ]*\).*/\1/p' "$work/pocl.err" | head -1)
{
    printf 'compiler   omega-mv %s: omc sha256 %s builds irspirv\n' "$rev" "$mvsha"
    printf 'command    { printf '"'"'ir\\n'"'"'; cat axpy.om; } | omc > axpy.ir && irspirv < axpy.ir > axpy.spv\n'
    printf 'translate  %s for irspirv, median of 7, %s, load1 %s\n' "$(ms "$t_s")" "$host" "$(load)"
    printf 'output     %s bytes, sha256 %s\n' "$(wc -c < "$work/axpy.spv")" "$(sha256sum "$work/axpy.spv" | cut -d' ' -f1)"
    printf 'validate   spirv-val --target-env opencl2.1: valid (%s)\n' "$(LD_LIBRARY_PATH=$lib "$sroot/root/usr/bin/spirv-val" --version | head -1)"
    printf 'run        PoCL 7.2 on the CPU (%s): run 3 0 1000 = %s, trap 0; reference 2498500, MATCH\n' "$dev" "${pocl_out% *}"
    printf 'not run    no GPU has executed this SPIR-V yet\n\n'
    LD_LIBRARY_PATH=$lib "$sroot/root/usr/bin/spirv-dis" "$work/axpy.spv"
} > captures/spirv.txt

# 4. CUDA C, compiled with nvcc for comparison only. Nothing is launched.
nvv=$("$nvcc" --version | sed -n 's/.*release \([^,]*\), V\(.*\)/\1 (V\2)/p')
cp axpy.cu "$work/axpy.cu"
cd "$work"
in=/dev/null; t_nv=$(median "$nvcc" -arch=sm_121 -o axpy-cuda axpy.cu); l_nv=$(load)
in=/dev/null; t_nvc=$(median "$nvcc" -arch=sm_121 -cubin -o axpy.cubin axpy.cu)
"$nvcc" -arch=sm_121 -cubin -o axpy.cubin axpy.cu
cd "$here"
tk=$(du -sbL "$(dirname "$(dirname "$nvcc")")" | cut -f1)
osz=$(( $(wc -c < "$omc") + $(wc -c < $P/irwasm) ))
{
    printf 'compiler   nvcc %s\n' "$nvv"
    printf 'command    nvcc -arch=sm_121 -o axpy axpy.cu   (host + device, a runnable program)\n'
    printf 'compile    %s, median of 7, %s, load1 %s\n' "$(ms "$t_nv")" "$host" "$l_nv"
    printf 'device     nvcc -arch=sm_121 -cubin: %s, median of 7\n' "$(ms "$t_nvc")"
    printf 'not run    this CUDA program has not been launched; that needs a GPU window\n'
    printf 'disasm     cuobjdump -sass axpy.cubin\n\n'
    "$(dirname "$nvcc")/cuobjdump" -sass "$work/axpy.cubin" | sed -n 's/^ *\(\/\*[0-9a-f]\{4\}\*\/.*;\).*/\1/p' | sed 's/  */ /g'
} > captures/nvcc.txt

# 5. The GB10 run that exists: hand-written asm2 vector add, public receipt in omega-mv.
{
    printf 'source     omega-mv seed/sm121/vecadd.s2, written in asm2 (not compiled from .om)\n'
    printf 'run        GB10 (sm_121), 1048576 sums, 0 wrong, launch_to_done_ns=214257, 2026-10-19\n'
    printf 'control    a mutated copy of the same kernel fails the same check\n'
    printf 'note       launched by omega-mv'"'"'s bytes tool with no CUDA library (seed/sm121/chip6.sh); the Spark still runs NVIDIA DGX OS and the GPU loads NVIDIA signed firmware; the run record says dirty=3 (the bytes tool, copied in)\n'
    printf 'disasm     nvdisasm, as committed in evidence/SEED-0/sm121/vecadd.sass.txt\n\n'
    git -C "$mv" show "$rev:evidence/SEED-0/sm121/vecadd.sass.txt" | sed -n 's/^ *\(\/\*[0-9a-f]\{4\}\*\/.*;\).*/\1/p' | grep -v 'NOP' | sed 's/  */ /g'
} > captures/gb10.txt

# 6. One sentence for the top of the page, built only from the numbers above.
printf 'Measured on my DGX Spark at %s, load1 %s, both compilers on the same %s of %s CPU cores. omc built the Omega kernel and its host loop into a Linux program in %s. nvcc %s built the CUDA C kernel and its host main in %s. Median of 7 each, same minute.\n' \
    "$stamp" "$l_nv" "$(nproc)" "$(nproc --all)" "$(ms "$t_prog")" "${nvv%% *}" "$(ms "$t_nv")" > captures/headline.txt

printf 'stamp\t%s\nkernel_sha256\t%s\nomc_kernel_us\t%s\nomc_program_us\t%s\nnative_run_us\t%s\nir_us\t%s\nirwasm_us\t%s\nirspirv_us\t%s\nnvcc_us\t%s\nnvcc_cubin_us\t%s\nwasm_sha256\t%s\nwasm_bytes\t%s\ncuda_toolkit_bytes\t%s\nomega_tool_bytes\t%s\n' \
    "$stamp" "$ksha" "$t_omc" "$t_prog" "$t_run" "$t_ir" "$t_w" "$t_s" "$t_nv" "$t_nvc" "$wsha" "$(wc -c < ../../public/lab/axpy.wasm)" "$tk" "$osz" > captures/record.tsv
mkdir -p ../../public/lab
cp captures/record.tsv ../../public/lab/record.tsv

# 7. The take-home, exactly as a visitor types it, from a clean directory and a fresh GitHub clone.
#    LAB_CLONE_REF names the branch to clone (default main); LAB_SKIP_HOME=1 keeps the last transcript.
if [ "${LAB_SKIP_HOME:-0}" != 1 ]; then
    home=$(mktemp -d)
    ref=${LAB_CLONE_REF:-main}
    {
        printf '$ git clone --depth 1 https://github.com/aien-dev/aienos.com\n'
        [ "$ref" = main ] || printf '(recorded from branch %s before merge)\n' "$ref"
        git -C "$home" clone -q --depth 1 --branch "$ref" https://github.com/aien-dev/aienos.com 2>&1
        printf '$ cd aienos.com\n$ sh om/lab-page/run.sh\n'
        (cd "$home/aienos.com" && sh om/lab-page/run.sh 2>&1)
        printf '(recorded %s in an empty directory, Linux aarch64, load1 %s)\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(cut -d' ' -f1 /proc/loadavg)"
    } > "$home/home.txt"
    grep -q 'PASS' "$home/home.txt" || { cat "$home/home.txt" >&2; echo 'take-home did not PASS' >&2; exit 7; }
    cp "$home/home.txt" captures/home.txt
    rm -rf "$home"
fi
echo "LAB-RECORD PASS $stamp"
