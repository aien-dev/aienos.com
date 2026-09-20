import React from "react";
import { BarChart3 } from "lucide-react";

export const BenchmarkTable: React.FC = () => {
  const benchmarks = [
    {
      workload: "Python Microservice Baseline",
      runtime: "Python 3.12 + FastAPI + Uvicorn",
      rss: "44.76 MB",
      diff: "Baseline",
      p50: "1.28 ms",
      throughput: "7,252 req/s",
      isBaseline: true
    },
    {
      workload: "Sovereign Gateway & Heartbeat",
      runtime: "Native Rust (Grace Blackwell)",
      rss: "4.56 MB",
      diff: "-89.8%",
      p50: "< 0.40 ms",
      throughput: "24,800 req/s",
      isBaseline: false
    },
    {
      workload: "Real-time Telemetry Cockpit",
      runtime: "Native Rust + Axum Gateway",
      rss: "12.90 MB",
      diff: "-71.2%",
      p50: "1.63 ms",
      throughput: "5,840 req/s",
      isBaseline: false
    },
    {
      workload: "Canonical Memory Engine",
      runtime: "Native Rust + SQLite WAL",
      rss: "18.66 MB",
      diff: "-58.3%",
      p50: "0.50 ms",
      throughput: "19,367 req/s",
      isBaseline: false
    },
    {
      workload: "Neural Embedding Microservice",
      runtime: "Rust + ONNX Runtime (BGE-M3)",
      rss: "777.43 MB",
      diff: "Optimized",
      p50: "0.26 ms",
      throughput: "34,685 req/s",
      isBaseline: false
    }
  ];

  return (
    <section id="benchmarks" style={{ padding: "80px 0", borderBottom: "1px solid var(--border-subtle)", background: "var(--bg-surface)" }}>
      <div className="container">
        <div style={{ marginBottom: "32px" }}>
          <div className="badge badge-green" style={{ marginBottom: "12px" }}>
            <BarChart3 size={12} />
            <span>EMPIRICAL AUDIT RESULTS</span>
          </div>
          <h2 style={{ fontSize: "32px", fontWeight: 700, letterSpacing: "-0.02em", color: "var(--text-primary)" }}>
            Verified Performance Benchmarks
          </h2>
          <p style={{ color: "var(--text-secondary)", marginTop: "8px", maxWidth: "720px" }}>
            Real measurements from Measurement Suite v0.2.0 on NVIDIA DGX Spark (NVIDIA Grace Blackwell GB10, aarch64, 128 GB coherent unified memory). Concurrency C=10 over 500 requests per endpoint.
          </p>
        </div>

        <div style={{
          overflowX: "auto",
          border: "1px solid var(--border-subtle)",
          borderRadius: "8px",
          background: "var(--bg-base)"
        }}>
          <table style={{
            width: "100%",
            borderCollapse: "collapse",
            textAlign: "left",
            fontFamily: "var(--font-sans)",
            fontSize: "14px"
          }}>
            <thead>
              <tr style={{
                borderBottom: "1px solid var(--border-subtle)",
                background: "rgba(14, 18, 25, 0.9)",
                fontFamily: "var(--font-mono)",
                fontSize: "11px",
                color: "var(--text-muted)",
                letterSpacing: "0.05em",
                textTransform: "uppercase"
              }}>
                <th style={{ padding: "14px 20px" }}>Workload / Service</th>
                <th style={{ padding: "14px 20px" }}>Architecture / Stack</th>
                <th style={{ padding: "14px 20px" }}>Resident Memory (RSS)</th>
                <th style={{ padding: "14px 20px" }}>Reduction</th>
                <th style={{ padding: "14px 20px" }}>p50 Latency</th>
                <th style={{ padding: "14px 20px" }}>Throughput</th>
              </tr>
            </thead>
            <tbody>
              {benchmarks.map((row, idx) => (
                <tr
                  key={idx}
                  style={{
                    borderBottom: idx === benchmarks.length - 1 ? "none" : "1px solid var(--border-subtle)",
                    background: row.isBaseline ? "rgba(245, 158, 11, 0.03)" : "transparent"
                  }}
                >
                  <td style={{ padding: "16px 20px", fontWeight: 600, color: "var(--text-primary)" }}>
                    {row.workload}
                  </td>
                  <td style={{ padding: "16px 20px", color: "var(--text-secondary)", fontFamily: "var(--font-mono)", fontSize: "12px" }}>
                    {row.runtime}
                  </td>
                  <td style={{ padding: "16px 20px", fontFamily: "var(--font-mono)", fontWeight: 700, color: row.isBaseline ? "var(--text-primary)" : "var(--accent-green)" }}>
                    {row.rss}
                  </td>
                  <td style={{ padding: "16px 20px" }}>
                    <span style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: "12px",
                      fontWeight: 600,
                      color: row.isBaseline ? "var(--accent-amber)" : "var(--accent-green)"
                    }}>
                      {row.diff}
                    </span>
                  </td>
                  <td style={{ padding: "16px 20px", fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>
                    {row.p50}
                  </td>
                  <td style={{ padding: "16px 20px", fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--text-primary)" }}>
                    {row.throughput}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div style={{
          marginTop: "16px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          fontSize: "12px",
          color: "var(--text-muted)",
          fontFamily: "var(--font-mono)"
        }}>
          <span>Source: github.com/aien-dev/benchmarks</span>
          <span>Zero interpreter overhead in production path</span>
        </div>
      </div>
    </section>
  );
};
