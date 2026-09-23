import React from "react";
import { Cpu, Terminal, HardDrive, Layers, GitBranch, Plug, FlaskConical } from "lucide-react";

export const ArchitectureMatrix: React.FC = () => {
  const pillars = [
    {
      icon: <Terminal size={22} color="var(--accent-green)" />,
      crate: "aien-cli",
      title: "Agent CLI and Orchestrator",
      desc: "Terminal runtime with an initiation sequence, a fail-closed policy engine, lifecycle hooks, and context compaction."
    },
    {
      icon: <HardDrive size={22} color="var(--accent-blue)" />,
      crate: "cortex-rs",
      title: "Persistent Memory",
      desc: "Knowledge store with SQLite persistence, plus cortex-encoder-rs, an ONNX embedding service for vector recall."
    },
    {
      icon: <Layers size={22} color="var(--accent-green)" />,
      crate: "aien-kv-cache",
      title: "Unified-Memory KV-Cache",
      desc: "Physical page pool on Grace Blackwell unified memory with copy-on-write branch forking for parallel reasoning paths."
    },
    {
      icon: <GitBranch size={22} color="var(--accent-blue)" />,
      crate: "aien-scheduler",
      title: "Continuous Batching",
      desc: "Continuous batching with chunked prefill, and the bench_inference_stack binary that produces its measurements."
    },
    {
      icon: <Cpu size={22} color="var(--accent-green)" />,
      crate: "aien-inference-abi",
      title: "Inference ABI",
      desc: "Tensor backend trait with Blackwell GB10 and CPU reference paths, plus request and event contracts in aien-inference-service."
    },
    {
      icon: <Plug size={22} color="var(--accent-blue)" />,
      crate: "spark-max-cabi",
      title: "Modular MAX Bridge",
      desc: "C-ABI bridge between Mojo kernels and Rust, including Qwen FP8 mixture-of-experts execution work."
    }
  ];

  return (
    <section id="architecture" className="section">
      <div className="container">
        <div className="section-head">
          <div className="badge badge-blue">
            <Cpu size={12} />
            <span>WHAT IS IMPLEMENTED TODAY</span>
          </div>
          <h2>Architecture</h2>
          <p>
            Each component below has tests or runnable targets in{" "}
            <a href="https://github.com/aien-dev/aien-sovereign-core" target="_blank" rel="noopener noreferrer">aien-sovereign-core</a>.
            Execution is hybrid CPU and GPU, not exclusively accelerated.
          </p>
        </div>

        <div className="card-grid">
          {pillars.map((item) => (
            <div
              key={item.crate}
              className="glow-box"
              style={{
                padding: "24px",
                borderRadius: "8px",
                display: "flex",
                flexDirection: "column",
                gap: "12px"
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "12px" }}>
                <div style={{
                  width: "40px",
                  height: "40px",
                  borderRadius: "6px",
                  background: "var(--bg-surface)",
                  border: "1px solid var(--border-subtle)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  flexShrink: 0
                }}>
                  {item.icon}
                </div>
                <code style={{ fontSize: "11px", color: "var(--text-muted)" }}>crates/{item.crate}</code>
              </div>
              <h3 style={{ fontSize: "17px", fontWeight: 700, color: "var(--text-primary)" }}>
                {item.title}
              </h3>
              <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.6 }}>
                {item.desc}
              </p>
            </div>
          ))}
        </div>

        <div style={{
          marginTop: "24px",
          padding: "16px 20px",
          borderRadius: "8px",
          border: "1px dashed var(--border-subtle)",
          display: "flex",
          gap: "12px",
          alignItems: "flex-start",
          fontSize: "14px",
          color: "var(--text-secondary)"
        }}>
          <FlaskConical size={18} color="var(--accent-amber)" style={{ flexShrink: 0, marginTop: "2px" }} />
          <span>
            <strong style={{ color: "var(--text-primary)" }}>Experimental, not yet core:</strong> AEGIS capability boundary,
            MCP broker integration, supervisor, debugger, cockpit gateway, and the distillation pipeline. Research concepts
            such as recursive self-improvement live in roadmap documents.
          </span>
        </div>
      </div>
    </section>
  );
};
