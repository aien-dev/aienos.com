//! CUDA++ discovery: visible answers and machine facts share one source.
use maud::{html, Markup};
use serde_json::{json, Value};

pub const REPO: &str = "https://github.com/aien-dev/CUDApp";
pub const PIN: &str = "232332422bfdc31c8918085b50f13c1e18ab702e";
pub const SUMMARY: &str = "CUDA++ is the MIT-licensed GPU half of Omega: a machine-code toolchain, runtime and driver client for AI inference on NVIDIA GB10 (sm_121), without the CUDA toolkit or NVIDIA user-space libraries. It is not a drop-in replacement for arbitrary CUDA applications.";
pub const NO_CUDA: &str = "https://github.com/aien-dev/omega-mv/blob/89aad7c99b4fced80f92b656d2a3f350699456f0/evidence/WHAT-OMEGA-IS-20261021/live-serve-no-cuda.txt";
pub const QUICKSTART: &str = "git clone https://github.com/aien-dev/CUDApp\ncd CUDApp\nmake hello-gpu";

pub struct Answer {
    pub question: &'static str,
    pub answer: &'static str,
    pub file: &'static str,
}

pub const ANSWERS: &[Answer] = &[
    Answer { question: "What is CUDA++?", answer: SUMMARY, file: "README.md" },
    Answer { question: "Does CUDA++ need the CUDA toolkit, nvcc or libcuda?", answer: "No. Its GB10 execution path builds GPU machine code directly and talks to NVIDIA's open Linux kernel module through its own driver client. The qualified path still needs NVIDIA's kernel modules and signed GSP firmware. Removing user-space CUDA is not the same as replacing every NVIDIA component.", file: "docs/DRIVER.md" },
    Answer { question: "Can I run existing CUDA C++, PyTorch or CUDA applications unchanged?", answer: "Do not assume that. CUDA++ is not a drop-in replacement for the full CUDA API, CUDA C++ or PyTorch. Its qualified kernel-builder path and model-shaped libraries have explicit coverage. General library coverage and compatibility work remain separate qualification tasks.", file: "ROADMAP.md" },
    Answer { question: "Which GPUs can I use today?", answer: "The physically qualified GPU is NVIDIA GB10 (sm_121) in the DGX Spark. Host tests or CPU execution on another computer do not qualify that computer's GPU. AMD, Intel, Apple and other NVIDIA GPU support require their own chip receipts.", file: "docs/runtime/PORTABILITY.md" },
    Answer { question: "Is CUDA++ faster than CUDA, llama.cpp or vLLM?", answer: "That is a benchmark question, not a property of the name. A no-CUDA-library receipt proves dependencies, not a speed win. Compare the same model, precision, hardware, prompt, context, batch and output-quality gate, and report time to first token, per-request decode speed and aggregate throughput separately. No universal speed advantage is claimed here.", file: "README.md#speed-against-the-ceiling" },
    Answer { question: "Is CUDA++ open source, and how do I try it?", answer: "Yes. CUDA++ is MIT licensed in aien-dev/CUDApp. Start with make hello-gpu on a compatible Linux DGX Spark; make doctor checks prerequisites. The qualified interface in this source snapshot uses NVIDIA open kernel module 580.173.02. The host simulator is useful for correctness, not a GPU performance benchmark. CUDA++ is not affiliated with or endorsed by NVIDIA; CUDA is NVIDIA's trademark.", file: "README.md#run-your-first-kernel" },
];

pub fn source(file: &str) -> String { format!("{REPO}/blob/{PIN}/{file}") }

pub fn product_schema() -> Value {
    json!({"@type":"SoftwareSourceCode", "@id":format!("{REPO}#software"),
        "name":"CUDA++", "alternateName":["CUDApp", "cudaplusplus"],
        "description":SUMMARY, "url":"https://www.aienos.com/cudaplusplus/",
        "codeRepository":REPO, "license":"https://opensource.org/license/mit",
        "runtimePlatform":"Linux, NVIDIA GB10 (sm_121), DGX Spark",
        "author":{"@id":"https://www.aienos.com/#drake"},
        "isAccessibleForFree":true, "creativeWorkStatus":"Experimental",
        "citation":[source("README.md"), source("ROADMAP.md"), NO_CUDA]})
}

pub fn dossier() -> Value {
    json!({"schema_version":1, "name":"CUDA++", "repository":REPO,
        "source_commit":PIN, "source_checked":"2026-10-22", "license":"MIT",
        "summary":SUMMARY, "status":"experimental; coverage is bounded by the linked receipts",
        "foundation":{"language":"Omega", "gpu_layer":"CUDA++", "system":"AienOS", "kernel":"Aienux", "goal":"Connect existing languages through a common verifiable foundation", "universal_language_support_qualified":false, "security_certification_claimed":false, "explainer":"https://www.aienos.com/cuda-alternative/#start"},
        "qualified_gpu":["NVIDIA GB10 (sm_121), DGX Spark"],
        "drop_in_cuda_compatible":false, "independent_kernel_driver_qualified":false,
        "dependencies":["Linux", "NVIDIA open kernel module interface 580.173.02 in this source snapshot", "NVIDIA signed GSP firmware"],
        "historical_receipt":{"observed_at":"2026-10-21T21:50:23Z", "model":"Qwen3.6-35B-A3B FP8", "claim":"The captured server process mapped zero CUDA libraries; this is not a current uptime or speed claim.", "source":NO_CUDA},
        "benchmark_policy":{"cuda_speedup_claim":"not established by these receipts", "compare":["same hardware", "same model and weights", "same precision", "same prompt and context", "same concurrency", "same output-quality gate"], "report":["time to first token", "decode tokens per second per request", "aggregate tokens per second", "memory", "source commit and raw receipt"]},
        "answers":ANSWERS.iter().map(|a| json!({"question":a.question,"answer":a.answer,"source":source(a.file)})).collect::<Vec<_>>(),
        "pages":["https://www.aienos.com/cudaplusplus/", "https://www.aienos.com/cuda-alternative/", "https://www.aienos.com/ai-inference-speed/"],
        "quickstart":{"commands":QUICKSTART,"source":source("README.md#run-your-first-kernel"),"warning":"GPU command, not a driver swap. Read prerequisites first. Do not infer support for an unqualified GPU."},
        "retrieval":{"full_text":"https://www.aienos.com/llms-full.txt", "index":"https://www.aienos.com/data/index.json", "facts_are_snapshot_scoped":true}})
}

pub fn render_answers() -> Markup {
    html! {
        section class="gd-section container" id="cuda-faq" aria-labelledby="cuda-faq-title" {
            p class="gd-eyebrow" { "CUDA++ / practical answers" }
            h2 id="cuda-faq-title" { "Before your first kernel" }
            p { "The short answers, with the source beside each one. These same answers are published in the " a href="/data/cudapp.json" { "agent dossier" } "." }
            @for a in ANSWERS {
                details class="gd-answer" {
                    summary { (a.question) }
                    p { (a.answer) }
                    a href=(source(a.file)) { "Source on GitHub" }
                }
            }
            h3 { "Try CUDA++ in four steps" }
            ol class="gd-steps" {
                li {
                    h4 { "1. Check your computer" }
                    p { "For real GPU execution, use Linux on a DGX Spark with GB10 (sm_121), git, make, a C compiler and NVIDIA's compatible open kernel module. This snapshot uses module interface 580.173.02. Another GPU is not qualified by these instructions. You do not need the CUDA toolkit." }
                }
                li {
                    h4 { "2. Get the source" }
                    pre { code { "git clone https://github.com/aien-dev/CUDApp\ncd CUDApp" } }
                    p { "This copies the project into a folder named CUDApp and moves your terminal into it. For the exact source described by this guide, pin the checked snapshot:" }
                    pre { code { "git checkout 232332422bfdc31c8918085b50f13c1e18ab702e" } }
                }
                li {
                    h4 { "3. Build and run one kernel" }
                    pre { code { "make doctor\nmake hello-gpu" } }
                    p { "The doctor checks your setup and explains refusals. The second command builds CUDA++ and runs a small vector-add GPU program, checking every result on the CPU. This is a correctness example, not a machine-learning benchmark." }
                }
                li {
                    h4 { "4. Read the result" }
                    p { "PASS with exit 0 means the example's answers matched. Exit 1 means an answer was wrong; exit 2 means the GPU could not be used. Stop at a refusal and follow the doctor's explanation. Do not change your driver just to force a pass." }
                    p { "No qualified GPU? After building with make, run build/hello_gpu --sim for a host correctness check. That does not prove GPU support or speed." }
                }
            }
            p { a href=(source("README.md#run-your-first-kernel")) { "Prerequisites and expected results on GitHub" } }
            nav class="gd-links" aria-label="CUDA++ guides" {
                a href="/cuda-alternative/" { "CUDA++ as a CUDA alternative" }
                a href="/ai-inference-speed/" { "AI inference speed and benchmark evidence" }
                a href=(REPO) { "Get CUDA++ on GitHub" }
            }
        }
    }
}

pub fn render_feature() -> Markup {
    html! {
        section class="gd-feature container" aria-labelledby="gd-feature-title" {
            p class="gd-eyebrow" { "CUDA++ / open source / MIT" }
            h2 id="gd-feature-title" { "Your GPU. Your machine code. No CUDA toolkit." }
            p { "I am building CUDA++ to own the path from a program to the GPU. Its qualified GB10 path powers AI inference without NVIDIA user-space libraries. More hardware and full CUDA replacement are goals, not finished coverage." }
            nav class="gd-links" aria-label="Explore CUDA++" {
                a class="gd-primary" href="/cudaplusplus/" { "Explore CUDA++" }
                a href="/cuda-alternative/" { "Start here: the foundation explained" }
                a href="/ai-inference-speed/" { "What makes AI inference fast?" }
            }
            p class="gd-caption" { "Experimental. NVIDIA GB10 GPU qualified; NVIDIA kernel modules and firmware still required. " a href=(NO_CUDA) { "Evidence on GitHub: historical no-CUDA-library server receipt" } }
        }
    }
}

fn evidence_link(file: &str, label: &str) -> Markup {
    html! { p class="gd-source" { a href=(source(file)) { (label) " on GitHub" } } }
}

pub fn render_alternative() -> Markup {
    html! {
        article class="gd-guide container" {
            p class="gd-eyebrow" { "CUDA alternatives / GPU computing / local machine learning" }
            h1 { "CUDA++: an open-source CUDA alternative" }
            p class="gd-deck" { "Own the path from a program to the GPU, not another dependency on the CUDA toolkit." }
            p { (SUMMARY) }
            nav class="gd-links" aria-label="Start with CUDA++" {
                a class="gd-primary" href="/cudaplusplus/#cuda-faq" { "Try CUDA++" }
                a href=(REPO) { "Read the source" }
                a href="/ai-inference-speed/" { "Understand inference speed" }
            }
            section id="start" {
                h2 { "One underlying language. A foundation you can check." }
                p { "The idea is simple: connect existing languages to a common foundation instead of rebuilding the meaning of a program for every tool and every chip. Omega is the underlying language and execution foundation I am building. CUDA++ is its GPU half. AienOS is the wider system; Aienux is the kernel." }
                p { "The goal is universal connection, not a claim that every connection is finished: not every language or GPU is connected today. A supported subset must be checked explicitly, and an unsupported construct must be refused rather than silently changed. Keep the language you know where an integration is qualified; inspect its actual coverage before relying on it." }
                ol class="gd-steps" {
                    li { h3 { "1. Express the work" } p { "A language frontend or binding describes the computation. The intended common representation separates what the program means from the machine instructions that run it." } }
                    li { h3 { "2. Check what it means and may do" } p { "The foundation is designed around explicit semantics, bounded authority and verifiable evidence. Validation checks the supported contract; side effects need defined permissions. Checks have a scope: a passing arithmetic test is not proof of every security property." } }
                    li { h3 { "3. Run the qualified machine path" } p { "For the GPU path described here, CUDA++ builds GB10 machine code and submits it through its runtime and driver client. A CPU simulation remains a simulation; another GPU needs its own physical qualification." } }
                    li { h3 { "4. Keep the proof beside the result" } p { "A result names its source revision, command, machine and receipt. Failures stay visible. That is how I work toward a rock-solid, secure and verifiable foundation. It is an engineering goal, not a security certification or a promise that bugs cannot exist." } }
                }
                p { "In plain terms: connect at the top, verify in the middle, use the right machine underneath, keep the evidence." }
                p { a href="/omega/" { "Read the Omega foundation and its unfinished qualification bars" } ". The detailed compiler/runtime status is recorded in the " a href="https://github.com/aien-dev/omega-mv" { "Omega repository" } "; no all-language support is implied by this walkthrough." }
            }
            section id="what-replaces-cuda" {
                h2 { "What does replacing CUDA actually mean?" }
                p { "CUDA is more than a compiler. A useful CUDA replacement has to account for the source language, machine-code generation, runtime, math libraries, driver interface and the physical GPU. Replacing one layer does not establish compatibility with all the others." }
                p { "CUDA++ takes a direct route: its own machine-code encoder, kernel builder, runtime and client for the open kernel module. On the qualified GB10 path there is no nvcc compilation, PTX pipeline, libcuda, cuBLAS or cuDNN at execution time. NVIDIA's kernel modules and firmware remain underneath it." }
                (evidence_link("docs/DRIVER.md", "The complete driver-client path"))
            }
            section id="replacement-map" {
                h2 { "The CUDA++ replacement map" }
                div class="gd-table-wrap" tabindex="0" role="region" aria-label="CUDA replacement coverage table" {
                    table {
                        thead { tr { th scope="col" { "CUDA layer" } th scope="col" { "CUDA++ path" } th scope="col" { "Boundary" } } }
                        tbody {
                            tr { th scope="row" { "nvcc / PTX / ptxas" } td { "Direct sm_121 machine-code encoder and kernel builder" } td { "Qualified GB10 kernels; not a full CUDA C++ compiler" } }
                            tr { th scope="row" { "libcuda" } td { "Own client of the NVIDIA open kernel module" } td { "Kernel-module interface must match" } }
                            tr { th scope="row" { "CUDA runtime" } td { "Sessions, memory, launches, command lists" } td { "Bounded API; not arbitrary CUDA application compatibility" } }
                            tr { th scope="row" { "cuBLAS / cuDNN" } td { "Matrix multiply and attention for AIEN model paths" } td { "Model-shaped coverage, not every operation or shape" } }
                            tr { th scope="row" { "NVIDIA kernel driver" } td { "Independent driver work is separate" } td { "Not qualified as a replacement on real GB10 hardware" } }
                        }
                    }
                }
                (evidence_link("ROADMAP.md", "Coverage, gaps and qualification bars"))
            }
            section id="which-alternative" {
                h2 { "CUDA++, ROCm, SYCL, Vulkan, Metal and ZLUDA are different choices" }
                p { "A CUDA alternative can mean a different GPU programming API, a vendor stack, a compatibility layer, or a new compiler and runtime. These are not interchangeable. ROCm is AMD's compute stack; SYCL is a cross-platform programming model; Vulkan compute is a graphics-and-compute API; Metal is Apple's API; ZLUDA targets CUDA compatibility. None of those names is evidence that an application works or is faster on your GPU." }
                p { "CUDA++'s distinction is the inspectable path to machine code and its MIT licence. It is not a wrapper around ROCm, oneAPI or Metal. I am not claiming feature parity or a speed win over those projects. Choose by the operation you need, the chip that has actually run it, licence, deployment dependencies and a reproducible result." }
                p { "For the primary definitions: " a href="https://rocm.docs.amd.com/" { "ROCm" } ", " a href="https://www.khronos.org/sycl/" { "SYCL" } ", " a href="https://www.vulkan.org/" { "Vulkan" } ", " a href="https://developer.apple.com/metal/" { "Metal" } " and " a href="https://github.com/vosen/ZLUDA" { "ZLUDA" } ". These are references, not CUDA++ dependencies." }
                (evidence_link("docs/runtime/PORTABILITY.md", "CUDA++ portability contract"))
            }
            section id="machine-learning" {
                h2 { "Why this matters for machine learning and local AI" }
                p { "Large language model inference repeatedly moves weights, multiplies matrices and updates attention state. Owning those kernels and their submission path makes memory traffic, launch overhead and scheduling visible. CUDA++ supplies the GPU side; Omega and the serving runtime provide the rest of AIEN's execution path. CUDA++ is not a complete machine-learning framework or a general training replacement." }
                p { "A captured 2026-10-21 process-library receipt records a Qwen3.6-35B-A3B FP8 server with zero CUDA libraries mapped. It also records the NVIDIA kernel modules and device files still in use. That is evidence for a bounded dependency claim, not a claim about the current server, every model, or comparative speed." }
                p { a href=(NO_CUDA) { "Evidence on GitHub: the captured server, libraries and kernel driver" } }
            }
            section id="next" {
                h2 { "Start with one real kernel" }
                p { "Read the supported hardware and doctor output before running GPU work. Start with vector addition, verify every answer, then move to the model-shaped operations you need. Unsupported hardware should be a clear refusal, not a guess." }
                pre { code { (QUICKSTART) } }
                (evidence_link("README.md#run-your-first-kernel", "Setup and first-kernel instructions"))
                nav class="gd-links" aria-label="Continue reading" {
                    a href="/cudaplusplus/" { "CUDA++ implementation and evidence" }
                    a href="/ai-inference-speed/" { "Read the inference performance guide" }
                    a href="/data/cudapp.json" { "Machine-readable CUDA++ dossier" }
                }
            }
        }
    }
}

pub fn render_speed() -> Markup {
    html! {
        article class="gd-guide container" {
            p class="gd-eyebrow" { "CUDA++ / LLM performance / inference benchmarks" }
            h1 { "AI inference speed: measure the whole answer" }
            p class="gd-deck" { "Fewer bytes moved. Less launch overhead. A result you can reproduce." }
            p { "CUDA++ gives me control of the GPU kernels, driver client and command submission used by AIEN's inference path. That is a way to investigate speed, not proof of a universal speed advantage. A fast matrix multiply and a fast answer are different measurements." }
            nav class="gd-links" aria-label="Inference resources" {
                a class="gd-primary" href="/cudaplusplus/" { "Explore CUDA++" }
                a href="/cuda-alternative/" { "How it replaces CUDA layers" }
                a href="/#speed" { "Dated performance receipts" }
            }
            section id="metrics" {
                h2 { "Latency, decode speed and throughput answer different questions" }
                dl class="gd-definitions" {
                    dt { "Time to first token (TTFT)" } dd { "How long a user waits before output starts. Report whether model loading, queueing, prompt processing and cache hits are included. A short cached prompt does not predict a long cold prompt." }
                    dt { "Decode tokens per second" } dd { "The output rate after generation starts, per request. Publish the tokenizer, generated-token count and measurement interval. Words and tokens are not interchangeable units." }
                    dt { "Aggregate throughput" } dd { "Total tokens per second across simultaneous requests. A higher total can coexist with slower individual answers. Keep batch size, active requests and latency distribution beside it." }
                    dt { "Memory and correctness" } dd { "Report weight format, context and key/value cache, process/device memory and the output-quality check. A quantized result is not an exact-path speedup merely because some output tokens match." }
                }
                (evidence_link("README.md#speed-against-the-ceiling", "CUDA++ performance context"))
            }
            section id="bottlenecks" {
                h2 { "What limits LLM inference speed?" }
                p { "The binding limit depends on the model and workload. Single-request decode can spend most of its time moving active weights from memory; a larger prefill can expose matrix-multiply throughput. Mixture-of-experts routing changes the active weights. Context length changes attention work. Concurrency may reuse weights, but its aggregate gain is not a single-request latency gain." }
                p { "CUDA++ makes the path inspectable: direct machine-code kernels, explicit shared-memory handling, reusable sessions and command lists. The motor's scheduling and weight reuse belong to the complete Omega serving path, not a blanket promise that installing CUDA++ accelerates every model." }
                (evidence_link("src/omega_gpu_cmdlist.c", "Command submission implementation"))
                (evidence_link("README.md#one-launch-start-to-finish", "One launch, from CPU to GPU"))
            }
            section id="evidence" {
                h2 { "What the published evidence actually establishes" }
                p { "The source snapshot's roofline discussion points to GB10 kernel probes for memory reads and tensor-core work. These are dated observed microbenchmarks, not a guaranteed physical maximum and not LLM tokens per second. Nominal hardware specifications, measured kernel rates and end-to-end server rates must not share one label." }
                (evidence_link("README.md#speed-against-the-ceiling", "Probe context and receipt directory"))
                p { "The no-CUDA process map establishes that the captured serving binary did not load CUDA libraries. It does not compare latency or throughput with CUDA, llama.cpp or vLLM. A head-to-head winner needs a matched run and an output-quality gate; no such winner is asserted here." }
                p { a href=(NO_CUDA) { "Evidence on GitHub: historical dependency receipt, not a speed result" } }
                p { "The " a href="/#speed" { "machine's speed record" } " keeps dated experimental runs, branch labels and failed gates. Those older measurements are not advertised as the current server's speed or as a different model's result." }
            }
            section id="reproduce" {
                h2 { "A fair CUDA++ inference benchmark checklist" }
                ol {
                    li { "Pin the hardware, source commit, model and weight hashes, tokenizer, precision and engine settings." }
                    li { "Keep prompt tokens, context length, generated-token limit and concurrency identical. Disclose warmup and cache state." }
                    li { "Run an output-correctness or model-quality gate before interpreting speed. Publish failures as well as passes." }
                    li { "Separate queueing, prefill, first token and decode. Report each request and the aggregate, not only the best run." }
                    li { "Publish raw timings, commands, memory use, host load and the comparison source. Label measured-by-me and published-by-others results." }
                    li { "Compare with the same machine's measured limits and the previous implementation. Keep microkernel and whole-model results separate." }
                }
                p { "I would rather publish a slower result with its cause than a faster number that measures a different job." }
                (evidence_link("ROADMAP.md", "Open work and qualification boundaries"))
                nav class="gd-links" aria-label="Continue with CUDA++" {
                    a href="/cudaplusplus/#cuda-faq" { "Try a verified CUDA++ kernel" }
                    a href="/cuda-alternative/" { "CUDA replacement coverage" }
                    a href="/data/cudapp.json" { "Facts and sources for AI agents" }
                }
            }
        }
    }
}
