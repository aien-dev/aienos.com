import React from "react";
import { Cpu, ShieldCheck, Zap, HardDrive, Terminal, GitMerge } from "lucide-react";

export const ArchitectureMatrix: React.FC = () => {
  const pillars = [
    {
      icon: <Cpu size={24} color="var(--accent-blue)" />,
      title: "Coherent Unified Address Space",
      desc: "Grace Blackwell GB10 pairs 20 ARM Cortex cores with Blackwell GPU silicon across 128 GB coherent LPDDR5x memory. Tensors and page tables are manipulated in-place with zero host-to-device PCIe copy penalty."
    },
    {
      icon: <Zap size={24} color="var(--accent-green)" />,
      title: "Pure Native Compiled Systems",
      desc: "Zero Python or Node interpreters in core agent services or background daemons. Rust handles high-throughput asynchronous networking and process isolation; Mojo handles accelerated SIMD mathematics and PagedAttention KV-caching."
    },
    {
      icon: <ShieldCheck size={24} color="var(--accent-amber)" />,
      title: "Hardware TPM 2.0 Key Vault",
      desc: "Strict SECURE_TPM_ONLY policy bound to /dev/tpmrm0 ECDSA P-256 hardware. Plaintext .env secret files are strictly forbidden on disk. Keys resolve dynamically in-memory with real-time redaction interceptors."
    },
    {
      icon: <HardDrive size={24} color="var(--accent-blue)" />,
      title: "Canonical Memory Engine",
      desc: "High-performance SQLite WAL storage engine operating at 0.50 ms p50 latency and 19,000 req/s. Retains verified operational post-mortems and structured lessons without memory bloat."
    },
    {
      icon: <Terminal size={24} color="var(--accent-green)" />,
      title: "Neural Syscall Dispatcher",
      desc: "Replaces legacy POSIX interruptions with semantic dispatch: sys_telemetry, sys_alloc_kv, sys_simd_benchmark, and sys_vault, routing context directly to hardware-resident cognitive models."
    },
    {
      icon: <GitMerge size={24} color="var(--accent-blue)" />,
      title: "Sovereign Dual-Remote Git Sync",
      desc: "Autonomous change-management and recursive self-improvement pipelines sync commits to both public remotes and self-hosted Forgejo repositories, ensuring complete engineering sovereignty."
    }
  ];

  return (
    <section id="architecture" style={{ padding: "80px 0", borderBottom: "1px solid var(--border-subtle)" }}>
      <div className="container">
        <div style={{ marginBottom: "48px" }}>
          <div className="badge badge-blue" style={{ marginBottom: "12px" }}>
            <Cpu size={12} />
            <span>HARDWARE-GROUNDED DESIGN</span>
          </div>
          <h2 style={{ fontSize: "32px", fontWeight: 700, letterSpacing: "-0.02em", color: "var(--text-primary)" }}>
            Architecture: Beyond the von Neumann Bottleneck
          </h2>
          <p style={{ color: "var(--text-secondary)", marginTop: "8px", maxWidth: "700px" }}>
            Engineered for physical hardware reality. High-efficiency systems design removing four decades of legacy computing debt.
          </p>
        </div>

        <div className="responsive-card-grid" style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))",
          gap: "24px"
        }}>
          {pillars.map((item, idx) => (
            <div
              key={idx}
              className="glow-box"
              style={{
                padding: "28px",
                borderRadius: "8px",
                display: "flex",
                flexDirection: "column",
                gap: "16px"
              }}
            >
              <div style={{
                width: "44px",
                height: "44px",
                borderRadius: "6px",
                background: "var(--bg-surface)",
                border: "1px solid var(--border-subtle)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center"
              }}>
                {item.icon}
              </div>
              <h3 style={{ fontSize: "18px", fontWeight: 700, color: "var(--text-primary)" }}>
                {item.title}
              </h3>
              <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.6 }}>
                {item.desc}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
