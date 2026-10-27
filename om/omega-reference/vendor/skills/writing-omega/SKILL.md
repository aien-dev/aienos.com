---
name: writing-omega
description: Portable guide to writing Omega (.om) programs that the seeded omc compiler accepts and runs correctly: program shape, types, words, traps, compiler errors 1..30, common mistakes with the exact refusal, and tested examples. Use before writing or reviewing any .om program on any machine.
---

# Writing Omega programs

This guide is for any agent or person writing `.om` on any machine. It needs only a clone of
[aien-dev/omega-mv](https://github.com/aien-dev/omega-mv) and of this repository. Every rule
below was observed with a program at the pin in the table; the programs are in
[`examples/`](../../examples/) and [`examples/gate.sh`](../../examples/gate.sh) reruns all of them.
For compiler internals, register rules, wire formats and the reasons behind each rule, read
[`skills/omega-encoding/SKILL.md`](../omega-encoding/SKILL.md).

| Pin | Value |
|---|---|
| omega-mv commit | `e5b32567a7aff3ce594e3254a622a846a6cd190b` |
| compiler `seed/out/omc` | sha256 `2c82480596bbdc50cda9626866209ef662d5834f278f839ff4ce0250c495029f` |
| language reference | `docs/one_layer/OMEGESE.md` at that commit |
| host | Linux AArch64 only; omc writes a static AArch64 Linux ELF |

The language reference is normative. Where this guide and OMEGESE disagree, the compiler at the pin
decides; this guide records the known disagreements.

## 1. Compile and run

```sh
git clone https://github.com/aien-dev/omega-mv && cd omega-mv
git checkout e5b32567a7aff3ce594e3254a622a846a6cd190b
sha256sum seed/out/omc      # must print the pinned hash
sh seed/omega-from-seed.sh  # optional: rebuild omc from the 408-byte loader
seed/out/omc < prog.om > prog; echo "compile $?"
chmod +x prog && ./prog; echo "exit $?"
```

- omc reads the unit on stdin and writes the executable on stdout. Exit 0 means accepted. Any other
  exit is the error number, and stderr has one line `omc: line N: error K`.
- **Never run the output of a refused compile**: stdout may still hold bytes.
- A unit with `fn main` is program mode (an executable). A unit without it is function mode (raw
  machine words for a host to call); never execute function-mode output directly.
- Check behavior, not compile success: compare stdout bytes and the exit status with what you
  expected, and run one deliberately wrong variant to see the check fail.
- To check a whole set the way this repository does:
  `examples/gate.sh path/to/seed/out/omc` prints `PASS name` or `FAIL name: reason` per program and
  ends `GATE PASS examples=N ...` or `GATE FAIL ...` (exit 1); a different compiler hash exits 2.

## 2. Program shape

```
note hello: the smallest complete program
text msg "hello, world\n"
fn main to int
k = write 1 msg 0 len msg
if ne k long len msg
ret 1
end
ret 0
end
```

([`examples/hello.om`](../../examples/hello.om)). The rules it shows:

- **One statement per line.** No semicolons, no braces. Blocks (`fn`, `if`, `else`, `while`, `for`,
  `struct`) close with `end`. Indentation is optional.
- **Prefix words only.** Every operation is a word followed by its values: `add x 1`,
  `mul add a b c`, `lt i n`. Each word has a fixed number of values. Parentheses may group a value
  and add nothing: `(add a b)`.
- **Comments are `note` lines.** `//` and `#` are refused (error 16).
- **`fn NAME p1 T1 p2 T2 ... to R`**, result `R` a type or `void`. Parameters fill **6 slots**: an
  OM-2 unsized `view T *` / `edit T *` takes 2 (pointer and count), every other parameter 1 (a
  scalar, a sized array, a struct). Two unsized views plus three scalars is error 9
  ([`examples/om2_param_slots.om`](../../examples/om2_param_slots.om),
  `mistake_om2_param_slots.om`). `fn main to int` takes no parameters and its `ret` value is the exit
  status (0..255).
- **Assignment is `name = value`.** The first assignment creates the name with the value's type. The
  `=` is optional, which is a trap: `return x` is accepted as "assign x to a new name `return`".
- A function may call a function defined later in the unit, and program-mode functions may recurse.
- Top-level lines are `note`, `fn`, `text`, `global`, `const`, `struct`, and in OM-2 units
  `layout`, `abstract`, `holds` and `export fn`. `struct` and `layout` lines come before the first
  `fn` (error 9 after it). A `const` is visible only after its line.

Every declaration and statement form at the pin (OMEGESE sections 9 to 12 are the full rules):

| Form | Where | Notes |
|---|---|---|
| `note ANY TEXT` | any line | a comment line |
| `om2`, `om2 debug`, `om2 release`, `om2 diag`, `om2 diag debug`, `om2 diag release` | first line that is not blank or `note` | opts the unit into OM-2 (section 9); `release` erases contract checks, `diag` adds a machine-readable stderr record |
| `om2 fmt` | the first line, at byte 0 | prints the canonical text of the rest of the input instead of compiling it |
| `text NAME "bytes"` | top level | a read-only `uchar` array |
| `global NAME T N` | top level, program mode | zero-filled array, never an argument |
| `const NAME T LITERAL` | top level | an integer or float value, visible after its line |
| `struct S f1 T1 f2 T2 ... end` | top level, before the first `fn` | one line; a field `f T N` is an array of 1..64 |
| `layout NAME SIZE`, `field F T [COUNT] at OFF little`/`big` lines, `end` | OM-2, top level, before the first `fn` | an external byte layout of SIZE 1..4096 bytes |
| `abstract NAME T [ops W ...]`, `holds NAME dup32` | OM-2, top level, before the first `fn` | T `uint` or `ulong`; W from `rotr rotl and or xor` |
| `fn NAME p1 T1 ... to R` ... `end` | top level | `inline` after R in OM-2; `export fn` marks an OM-2 function-mode entry |
| parameter `p T` | `fn` line | a scalar |
| parameter `a view T N` / `a edit T N` | `fn` line | a sized array, read-only or writable |
| parameter `a view T *` / `a edit T *` | `fn` line, OM-2 | an array of any length |
| parameter `s edit T N fresh` | `fn` line, OM-2 | scratch: every element written before it is read |
| parameter `p view S` / `p edit S` | `fn` line | a struct, or an OM-2 layout |
| parameter `o out T N` | `fn` line, OM-2 | written by the unit; the calling host copies it out after status 0 |
| `name = value` | function body | the first assignment creates the name |
| `put A i v`, `put p f v` | function body | store into an array element or a field |
| `f v1 v2 ...` | function body | a call to a `void` function |
| `if C` ... `else` ... `end` | function body | `else` is optional; no `else if` |
| `while C bound N` ... `end` | function body | N 1..999999 |
| `for i T K` ... `end` | function body | `unroll` after K in OM-2, K 1..64 |
| `break`, `continue` | inside a loop | |
| `ret`, `ret v` | function body | `ret v` in a function with a result |
| `exit N`, `trap N` | function body | exit status N; trap N 1..14 exits 100+N |
| `requires C`, `ensures C` | OM-2, the first lines of a function | `result` names the returned value |
| `invariant C` | OM-2, the first line of a loop body | |

## 3. Types and values

| Type | Meaning |
|---|---|
| `bool` | `true` / `false`, the result of comparisons |
| `uchar`, `ushort`, `uint`, `ulong` | unsigned 8, 16, 32, 64 bit (also spelled `u8 u16 u32 u64`) |
| `byte`, `short`, `int`, `long` | signed 8, 16, 32, 64 bit (also spelled `i8 i16 i32 i64`); `byte` is **signed**, and `char` is not a type (error 3) |
| `f32`, `f64` | IEEE binary32, binary64 |
| `void` | no result (function result only) |

- **No implicit conversion.** Both values of an operation have one type (error 7). Convert with the
  type word: `long x`, `uint b`, `f64 n`.
- **A bare number takes its type from context**: the other value of its operation, the parameter it
  is passed to, the result type at `ret`. With no context it is error 11: write `x = uint 5`, never
  `x = 5` for a new name.
- Integer literals are decimal with an optional leading `-`. A literal that does not fit its type
  is error 8. No character literals: write `uchar 65`, not `'A'` (error 3).
- Float literals need digits on both sides of the dot: `2.0f64`, `1.0e30f64`, `0.5f32`. The suffix is
  optional where context gives the type.
- **Hex only in OM-2** (section 9): lowercase `0x` and lowercase digits, value must fit
  (`int 0xffffffff` is error 8; write `int -1`).
- `sizeof T`, `sizeof S` and `offsetof S f` are compile-time `ulong` constants.

## 4. Words

| Kind | Words |
|---|---|
| arithmetic | `add sub mul div rem neg` |
| bits | `and or xor not shl shr` (`rotr rotl` in OM-2) |
| compare (give `bool`) | `eq ne lt le gt ge` |
| float | `add sub mul div neg`, `fsqrt fabs fmin fmax` |
| arrays | `new`, `get A i`, `put A i v`, `len A` |
| structs | `new S v1 v2 ...`, `get p f`, `put p f v` |
| control | `if` / `else` / `end`, `while COND bound N`, `for i T K`, `break`, `continue`, `ret`, `exit`, `trap N` |
| conversion | the type words, `bitcast T v` |
| I/O | `read fd A off n`, `write fd A off n`, `pread` / `pwrite fd A off n pos`, and the 67 host words of section 10 |

- `and`, `or`, `xor` on integers of any width are bitwise, not logical: `and ulong 12 ulong 10` is 8
  ([`examples/int_bits_and.om`](../../examples/int_bits_and.om)). Compare first when you want a bool.
- `and` / `or` on bools evaluate **both** sides; there is no short circuit
  ([`examples/mistake_short_circuit.om`](../../examples/mistake_short_circuit.om) traps 3 on the
  index the C reader would expect to be skipped). Test the index in an outer `if`.
- Comparisons follow the type: on `ulong`, `lt x y` with x = 2^64-1 and y = 1 is false.
- `div` and `rem` on signed values round toward zero; `rem` takes the dividend's sign;
  `shr` on a signed value keeps the sign.
- A function with a result is a value, not a statement: write `v = f 1`, never a bare `f 1`
  (error 7). A `void` function is called as a statement.
- A call is the name then its values with no brackets or commas: `ret f 1 2`.

## 5. Traps: what stops a running program

A trap ends the process with exit status 100 + N. Output already written stays written.

| Cause | Trap | Exit | Example |
|---|---|---|---|
| signed overflow in `add sub mul neg` | 1 | 101 | `examples/trap1_overflow.om` |
| `div` or `rem` by zero, and signed minimum divided by -1 | 2 | 102 | `trap2_divzero.om` |
| array index or I/O slice out of bounds | 3 | 103 | `trap3_bounds.om` |
| a loop past its bound | 4 | 104 | `trap4_loop.om` |
| an integer conversion whose value does not fit | 5 | 105 | `trap5_conv.om` |
| stack exhausted (8 MiB) by calls or local arrays | 6 | 106 | `trap6_stack.om` |
| shift or rotate amount outside 0..width-1 | 7 | 107 | `trap7_shift.om` |
| a contract check that fails (OM-2) | 9, 10 | 109, 110 | `contract_requires.om`, `contract_ensures.om` |
| `trap N`, N 1..14 | N | 100+N | `trap_code.om` (`trap 14`, exit 114) |

- Unsigned arithmetic wraps modulo 2^width and never traps
  ([`examples/unsigned_wrap.om`](../../examples/unsigned_wrap.om)).
- Integer conversions are checked: `uchar x` with x = 300 traps 5. Keep the low bits on purpose with
  a mask: `uchar and x 255`.
- Float to integer truncates toward zero and saturates at the type's limits; NaN gives 0; it never
  traps ([`examples/float_to_int.om`](../../examples/float_to_int.om)). No float operation traps:
  1/0 is infinity, 0/0 is NaN, and NaN compares false except with `ne`.
- `exit N` ends the process with status N directly, without the trap convention.

## 6. Loops and bounds

- `while COND bound N` runs the body at most N times, N 1..999999 (error 8 outside). Without
  `bound` the limit is 1000000. One more pass traps 4. Choose N from an argument you can write down
  in a `note`, for example "every pass consumes at least one byte of a 65536-byte input".
- `for i T K` runs i from 0 to K-1. `for` takes no `bound` (error 5); K is the limit.
- `break` and `continue` work in both loops. There is no `else if`: nest an `if` inside `else`.
- No loop runs forever and recursion is bounded by the stack.

## 7. Arrays, texts, globals, structs

- **Local array:** `a = new T N v`, N elements each v, only in a function's outermost block (error 9
  inside a block). N is a literal or an integer `const`; a variable N is error 16. There is no heap
  and no `free` (error 9).
- **Global array:** `global NAME T N` at top level, zero-filled, program mode only. **A global is
  never an argument** (error 19); a function uses it by naming it.
- **Text:** `text NAME "bytes"` is a read-only `uchar` array. Escapes are only `\n \t \\ \"`
  (error 22 for others).
- **Array parameters** state type and exact length: `a view uchar 64` (read-only) or
  `a edit uchar 64` (writable). Another length is error 7. In OM-2, `a view T *` / `a edit T *` take
  any length. Writing to a view or text is error 20.
- `get A i` / `put A i v`: a literal index outside the array is error 21; every other index is checked
  at run time (trap 3). There are no pointers and no address arithmetic.
- **Structs:** `struct Pair a uint b long end`, before the first `fn`. Every field is an 8-byte cell,
  so a struct is never a wire or file layout. `p = new Pair 5 -12`, pass as `p edit Pair` or
  `p view Pair` ([`examples/structs.om`](../../examples/structs.om)).
- **Bytes on the wire:** build them with shifts and masks into a `uchar` array
  ([`examples/wire_le.om`](../../examples/wire_le.om)); OM-2 also has `layout`.

## 8. Names and limits

- Variable, parameter, global, text, const, struct and field names: at most 8 characters (error 13).
  Function names: at most 63, but see section 13 for names that start with an 8-letter keyword.
- A new name may not repeat a visible name, a function or a struct (error 10). A name first assigned
  inside a block ends at that block's `end`; assign it before the block to keep it.
- Reserved words cannot be names, including `len read write text new free view edit global note
  exit const bound` and every host word (`stat`, `link`, `send`, `random`, `fork`, ...). `len` as a
  name is error 16, `free` is error 9.
- 16 registers per function hold parameters, live names, loop counters and the temporaries of the
  current statement. The 17th is error 12 (`examples/err12_registers.om`). The fix is structural:
  move part of the work into a helper function that takes a few scalars and returns one value, end
  the life of names you no longer need by computing them inside a block, and split a long nested
  expression into several assignments of one or two words each.
- Size limits: an array up to 16777216 elements, 4 MiB of local arrays per function, 1 GiB of
  globals (error 23).

## 9. OM-2 units

A unit whose first line that is not blank or `note` is exactly `om2` is an OM-2 unit. OM-1 code keeps
its meaning inside it. OM-2 adds:

- lowercase hex literals; `rotr x n`, `rotl x n` on unsigned types (trap 7 for an amount outside
  0..width-1);
- unsized parameters `a view T *` / `a edit T *` with `len a` the run-time count;
- `part A off n`, only as an argument to an unsized parameter: the n elements of A from off,
  checked at run time (trap 3). A part of a global is error 19; as a value of its own it is error 9;
  to a sized parameter it is error 7;
- `for i T K unroll`, `fn ... inline`, `layout` (external byte layouts), `fresh` scratch arrays
  (error 27), `abstract` types with `seal` / `open` (error 28), `export fn` (at most 32, error 14);
- executable contracts: `requires` and `ensures` lines at the start of a function, `invariant` as the
  first line of a loop body. A failing `requires` traps 9 (exit 109), a failing `ensures` or
  `invariant` traps 10 (exit 110). A clause may not call a function, use floats or host words
  (error 30). The first line `om2 release` erases the checks
  ([`examples/contract_release.om`](../../examples/contract_release.om)).

Examples: [`om2_tour.om`](../../examples/om2_tour.om), [`om2_views.om`](../../examples/om2_views.om),
[`om2_hex_rotr.om`](../../examples/om2_hex_rotr.om), [`contracts_ok.om`](../../examples/contracts_ok.om).

## 10. Host I/O

- Every host word returns a `long`: a count, or minus the Linux errno. `read` may return fewer bytes
  than asked, 0 at end of input, or a negative error; `write` may write fewer. Loop until done and
  handle `k < 0` and `k == 0` separately
  ([`examples/stdin_count.om`](../../examples/stdin_count.om)).
- Descriptors, flags and modes are `int`; offsets, counts and capacities `ulong`; buffers `uchar`
  arrays. Slices are checked before the kernel call (trap 3).
- `main` takes no parameters: read arguments with `argcnt` and `argread`
  ([`examples/echo_args.om`](../../examples/echo_args.om)) and the environment with `envcnt` and
  `envread` ([`examples/env_read.om`](../../examples/env_read.om)). Short capacity returns -105.
- Host words carry the process's own permissions; they are not a sandbox.

The 67 host words at the pin, with their values (OMEGESE section 10 has each one's exact contract).
`A` and `B` are `uchar` arrays, `off`, `n` and `capacity` are `ulong`, and descriptors, flags, modes
and ids are `int`. A path is the slice `A off n`; a packed list is a `u16` count, then `u16`-length
records. Each word is reserved: using it as a name is error 16 at the pin, while an unreserved name
in the same program compiles.

| Area | Words |
|---|---|
| files | `fopen A off n flags mode`, `fclose fd`, `fsync fd`, `fdsync fd`, `syncfs fd`, `memfd A off n flags`, `fseek fd offset whence`, `ftrunc fd length`, `flock fd op`, `fstat fd A off n`, `getdnts fd A off n` |
| paths | `chmod A off n mode`, `fchmod fd mode`, `chown A off n uid gid`, `faccess A off n mode`, `truncate A off n length`, `unlink A off n`, `makedir A off n mode`, `rmdir A off n`, `cwd A off capacity`, `chdir A off n`, `fchdir fd`, `readlnk A off n B off capacity`, `stat A off n B off capacity`, `lstat A off n B off capacity`, `renames A off n flags`, `link A off n`, `symlnk A off n`, `umask mask` |
| processes | `getpid`, `getppid`, `getuid`, `geteuid`, `getgid`, `getegid`, `setsid`, `getpgid pid`, `getsid pid`, `setpgid pid pgid`, `fork`, `killpid pid signal`, `waitpid pid A off n options`, `pipefds A off n flags`, `dupfd fd target flags`, `execprog A off n` |
| arguments, environment, randomness | `argcnt`, `argread index A off capacity`, `envcnt`, `envread index A off capacity`, `random A off n flags` |
| time | `monons`, `realns`, `sleepns interval` |
| sockets | `socknew domain kind protocol`, `bind fd A off n`, `connect fd A off n`, `listen fd backlog`, `accept fd flags`, `sockname fd A off n`, `sockopt fd level option A off n`, `recv fd A off n flags`, `send fd A off n flags`, `sendmsg fd A off n flags`, `shutdown fd how`, `waitio fd events interval`, `pollfds A off n interval`, `peercred fd A off capacity` |

## 11. Compiler errors

Exit status = error number. The only message is the one stderr line `omc: line N: error K`, where N
is the line being read (0 when no line was read). One refusal per code is in `examples/errNN_*.om`
with its exact stderr line in the matching `.expect`, except 24, which needs a generated program.

| Code | Meaning | Example | Fix |
|---|---|---|---|
| 1 | omc could not read its input | `err01_input` (stdin is a directory) | give omc a readable file on stdin |
| 2 | omc could not write its output | `err02_output` (stdout is `/dev/full`) | send stdout to a writable file with space |
| 3 | unknown name, field or function | `err03_unknown_name` | assign the name before use; check the spelling |
| 4 | the line ends before the statement is complete | `err04_line_ends` | give every word all of its values |
| 5 | words left after a complete statement | `err05_extra_words` | count each word's values; there is no infix |
| 6 | malformed number | `err06_bad_number` | decimal digits; hex only in OM-2, lowercase |
| 7 | type mismatch | `err07_type_mismatch` | convert with a type word: `add int 1 int u` |
| 8 | number out of range | `err08_out_of_range` | a literal that fits its type; a bound 1..999999; a trap code 1..14 |
| 9 | a word in a place where it is not allowed | `err09_disallowed` | move it: `new` only in a function's outermost block, `struct` before the first `fn` |
| 10 | duplicate name | `err10_duplicate` | choose a name that is not visible, not a function, not a struct |
| 11 | a number with nothing to give it a type | `err11_ambiguous` | `x = uint 5` |
| 12 | register pool exhausted (16 per function) | `err12_registers` | fewer live names, a helper function, shorter expressions (section 8) |
| 13 | name too long | `err13_long_name` | at most 8 characters (function names 63) |
| 14 | compiler capacity: names, functions, exports, expansion | `err14_capacity` | split the function or the unit |
| 15 | the input ends inside an open block | `err15_missing_end` | close every block with `end` |
| 16 | syntax: unknown word or misplaced form | `err16_syntax` | start each line with a statement word; comments are `note` lines |
| 17 | a line after `ret`, `break`, `continue`, `trap` or `exit` in the same block | `err17_after_path` | delete the line or move it above |
| 18 | a function with a result whose `end` is reachable | `err18_missing_ret` | end every path with `ret v` |
| 19 | a global as an argument, or one array twice in a call | `err19_global_arg` | name the global inside the function; pass distinct arrays |
| 20 | writing to a text or a view | `err20_readonly` | an `edit` parameter, or a local copy |
| 21 | a literal index outside the array | `err21_const_index` | an index 0..N-1 |
| 22 | a text escape other than `\n \t \\ \"` | `err22_text_escape` | build other bytes with `put` |
| 23 | array or pool size limit | `err23_size` | smaller arrays (section 8 limits) |
| 24 | a conditional branch farther than 1 MiB of code | generated, below | move long bodies into functions |
| 25, 26 | never emitted at the pin | none | none |
| 27 | OM-2 `fresh` element read before it is written | `err27_fresh` | `put` every element before the `get` |
| 28 | OM-2 `seal` outside the constructor | `err28_seal` | `seal` only as the whole body of `fn NAMEmk ... to NAME` |
| 29 | OM-2 `layout` field past the size or overlapping | `err29_layout` | keep every field inside SIZE and apart |
| 30 | OM-2 contract clause not allowed | `err30_contract` | integer words on parameters only (and `result` in `ensures`); no calls, floats or host words |

How the emitted set was established at the pin: in `seed/omc/omc.om`, `fn fail` (line 293) writes
the stderr line and exits with the code (`exit int code`, line 322). The only other exits are
`exit int 0` and `exit 0`, so every refusal goes through `fail`. Its callers pass the literals 1..24
and 27..30, or forward a literal 3, 4, 6, 8, 13 or 16 through `lenval`, `nfail` and `unkfail`. No
caller passes 25 or 26, so those two codes are never emitted. A trap inside omc itself would exit
100 + N, which is not an error code.

Error 24 needs more than 1 MiB of code inside one loop. This program, with 270000 copies of the
line `x = add x 1` (4 bytes of code each), was refused with `omc: line 270006: error 24`, the line
of the loop's `end`; with 140000 copies it compiled:

```sh
{ printf 'fn main to int\nx = ulong 0\nc = true\nwhile c bound 1\n'
  yes 'x = add x 1' | head -n 270000
  printf 'c = false\nend\nret int and x 1\nend\n'; } > e24.om
seed/out/omc < e24.om > e24; echo "compile $?"   # compile 24
```

## 12. Common mistakes

Each row is a program in `examples/` with its observed refusal.

| Mistake | Write instead | Refusal | Example |
|---|---|---|---|
| `y = x + 1` | `y = add x 1` | error 5 | `mistake_infix.om` |
| `f(1, 2)` | `f 1 2` | error 6 | `mistake_c_call.om` |
| `if x > 0` | `if gt x 0` | error 7 | `mistake_compare_operator.om` |
| `'A'` | `uchar 65` | error 3 | `mistake_char_literal.om` |
| `// comment` | `note comment` | error 16 | `mistake_slash_comment.om` |
| `x = 1;` | `x = uint 1` | error 6 | `mistake_semicolon.om` |
| `return x` | `ret x` | error 18 (silent assignment) | `mistake_return.om` |
| `else if c` | `else`, then a nested `if c` | error 5 | `mistake_else_if.om` |
| `fn main argc int to int` | `argcnt`, `argread` | error 9 | `mistake_main_args.om` |
| `for i ulong n bound 10` | `while lt i n bound 10` | error 5 | `mistake_for_bound.om` |
| `while c bound 1000000` | a bound 1..999999 | error 8 | `mistake_bound_range.om` |
| `a = new uchar n 0` | a literal or `const` length | error 16 | `mistake_new_var_len.om` |
| `free a` | nothing: no heap | error 9 | `mistake_free.om` |
| `len = ...` | another name | error 16 | `mistake_reserved_name.om` |
| `neg = ...` or `text neg "a"` | another name: `neg` is the negation word | error 16 | `mistake_neg_name.om` |
| a bare call `f 1` with a result | `v = f 1` | error 7 | `mistake_value_stmt.om` |
| a 7th parameter | a struct or an array | error 9 | `mistake_seven_params.om` |
| 2 `view T *` plus 3 scalars (7 slots) | a sized view, a struct, or fewer scalars | error 9 | `mistake_om2_param_slots.om` |
| `emit buf` with `buf` a global | name the global inside the function | error 19 | `err19_global_arg.om` |
| `a view uchar 8` given a 16-element array | the exact length, or OM-2 `view uchar *` | error 7 | `mistake_param_length.om` |
| `trap 15` | `trap 1` .. `trap 14` | error 8 | `mistake_trap_range.om` |
| `struct` after a `fn` | every `struct` before the first `fn` | error 9 | `mistake_struct_after_fn.om` |
| `rotr` or hex without `om2` | an `om2` first line | error 3 | `mistake_om1_rotr.om` |
| `0X10` or `0xFF` | `0x10`, `0xff` | error 6 | `mistake_om2_upper_hex.om` |
| `int 0xffffffff` | `int -1` | error 8 | `mistake_om2_int_hex.om` |
| `part` of a global | `part` of a local | error 19 | `mistake_om2_part_global.om` |
| `part` to a sized parameter | an unsized `view T *` parameter | error 7 | `mistake_om2_part_sized.om` |
| `s = part a 0 2` | `part` only as an argument | error 9 | `mistake_om2_part_statement.om` |
| `if and lt i n eq get a i 0` (C short circuit) | an outer `if lt i n` | trap 3, exit 103 | `mistake_short_circuit.om` |

## 13. Known compiler defects at the pin

- **`bitcast` across signedness below 64 bits** (fix proposed in draft omega-mv #896, not in the
  pin): the result does not compare equal to the reinterpreted value. Observed wrong: `uint` to
  `int` (4294967295 is not -1, and adding 2 traps 1), `int` to `uint`, `ushort` to and from
  `short`, `uchar` to and from `byte`, and `f32` to `int` (-1.0 is not -1082130432 and not below 0).
  Observed correct: `ulong` to and from `long`, `f64` to `long`, `f32` to `uint`, `int` to `f32`.
  Until a fixed compiler is pinned, do not `bitcast` between narrow integer types or from `f32` to
  `int`; decode signed 8, 16 and 32-bit values arithmetically: load into `long`, then subtract 2^32
  (or 2^16, 2^8) when the value is at least 2^31 (2^15, 2^7), as
  [`examples/wire_le.om`](../../examples/wire_le.om) does. For float bits, `bitcast uint` of an
  `f32` is correct.
- **Empty text through `view uchar *`** (omega-mv issue #775): the parameter does not arrive with
  `len` 0 (observed 60 and 28, depending on the other texts in the unit), so a loop over it runs past
  the text (a crash has been reported; not reproduced here). Workaround: pass `part e 0 0`, which
  arrives with `len` 0
  ([`examples/empty_text_part.om`](../../examples/empty_text_part.om)).
- **Function names that begin with an 8-letter keyword:** a function name longer than 8 bytes whose
  first 8 bytes are a keyword (`continue_x`, `truncate_number`, `sockname_x`, `execprog_x`,
  `peercred_x`) is error 16, although function names may be 63 characters. Shorter keyword prefixes
  are fine (`breakx`, `whilex`, `getpid_x`). Workaround: rename (`cont_x`, `trunc_num`)
  ([`examples/defect_keyword_prefix.om`](../../examples/defect_keyword_prefix.om)).
- A fix for both is in progress in omega-mv; until a new compiler is pinned, use the workarounds.
- OMEGESE 9.9 lists `+ - * /` for floats; the compiler refuses them. Write `add sub mul div`.

## 14. Example index

All in [`examples/`](../../examples/); each `NAME.om` has `NAME.expect` (compile status, exit status
or stderr line) and, when it prints, `NAME.stdout` with the exact bytes. Wolfram checks of the
arithmetic expectations are in [`examples/WOLFRAM.md`](../../examples/WOLFRAM.md).

- Start here: `hello`, `exit_status`, `print_decimal`, `functions`, `loops`, `arrays`, `text`.
- Arithmetic: `int_arith`, `int_bits_and`, `unsigned_wrap`, `checked_conv`, `float_arith`,
  `float_to_int`, `consts`.
- Data: `globals`, `structs`, `wire_le`.
- I/O: `stdin_count`, `echo_args`, `env_read`, `tmp_files`.
- Traps: `trap1_overflow` .. `trap7_shift`, `trap_code`.
- OM-2 and contracts: `om2_tour`, `om2_views`, `om2_hex_rotr`, `om2_part_trap`, `om2_param_slots`,
  `contracts_ok`, `contract_requires`, `contract_ensures`, `contract_invariant`, `contract_release`.
- Known defects and their workarounds: `empty_text_part`, `defect_keyword_prefix`.
- Refusals: `err01_input` .. `err30_contract`, `mistake_*`.
