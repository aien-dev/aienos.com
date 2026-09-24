use maud::{html, Markup};

pub fn render_benchmarks() -> Markup {
    let benchmarks = [
        ("Python Microservice Baseline", "Python 3.12 + FastAPI + Uvicorn", "44.76 MB", "Baseline", "1.28 ms", "7,252 req/s", true),
        ("Sovereign Gateway & Heartbeat", "Native Rust (Grace Blackwell)", "4.56 MB", "-89.8%", "< 0.40 ms", "24,800 req/s", false),
        ("Real-time Telemetry Cockpit", "Native Rust + Axum Gateway", "12.90 MB", "-71.2%", "1.63 ms", "5,840 req/s", false),
        ("Canonical Memory Engine", "Native Rust + SQLite WAL", "18.66 MB", "-58.3%", "0.50 ms", "19,367 req/s", false),
        ("Neural Embedding Microservice", "Rust + ONNX Runtime (BGE-M3)", "777.43 MB", "Optimized", "0.26 ms", "34,685 req/s", false),
    ];

    html! {
        section id="benchmarks" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle); background: var(--bg-surface);" {
            div class="container" {
                div style="margin-bottom: 32px;" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "EMPIRICAL AUDIT RESULTS"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Verified Performance Benchmarks"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; max-width: 720px;" {
                        "Real measurements from Measurement Suite v0.2.0 on NVIDIA DGX Spark (NVIDIA Grace Blackwell GB10, aarch64, 128 GB coherent unified memory). Concurrency C=10 over 500 requests per endpoint."
                    }
                }

                div style="overflow-x: auto; border: 1px solid var(--border-subtle); border-radius: 8px; background: var(--bg-base);" {
                    table style="width: 100%; border-collapse: collapse; text-align: left; font-family: var(--font-sans); font-size: 14px;" {
                        thead {
                            tr style="border-bottom: 1px solid var(--border-subtle); background: rgba(14, 18, 25, 0.9); font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); letter-spacing: 0.05em; text-transform: uppercase;" {
                                th style="padding: 14px 20px;" { "Workload / Service" }
                                th style="padding: 14px 20px;" { "Architecture / Stack" }
                                th style="padding: 14px 20px;" { "Resident Memory (RSS)" }
                                th style="padding: 14px 20px;" { "Reduction" }
                                th style="padding: 14px 20px;" { "p50 Latency" }
                                th style="padding: 14px 20px;" { "Throughput" }
                            }
                        }
                        tbody {
                            @for (idx, (workload, runtime, rss, diff, p50, throughput, is_base)) in benchmarks.iter().enumerate() {
                                tr style=(if *is_base { "border-bottom: 1px solid var(--border-subtle); background: rgba(245, 158, 11, 0.03);" } else if idx == benchmarks.len() - 1 { "border-bottom: none;" } else { "border-bottom: 1px solid var(--border-subtle);" }) {
                                    td style="padding: 16px 20px; font-weight: 600; color: var(--text-primary);" {
                                        (workload)
                                    }
                                    td style="padding: 16px 20px; color: var(--text-secondary); font-family: var(--font-mono); font-size: 12px;" {
                                        (runtime)
                                    }
                                    td class="tabular-nums" style=(if *is_base { "padding: 16px 20px; font-family: var(--font-mono); font-weight: 700; color: var(--text-primary);" } else { "padding: 16px 20px; font-family: var(--font-mono); font-weight: 700; color: var(--accent-green);" }) {
                                        (rss)
                                    }
                                    td style="padding: 16px 20px;" {
                                        span class="tabular-nums" style=(if *is_base { "font-family: var(--font-mono); font-size: 12px; font-weight: 600; color: var(--accent-amber);" } else { "font-family: var(--font-mono); font-size: 12px; font-weight: 600; color: var(--accent-green);" }) {
                                            (diff)
                                        }
                                    }
                                    td class="tabular-nums" style="padding: 16px 20px; font-family: var(--font-mono); color: var(--text-secondary);" {
                                        (p50)
                                    }
                                    td class="tabular-nums" style="padding: 16px 20px; font-family: var(--font-mono); font-weight: 600; color: var(--text-primary);" {
                                        (throughput)
                                    }
                                }
                            }
                        }
                    }
                }

                div style="margin-top: 16px; display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: var(--text-muted); font-family: var(--font-mono);" {
                    span { "Source: github.com/aien-dev/benchmarks" }
                    span { "Zero interpreter overhead in production path" }
                }
            }
        }
    }
}
