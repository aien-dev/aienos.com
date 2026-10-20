//! /proof/ : the public proof (ledger L182). A 27B language model on a DGX Spark through Omega, with no CUDA, one
//! command a stranger can run, and a receipt behind every number. Markdown copy: aien-dev/.github profile/PROOF.md.
//!
//! Every figure below links to the file it came from at a pinned commit. The demo row comes from the receipt of the
//! clean run published in aien-dev/aien-receipts (DEMO_* constants); nothing here is estimated.

use maud::{html, Markup};

pub const TITLE: &str = "Proof: a 27B model on a DGX Spark with no CUDA | AIEN";
pub const DESCRIPTION: &str = "Omega runs Qwen3.8-27B on an NVIDIA DGX Spark without CUDA, libcuda or cubin files. One command to run it yourself, and a receipt behind every number.";
pub const URL: &str = "https://www.aienos.com/proof/";

/// The one command, as the page and the thread show it.
pub const COMMAND: &str = "curl -sSLO https://aienos.com/run && less run && sh run";

const SC: &str = "https://github.com/aien-dev/aien-sovereign-core-mv";
const W4_NOTES: &str = "https://github.com/aien-dev/aien-sovereign-core-mv/blob/8e43126a/evidence/qwen38-w4default-20261020T0826Z/NOTES.md";
const W4_README: &str = "https://github.com/aien-dev/aien-sovereign-core-mv/blob/8e43126a/docs/inference/qwen38/README.md";
const W4_GATE: &str = "https://github.com/aien-dev/aien-sovereign-core-mv/blob/8e43126a/docs/inference/qwen38/w4-fit-gate.json";
const CEILING: &str = "https://github.com/aien-dev/omega-mv/blob/3c8ce239/docs/MACHINE_CEILING.md";
const NVRM: &str = "https://github.com/aien-dev/physics-mv/blob/6d7cf0d4/nvrm/nvrm.c";
const PROGRAM: &str = "https://github.com/aien-dev/omega-mv/blob/4aa7d6ca/docs/PROGRAM.md";
const ATLAS: &str = "https://github.com/aien-dev/atlas";
const RECEIPTS: &str = "https://github.com/aien-dev/aien-receipts";
const OMEGA: &str = "https://github.com/aien-dev/omega-mv";
const PHYSICS: &str = "https://github.com/aien-dev/physics-mv";

// The clean run of the demo (aien-receipts demos/). Filled from its demo-receipt.json and answer.json.
pub const DEMO_RECEIPT: &str = "https://github.com/aien-dev/aien-receipts/tree/main/demos/l182-one-command/20261020T184322Z";
pub const DEMO_TOK_S: &str = "14.15 tokens/s (calc:62fd33ec)";
pub const DEMO_SHARE: &str = "0.708 (calc:8fe79f5c)";
pub const DEMO_CHIP_TOK_S: &str = "14.22 tokens/s";
pub const DEMO_TOTAL_MIN: &str = "12.9";
pub const DEMO_STEPS: &str = "83";

/// One measured rung: what, result, share of 273 GB/s, receipt link.
struct Rung {
    what: &'static str,
    result: &'static str,
    share: &'static str,
    receipt: &'static str,
    receipt_name: &'static str,
}

fn rungs() -> Vec<Rung> {
    vec![
        Rung { what: "GPU reading memory, best single path", result: "245.1 GB/s", share: "0.898", receipt: CEILING, receipt_name: "MACHINE_CEILING.md" },
        Rung { what: "Every memory path loaded at once (GPU, CPU, disk)", result: "222.3 GB/s (calc:e2b9a9f6)", share: "0.814 (calc:a3c62418)", receipt: CEILING, receipt_name: "MACHINE_CEILING.md" },
        Rung { what: "Whole 27B model, 8-bit weights (25.65 GB read per token), chip time", result: "8.50 tokens/s (calc:69c20e5a)", share: "0.799 (calc:cf5b6ca5)", receipt: W4_NOTES, receipt_name: "NOTES.md" },
        Rung { what: "Whole 27B model, 4-bit weights (13.65 GB on the chip), chip time", result: "14.29 tokens/s (calc:f0c1b118)", share: "0.714 (calc:babb1a77)", receipt: W4_NOTES, receipt_name: "NOTES.md" },
        Rung { what: "Same, wall clock, free running", result: "12.39 tokens/s (calc:2e64d8f1)", share: "0.619 (calc:4148346f)", receipt: W4_NOTES, receipt_name: "NOTES.md" },
        Rung { what: "The demo below, clean folder, wall clock", result: DEMO_TOK_S, share: DEMO_SHARE, receipt: DEMO_RECEIPT, receipt_name: "demo receipt" },
    ]
}

fn a(href: &str, text: &str) -> Markup {
    html! { a href=(href) { (text) } }
}

pub fn render_proof() -> Markup {
    html! {
        article class="narr-page" {
            div class="narr-column" {
                header class="narr-header" {
                    p class="narr-kicker" { "Proof" }
                    h1 class="narr-title" { "A 27B language model on a DGX Spark, with no CUDA" }
                    p {
                        "One person, Drake Stapleton, and Claude built an engine called Omega that runs Qwen3.8-27B on an NVIDIA DGX Spark (the GB10 chip) without CUDA: no CUDA toolkit, no libcuda, no cubin files. Omega writes its own GPU machine code and hands it to the open NVIDIA kernel driver through a short list of system calls. You can run it yourself with one command, and every number on this page links to the file it came from."
                    }
                }
                section id="run" class="narr-section" aria-labelledby="run-heading" {
                    h2 id="run-heading" { "Run it yourself" }
                    p { "On a DGX Spark with the stock DGX OS:" }
                    pre { code { (COMMAND) } }
                    p { "Or from GitHub: " code { "git clone https://github.com/aien-dev/aienos.com && sh aienos.com/public/run" } "." }
                    p {
                        "In order, the script checks the machine, installs Rust inside its own folder if you have none, fetches three source repositories at pinned commits, builds Omega and the demo, downloads the official Qwen3.8-27B-FP8 weights (Apache 2.0) at a pinned revision and checks the sha256 of every file, converts them once to 4 bits, answers one prompt and prints the speed. Everything stays in one folder: 43 GB when our run finished (the note inside the script says about 65 GB; it asks for 70 GB free). No sudo, no system packages."
                    }
                    p {
                        "Our clean run took " (DEMO_TOTAL_MIN) " minutes (calc:e412ee1c) from an empty folder to the GPU step, on a Spark that was busy with other work. On the GPU it loaded the model in 26.6 s, read the 28-token prompt in 2.0 s and wrote " (DEMO_STEPS) " tokens (the word pieces a model writes) at " (DEMO_TOK_S) " wall clock, " (DEMO_CHIP_TOK_S) " while the GPU was working. Timed again on its own, while another job was sharing the GPU, the GPU step took 107.3 s end to end and wrote exactly the same 83 tokens; setup plus that step is 14.7 minutes (calc:3cb838d3). Its full output and timings are in the " (a(DEMO_RECEIPT, "demo receipt")) ", so you can compare your run with ours."
                    }
                }
                section id="numbers" class="narr-section" aria-labelledby="numbers-heading" {
                    h2 id="numbers-heading" { "The numbers, and what each one measures" }
                    p {
                        "Writing one token means reading every weight of the model once, so tokens per second times bytes per token is the memory traffic. NVIDIA's figure for the Spark is 273 GB/s; that is the wall. Chip time counts only the time the GPU is working; wall clock also counts the host's time between tokens. The rows go from the narrowest measurement to the most complete, so the gaps stay visible."
                    }
                    div class="mx-table-wrap" {
                        table class="mx-table" {
                            thead { tr { th { "What was measured" } th { "Result" } th { "Share of 273 GB/s" } th { "Receipt" } } }
                            tbody {
                                @for r in rungs() {
                                    tr { td { (r.what) } td { (r.result) } td { (r.share) } td { (a(r.receipt, r.receipt_name)) } }
                                }
                            }
                        }
                    }
                }
                section id="not" class="narr-section" aria-labelledby="not-heading" {
                    h2 id="not-heading" { "What it is not" }
                    ul {
                        li { "One model, one prompt at a time, greedy decoding (always the most likely next token). No batching and no server in this demo." }
                        li { "The prompt is read one token at a time through the same path as writing. Short prompts are quick; a 2,047-token prompt takes about 148 s (" (a(W4_README, "README")) "). A faster prompt path is the next piece of work." }
                        li { "4 bits loses some accuracy against 8 bits. The 4-bit check passed every mark frozen before the run (" (a(W4_GATE, "w4-fit-gate.json")) "): the same next token as 8 bits on 0.938 of positions pooled over five scored texts, against a limit of 0.87 (" (a(W4_NOTES, "NOTES.md")) ")." }
                        li { "Tested only on driver 580.173.02. The driver interface is pinned to that version and the script stops on any other." }
                        li { "The GPU's own firmware (GSP, from NVIDIA, loaded by nvidia.ko) still runs. What is gone is the user-space CUDA stack." }
                        li { "Omega and the host side are on public branches in review; the driver file is already on main. The commits are pinned and tagged demo-l182-v1, so they will not move." }
                    }
                }
                section id="driver" class="narr-section" aria-labelledby="driver-heading" {
                    h2 id="driver-heading" { "How Omega talks to the GPU" }
                    p {
                        "The whole driver client is one C file of 813 lines (" (a(NVRM, "nvrm.c")) "). It opens /dev/nvidiactl and /dev/nvidia-uvm and uses 8 commands of the nvidia.ko kernel module (card info, register, allocate, control, free, map memory, map and unmap GPU memory) and 11 of nvidia-uvm.ko (set up the shared address space, register the GPU and its channel, map memory). Omega writes the GPU's command stream and machine code itself, in C. The instruction encodings were worked out by comparing against output of NVIDIA's compiler, which is kept only as a reference inside tests; it never runs in the build. The script prints the demo program's linked libraries so you can check there is no libcuda."
                    }
                }
                section id="built" class="narr-section" aria-labelledby="built-heading" {
                    h2 id="built-heading" { "How it was built" }
                    p {
                        "Drake is an engineer, not a programmer. He directed Claude agents (Opus, Sonnet, Fable) and, for the hardest bugs, GPT-6 Astra, from one chat on the Spark itself. The rules that made it work:"
                    }
                    ul {
                        li { strong { "A seed. " } "The chain starts from " (a(ATLAS, "Atlas")) ", a 1,480-byte hand-written AArch64 binary that checks the next boot stage against a pinned SHA-256 digest. This demo runs on Linux and does not boot through Atlas." }
                        li { strong { "A judge. " } "A change lands only after the Spark judge runs the tests on the real chip and signs the result. Signed bundles are published in " (a(RECEIPTS, "aien-receipts")) "." }
                        li { strong { "A ledger. " } "Every job is a line with an owner and a \"done when\". Agents write what they did beside it." }
                        li { strong { "One PR desk. " } "One agent alone opens and merges pull requests, in order, after the judge passes them." }
                        li { strong { "No guessed numbers. " } "A figure is published only if it points to a measurement, and arithmetic goes through a calculator that returns a receipt id (the calc: ids above). The calculator ledger is internal; the inputs are in the linked files." }
                    }
                    p { "The plan that ties it together is " (a(PROGRAM, "the Omega Program")) "." }
                }
                section id="check" class="narr-section" aria-labelledby="check-heading" {
                    h2 id="check-heading" { "Check it" }
                    p {
                        "Source: " (a(OMEGA, "omega-mv")) " (engine), " (a(PHYSICS, "physics-mv")) " (driver interface), " (a(SC, "aien-sovereign-core-mv")) " (host side). Receipts: " (a(RECEIPTS, "aien-receipts")) "."
                    }
                }
            }
        }
    }
}
