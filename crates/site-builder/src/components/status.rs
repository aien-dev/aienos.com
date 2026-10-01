use maud::{html, Markup};

/// The canonical public state of the project. This list is the single
/// source of truth: the homepage status section, /status.json, and
/// /llms.txt all render from it. Statuses are a small fixed set:
/// IMPLEMENTED, QUALIFIED, EXPERIMENTAL, BLOCKED, PLANNED.
/// Bump UPDATED when a state changes; states change when the evidence
/// changes, not when the copy does.
pub const UPDATED: &str = "2026-10-01";

pub struct LayerState {
    pub name: &'static str,
    pub status: &'static str,
    pub note: &'static str,
    pub receipt_label: Option<&'static str>,
    pub receipt_url: Option<&'static str>,
}

pub fn layer_states() -> Vec<LayerState> {
    vec![
        LayerState {
            name: "Omega foundation gates",
            status: "QUALIFIED",
            note: "Gates 1 through 14 closed with one combined admission receipt: 230 checks, 0 failures.",
            receipt_label: Some("aien-dev/omega · PR #111"),
            receipt_url: Some("https://github.com/aien-dev/omega/pull/111"),
        },
        LayerState {
            name: "Turing instrument",
            status: "QUALIFIED",
            note: "The frozen measurement profile (EXP-001 calibration) holds, and the TY-2 yield log re-verified at T = 2,559,679.825 bits. Qualified for the calibration scope; discovery claims are the separate experiment program below.",
            receipt_label: Some("aien-dev/omega · PR #88"),
            receipt_url: Some("https://github.com/aien-dev/omega/pull/88"),
        },
        LayerState {
            name: "Native C kernel",
            status: "IMPLEMENTED",
            note: "Freestanding C kernel on main. Boots via UEFI in emulation, with NVMe, ARGUS capability checks, and the sealed Store in the boot path.",
            receipt_label: Some("aien-dev/aienos · PR #194"),
            receipt_url: Some("https://github.com/aien-dev/aienos/pull/194"),
        },
        LayerState {
            name: "Native machine qualification (M5)",
            status: "BLOCKED",
            note: "First qualification receipt: NOT QUALIFIED, 10 blocked, 6 missing. Published, and being worked down in public.",
            receipt_label: Some("aien-dev/aienos · PR #187"),
            receipt_url: Some("https://github.com/aien-dev/aienos/pull/187"),
        },
        LayerState {
            name: "Turing experiments (EXP-001 / 002 / 003)",
            status: "EXPERIMENTAL",
            note: "Validation runs under frozen, published protocols. Milestones and failures both get receipts.",
            receipt_label: Some("aien-dev/omega · PR #119"),
            receipt_url: Some("https://github.com/aien-dev/omega/pull/119"),
        },
        LayerState {
            name: "AIEN, the persistent agent",
            status: "EXPERIMENTAL",
            note: "Runs in the lab on the DGX Spark. The learning loop, learning from verified experience, is the active build.",
            receipt_label: Some("aien-dev/aienos"),
            receipt_url: Some("https://github.com/aien-dev/aienos"),
        },
        LayerState {
            name: "Atlas, the awakening layer",
            status: "PLANNED",
            note: "The seed that wakes AIENOS on new hardware. Doctrine published in the architecture repository; implementation ahead.",
            receipt_label: Some("aien-dev/aien-architecture"),
            receipt_url: Some("https://github.com/aien-dev/aien-architecture"),
        },
        LayerState {
            name: "Public release",
            status: "PLANNED",
            note: "Pre-alpha. No supported release or installer yet. When there is one, it ships with versions, hashes, and signatures, inspectable before it runs.",
            receipt_label: None,
            receipt_url: None,
        },
    ]
}

fn chip_style(status: &str) -> &'static str {
    match status {
        "QUALIFIED" => "border: 1px solid rgba(0, 229, 153, 0.4); color: var(--accent-green); background: rgba(0, 229, 153, 0.08);",
        "IMPLEMENTED" => "border: 1px solid rgba(45, 127, 249, 0.4); color: var(--accent-blue); background: rgba(45, 127, 249, 0.08);",
        "EXPERIMENTAL" => "border: 1px solid rgba(245, 158, 11, 0.4); color: #f5a623; background: rgba(245, 158, 11, 0.08);",
        "BLOCKED" => "border: 1px solid rgba(248, 113, 113, 0.4); color: #f87171; background: rgba(248, 113, 113, 0.08);",
        _ => "border: 1px solid var(--border-subtle); color: var(--text-muted); background: var(--bg-surface);",
    }
}

pub fn render_status() -> Markup {
    let layers = layer_states();

    html! {
        section id="status" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="max-width: 760px; margin-bottom: 36px;" {
                    div class="badge badge-amber" style="margin-bottom: 12px;" {
                        "CANONICAL PROJECT STATE"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Where it honestly stands"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; line-height: 1.6;" {
                        "One state of the project, published once and consumed everywhere. This section, "
                        a href="/status.json" style="color: var(--accent-green); font-weight: 600;" { "status.json" }
                        ", and "
                        a href="/llms.txt" style="color: var(--accent-green); font-weight: 600;" { "llms.txt" }
                        " all render from the same source, so they cannot drift apart. Last updated " (UPDATED) "."
                    }
                }

                div style="display: flex; flex-direction: column;" {
                    @for layer in &layers {
                        div style="display: flex; gap: 20px; padding: 18px 0; border-top: 1px solid var(--border-subtle); align-items: flex-start; flex-wrap: wrap;" {
                            span class="badge" style=(format!("font-size: 10px; min-width: 118px; justify-content: center; {}", chip_style(layer.status))) {
                                (layer.status)
                            }
                            div style="flex: 1; min-width: 260px;" {
                                div style="font-weight: 700; color: var(--text-primary); font-size: 16px;" {
                                    (layer.name)
                                }
                                p style="color: var(--text-secondary); font-size: 14.5px; line-height: 1.6; margin: 4px 0 0 0;" {
                                    (layer.note)
                                }
                                @if let (Some(label), Some(url)) = (layer.receipt_label, layer.receipt_url) {
                                    div style="margin-top: 6px; font-family: var(--font-mono); font-size: 12px;" {
                                        a href=(url) target="_blank" rel="noopener noreferrer" style="color: var(--accent-green); font-weight: 600;" {
                                            "RECEIPT: " (label) " →"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div style="display: flex; gap: 12px; flex-wrap: wrap; margin-top: 32px;" {
                    a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="background: var(--accent-green); color: var(--bg-base); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 700;" {
                        "Follow the work on GitHub"
                    }
                    a href="#research" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                        "Read the research"
                    }
                    a href="#waitlist" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 10px 20px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                        "Join the waitlist"
                    }
                }
            }
        }
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// The same canonical state as /status.json, for machines, badges,
/// external agents, and eventually AIEN itself.
pub fn status_json() -> String {
    let layers = layer_states();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"project\": \"AIENOS\",\n");
    out.push_str(&format!("  \"updated\": \"{}\",\n", UPDATED));
    out.push_str("  \"canonical\": \"https://www.aienos.com/#status\",\n");
    out.push_str("  \"statuses\": [\"IMPLEMENTED\", \"QUALIFIED\", \"EXPERIMENTAL\", \"BLOCKED\", \"PLANNED\"],\n");
    out.push_str("  \"layers\": [\n");
    for (i, layer) in layers.iter().enumerate() {
        let receipt = match layer.receipt_url {
            Some(url) => format!("\"{}\"", json_escape(url)),
            None => "null".to_string(),
        };
        let comma = if i + 1 < layers.len() { "," } else { "" };
        out.push_str(&format!(
            "    {{\"name\": \"{}\", \"status\": \"{}\", \"note\": \"{}\", \"receipt\": {}}}{}\n",
            json_escape(layer.name),
            layer.status,
            json_escape(layer.note),
            receipt,
            comma
        ));
    }
    out.push_str("  ]\n}\n");
    out
}

/// /llms.txt: the project in plain text for AI readers and crawlers.
pub fn llms_txt() -> String {
    let mut out = String::new();
    out.push_str("# AIENOS\n\n");
    out.push_str("> AIENOS is an experimental sovereign machine for persistent machine intelligence: Atlas awakens it on hardware you own, Omega turns intent into verified programs, and AIEN learns from verified experience. Intelligence should be property, not rent. Every claim carries a public receipt.\n\n");
    out.push_str("AIENOS is more than an operating system. It is a substrate where a persistent intelligence lives on the owner's hardware, keeps its memory and identity on the owner's side of the line, and treats hardware as capability, not identity. Status: experimental, pre-alpha.\n\n");
    out.push_str("## The lineage\n\n");
    out.push_str("- Atlas: awakens. The seed that wakes AIENOS on a machine.\n");
    out.push_str("- AIENOS: persists. The sovereign substrate that owns the hardware and keeps the agent alive and continuous.\n");
    out.push_str("- Omega: defines, synthesizes, verifies. Turns intent into programs and proves them against reality before they are trusted.\n");
    out.push_str("- AIEN: learns. The persistent intelligence. Weights suggest, programs explain, evidence teaches.\n");
    out.push_str("- Around the loop, not in the pipeline: Aegis verifies generated code and changes nothing; Forge realizes what Omega verified; Argus watches capability enforcement; Cortex is the canonical memory.\n\n");
    out.push_str("## Current state\n\n");
    for layer in layer_states() {
        out.push_str(&format!("- {}: {}. {}\n", layer.name, layer.status, layer.note));
    }
    out.push_str("\nMachine-readable state: https://www.aienos.com/status.json\n\n");
    out.push_str("## Links\n\n");
    out.push_str("- Homepage: https://www.aienos.com/\n");
    out.push_str("- Current status: https://www.aienos.com/#status\n");
    out.push_str("- Evidence: https://www.aienos.com/#evidence\n");
    out.push_str("- Research index: https://www.aienos.com/research/\n");
    out.push_str("- The Turing, the unit of machine understanding: https://www.aienos.com/turing/\n");
    out.push_str("- Licensing (Apache-2.0 WITH LLVM-exception): https://www.aienos.com/licensing/\n");
    out.push_str("- GitHub organization: https://github.com/aien-dev\n");
    out
}
