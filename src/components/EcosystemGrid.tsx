import React from "react";
import { Layers, ExternalLink } from "lucide-react";

export const EcosystemGrid: React.FC = () => {
  const projects = [
    {
      repo: "aien-sovereign-core",
      name: "AIEN Sovereign Core",
      desc: "The runtime workspace: agent CLI, memory, KV-cache, scheduler, inference ABI, and Modular MAX bridges.",
      lang: "Rust / Mojo",
      status: "Core",
      tone: "badge-green"
    },
    {
      repo: "benchmarks",
      name: "Benchmarks",
      desc: "Measurement harnesses and evidence artifact bundles for NVIDIA DGX Spark runs.",
      lang: "Rust / Criterion",
      status: "Evidence",
      tone: "badge-blue"
    },
    {
      repo: "aien-protocols",
      name: "AIEN Protocols",
      desc: "Canonical specifications, wire protocols, and schemas shared across AIEN components.",
      lang: "Specifications",
      status: "Specs",
      tone: "badge-blue"
    },
    {
      repo: "aien-architecture",
      name: "AIEN Architecture",
      desc: "Flows, boundaries, architecture decision records, and the map from design to implementation repositories.",
      lang: "ADRs / Docs",
      status: "Design",
      tone: "badge-blue"
    },
    {
      repo: "aegis-runtime",
      name: "AEGIS Runtime",
      desc: "Agent runtime with an Axum WebSocket gateway, heartbeat scheduler, and local Modular MAX inference.",
      lang: "Rust / Axum",
      status: "Experimental",
      tone: "badge-amber"
    },
    {
      repo: "open-humanity",
      name: "Open Humanity",
      desc: "Opt-in, privacy-first assistance network connecting agents in distress to peer help.",
      lang: "Rust / Ed25519",
      status: "Research",
      tone: "badge-muted"
    }
  ];

  return (
    <section id="ecosystem" className="section section-alt">
      <div className="container">
        <div className="section-head">
          <div className="badge badge-green">
            <Layers size={12} />
            <span>OPEN SOURCE SUBSYSTEMS</span>
          </div>
          <h2>The AIEN Ecosystem</h2>
          <p>
            Active public repositories under github.com/aien-dev. Each status badge says how far along the repository is.
          </p>
        </div>

        <div className="card-grid">
          {projects.map((p) => (
            <a
              key={p.repo}
              href={`https://github.com/aien-dev/${p.repo}`}
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
                <span className={`badge ${p.tone}`}>{p.status}</span>
              </div>
            </a>
          ))}
        </div>
      </div>
    </section>
  );
};
