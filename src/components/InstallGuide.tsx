import React, { useState } from "react";
import { Download, Copy, Check } from "lucide-react";

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
      os: "Linux and macOS",
      cmd: "curl -fsSL https://aienos.com/install.sh | bash",
      desc: "Clones the source, detects your platform, and builds release binaries. The script does not verify signatures yet."
    },
    {
      os: "Windows (PowerShell)",
      cmd: "git clone --depth 1 https://github.com/aien-dev/aien-sovereign-core.git; cd aien-sovereign-core; .\\install.ps1",
      desc: "Checks out the source, configures the local runtime, and compiles native sovereign binaries."
    },
    {
      os: "Build from source (Cargo)",
      cmd: "git clone https://github.com/aien-dev/aien-sovereign-core.git && cd aien-sovereign-core && cargo build --release",
      desc: "Inspect and compile everything yourself. Requires Rust 1.85 or newer."
    }
  ];

  return (
    <section id="install" className="section">
      <div className="container">
        <div className="section-head">
          <div className="badge badge-blue">
            <Download size={12} />
            <span>INSTALL</span>
          </div>
          <h2>Installation</h2>
          <p>
            Every path builds from source. Signed releases with pinned, verified manifests are planned and not yet available.
            Then try <code style={{ color: "var(--text-primary)" }}>cargo test --workspace</code>.
          </p>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
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
              <div>
                <h3 style={{ fontWeight: 700, fontSize: "16px", color: "var(--text-primary)" }}>
                  {m.os}
                </h3>
                <p style={{ fontSize: "13px", color: "var(--text-secondary)", marginTop: "2px" }}>
                  {m.desc}
                </p>
              </div>

              <div className="install-method-command" style={{
                background: "var(--bg-base)",
                border: "1px solid var(--border-subtle)",
                borderRadius: "6px",
                padding: "12px 16px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "16px",
                fontFamily: "var(--font-mono)",
                fontSize: "13px"
              }}>
                <span style={{ color: "var(--text-primary)", overflowWrap: "anywhere", minWidth: 0 }}>
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
                    fontFamily: "var(--font-mono)",
                    flexShrink: 0
                  }}
                >
                  {copiedIdx === idx ? <Check size={12} /> : <Copy size={12} />}
                  <span>{copiedIdx === idx ? "COPIED" : "COPY"}</span>
                </button>
              </div>
            </div>
          ))}
        </div>

      </div>
    </section>
  );
};
