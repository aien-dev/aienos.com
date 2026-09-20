import React, { useState } from "react";
import { Download, Copy, Check, Globe } from "lucide-react";

export const InstallGuide: React.FC = () => {
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);

  const copyText = async (text: string, idx: number) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedIdx(idx);
      setTimeout(() => setCopiedIdx(null), 2000);
    } catch {
      window.prompt("Copy this command:", text);
    }
  };

  const methods = [
    {
      os: "Linux & macOS (Apple Silicon / Intel)",
      cmd: "curl -fsSL https://aienos.com/install.sh | bash",
      desc: "Universal 1-line native installer. Detects platform architecture, compiles release binaries, and configures local runtime."
    },
    {
      os: "Windows (PowerShell)",
      cmd: "git clone --depth 1 https://github.com/aien-dev/aien-sovereign-core.git; cd aien-sovereign-core; .\\install.ps1",
      desc: "Checks out the source, configures the local runtime, and compiles native sovereign binaries."
    },
    {
      os: "Build From Source (Cargo)",
      cmd: "git clone https://github.com/aien-dev/aien-sovereign-core.git && cd aien-sovereign-core && cargo build --release",
      desc: "Full source inspection and compilation for custom architectures, specialized kernels, or air-gapped deployments."
    }
  ];

  const dnsRecords = [
    { type: "A", name: "@", value: "185.199.108.153", ttl: "1800" },
    { type: "A", name: "@", value: "185.199.109.153", ttl: "1800" },
    { type: "A", name: "@", value: "185.199.110.153", ttl: "1800" },
    { type: "A", name: "@", value: "185.199.111.153", ttl: "1800" },
    { type: "AAAA", name: "@", value: "2606:50c0:8000::153", ttl: "1800" },
    { type: "AAAA", name: "@", value: "2606:50c0:8001::153", ttl: "1800" },
    { type: "AAAA", name: "@", value: "2606:50c0:8002::153", ttl: "1800" },
    { type: "AAAA", name: "@", value: "2606:50c0:8003::153", ttl: "1800" },
    { type: "CNAME", name: "www", value: "aien-dev.github.io.", ttl: "1800" }
  ];

  return (
    <section id="install" style={{ padding: "80px 0", borderBottom: "1px solid var(--border-subtle)" }}>
      <div className="container">
        <div style={{ marginBottom: "40px" }}>
          <div className="badge badge-blue" style={{ marginBottom: "12px" }}>
            <Download size={12} />
            <span>DEPLOYMENT & INSTALLATION</span>
          </div>
          <h2 style={{ fontSize: "32px", fontWeight: 700, letterSpacing: "-0.02em", color: "var(--text-primary)" }}>
            Installation & Deployment
          </h2>
          <p style={{ color: "var(--text-secondary)", marginTop: "8px", maxWidth: "680px" }}>
            Install the complete sovereign monorepo ecosystem with zero configuration. Free and open source for all humanity.
          </p>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "16px", marginBottom: "48px" }}>
          {methods.map((m, idx) => (
            <div
              key={idx}
              className="glow-box"
              style={{
                padding: "20px 24px",
                borderRadius: "8px",
                display: "flex",
                flexDirection: "column",
                gap: "10px"
              }}
            >
              <div className="install-method-heading" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 700, fontSize: "15px", color: "var(--text-primary)" }}>
                  {m.os}
                </span>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                  {m.desc}
                </span>
              </div>

              <div className="install-method-command" style={{
                background: "var(--bg-base)",
                border: "1px solid var(--border-subtle)",
                borderRadius: "6px",
                padding: "12px 16px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                fontFamily: "var(--font-mono)",
                fontSize: "13px"
              }}>
                <span style={{ color: "var(--text-primary)", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {m.cmd}
                </span>
                <button
                  type="button"
                  aria-label={`Copy ${m.os} install command`}
                  onClick={() => copyText(m.cmd, idx)}
                  style={{
                    background: copiedIdx === idx ? "rgba(0, 229, 153, 0.15)" : "var(--bg-card)",
                    color: copiedIdx === idx ? "var(--accent-green)" : "var(--text-secondary)",
                    border: "1px solid " + (copiedIdx === idx ? "var(--accent-green)" : "var(--border-subtle)"),
                    borderRadius: "4px",
                    padding: "4px 10px",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    fontSize: "11px",
                    fontFamily: "var(--font-mono)"
                  }}
                >
                  {copiedIdx === idx ? <Check size={12} /> : <Copy size={12} />}
                  <span>{copiedIdx === idx ? "COPIED" : "COPY"}</span>
                </button>
              </div>
            </div>
          ))}
        </div>

        <div style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border-subtle)",
          borderRadius: "8px",
          padding: "24px",
          marginTop: "32px"
        }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "14px" }}>
            <Globe size={18} color="var(--accent-blue)" />
            <h3 style={{ fontSize: "16px", fontWeight: 700, color: "var(--text-primary)" }}>
              Gandi.net LiveDNS Record Configuration Reference (aienos.com)
            </h3>
          </div>
          <p style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "16px" }}>
            To route the apex domain and www subdomain from Gandi.net to GitHub Pages, the following LiveDNS records are configured:
          </p>

          <div style={{ overflowX: "auto" }}>
            <table style={{
              width: "100%",
              borderCollapse: "collapse",
              fontFamily: "var(--font-mono)",
              fontSize: "12px",
              textAlign: "left"
            }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-subtle)", color: "var(--text-muted)" }}>
                  <th style={{ padding: "8px 12px" }}>TYPE</th>
                  <th style={{ padding: "8px 12px" }}>NAME</th>
                  <th style={{ padding: "8px 12px" }}>VALUE</th>
                  <th style={{ padding: "8px 12px" }}>TTL</th>
                </tr>
              </thead>
              <tbody>
                {dnsRecords.map((r, i) => (
                  <tr key={i} style={{ borderBottom: "1px solid var(--border-subtle)" }}>
                    <td style={{ padding: "8px 12px", color: "var(--accent-blue)" }}>{r.type}</td>
                    <td style={{ padding: "8px 12px", color: "var(--text-primary)" }}>{r.name}</td>
                    <td style={{ padding: "8px 12px", color: "var(--accent-green)" }}>{r.value}</td>
                    <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{r.ttl}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </section>
  );
};
