import React from "react";
import { Layers, ExternalLink } from "lucide-react";

export const EcosystemGrid: React.FC = () => {
  const projects = [
    {
      repo: "aien-sovereign-core",
      name: "AIEN Sovereign Core",
      desc: "Core native agent runtime, fail-closed safety engine, Axum web gateway, and custom Modular MAX model architectures.",
      lang: "Rust 1.85+ / MAX 26.5",
      stars: "Core Engine",
      url: "https://github.com/aien-dev/aien-sovereign-core"
    },
    {
      repo: "spark-neural-os",
      name: "Spark Neural OS",
      desc: "GPU-native neural operating system, resident cognitive kernel, and pure Mojo 1.0 SIMD vector kernels.",
      lang: "Mojo 1.0 / MLIR",
      stars: "Kernel Stack",
      url: "https://github.com/aien-dev"
    },
    {
      repo: "spark-cockpit",
      name: "AIEN Universal Cockpit",
      desc: "Ultra-low-latency Axum HTTP/SSE gateway and PWA console delivering real-time telemetry, reasoning logs, and vault inspection.",
      lang: "Rust / Axum",
      stars: "12.9 MB RSS",
      url: "https://github.com/aien-dev"
    },
    {
      repo: "spark-rsi",
      name: "Spark RSI Engine",
      desc: "Recursive Self-Improvement engine with TPM ECDSA-signed cryptographic ledger (.rsi/ledger.db) and dual-jail container isolation.",
      lang: "Rust / TPM 2.0",
      stars: "Cryptographic",
      url: "https://github.com/aien-dev/spark-rsi"
    },
    {
      repo: "openclaw-rs",
      name: "OpenClaw Sovereign Host",
      desc: "Ultra-high-performance process supervisor and event dispatcher operating under 5MB resident set size.",
      lang: "Rust",
      stars: "4.56 MB RSS",
      url: "https://github.com/aien-dev/openclaw-rs"
    },
    {
      repo: "cortex-rs",
      name: "Cortex Canonical Memory",
      desc: "Canonical memory and vector storage engine with SQLite WAL persistence, sub-millisecond search, and graph traversal.",
      lang: "Rust / SQLite",
      stars: "19k req/s",
      url: "https://github.com/aien-dev/cortex-rs"
    }
  ];

  return (
    <section id="ecosystem" style={{ padding: "80px 0", borderBottom: "1px solid var(--border-subtle)", background: "var(--bg-surface)" }}>
      <div className="container">
        <div style={{ marginBottom: "48px" }}>
          <div className="badge badge-green" style={{ marginBottom: "12px" }}>
            <Layers size={12} />
            <span>OPEN SOURCE SUBSYSTEMS</span>
          </div>
          <h2 style={{ fontSize: "32px", fontWeight: 700, letterSpacing: "-0.02em", color: "var(--text-primary)" }}>
            The AIEN Sovereign Ecosystem
          </h2>
          <p style={{ color: "var(--text-secondary)", marginTop: "8px", maxWidth: "680px" }}>
            Modular, independently verifiable native components operating in concert. Licensed under Apache-2.0 and SRCL-1.0.
          </p>
        </div>

        <div className="responsive-card-grid" style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(340px, 1fr))",
          gap: "24px"
        }}>
          {projects.map((p, idx) => (
            <a
              key={idx}
              href={p.url}
              target="_blank"
              rel="noopener noreferrer"
              className="glow-box"
              style={{
                padding: "24px",
                borderRadius: "8px",
                display: "flex",
                flexDirection: "column",
                justifyContent: "space-between",
                background: "var(--bg-base)"
              }}
            >
              <div>
                <div style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  marginBottom: "12px"
                }}>
                  <span style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: "12px",
                    color: "var(--accent-blue)",
                    fontWeight: 600
                  }}>
                    {p.repo}
                  </span>
                  <ExternalLink size={14} color="var(--text-muted)" />
                </div>
                <h3 style={{ fontSize: "18px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "8px" }}>
                  {p.name}
                </h3>
                <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.5, marginBottom: "20px" }}>
                  {p.desc}
                </p>
              </div>

              <div style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                paddingTop: "14px",
                borderTop: "1px solid var(--border-subtle)",
                fontFamily: "var(--font-mono)",
                fontSize: "11px",
                color: "var(--text-muted)"
              }}>
                <span>{p.lang}</span>
                <span className="badge badge-blue">{p.stars}</span>
              </div>
            </a>
          ))}
        </div>
      </div>
    </section>
  );
};
