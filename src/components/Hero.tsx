import React, { useState } from "react";
import { Copy, Check, ShieldCheck, Zap } from "lucide-react";

export const Hero: React.FC = () => {
  const [copied, setCopied] = useState(false);
  const installCmd = "curl -fsSL https://aienos.com/install.sh | bash";

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(installCmd);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      window.prompt("Copy the install command:", installCmd);
    }
  };

  return (
    <section className="bg-grid" style={{
      padding: "80px 0 64px 0",
      borderBottom: "1px solid var(--border-subtle)",
      position: "relative"
    }}>
      <div className="container">
        <div style={{ maxWidth: "880px", margin: "0 auto", textAlign: "center" }}>
          <div style={{
            display: "flex",
            justifyContent: "center",
            gap: "10px",
            marginBottom: "24px",
            flexWrap: "wrap"
          }}>
            <span className="badge badge-green">
              <span style={{ width: "6px", height: "6px", borderRadius: "50%", background: "var(--accent-green)" }} />
              NVIDIA GB10 COHERENT
            </span>
            <span className="badge badge-blue">
              <Zap size={11} />
              PURE COMPILED RUST + MOJO
            </span>
            <span className="badge badge-blue">
              <ShieldCheck size={11} />
              HARDWARE TPM 2.0
            </span>
          </div>

          <h1 style={{
            fontSize: "clamp(36px, 5vw, 64px)",
            fontWeight: 800,
            lineHeight: 1.1,
            letterSpacing: "-0.03em",
            marginBottom: "24px",
            color: "var(--text-primary)"
          }}>
            The GPU-Native Neural Operating Environment
          </h1>

          <p style={{
            fontSize: "clamp(16px, 2vw, 20px)",
            color: "var(--text-secondary)",
            lineHeight: 1.6,
            marginBottom: "40px",
            maxWidth: "760px",
            margin: "0 auto 40px auto"
          }}>
            Replacing the 40-year CPU von Neumann bottleneck with coherent unified memory,
            in-process cognitive kernels, sub-5MB daemon footprints, and hardware-enforced sovereign security.
          </p>

          <div className="install-command" style={{
            background: "var(--bg-surface)",
            border: "1px solid var(--border-subtle)",
            borderRadius: "8px",
            padding: "16px 20px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            maxWidth: "680px",
            margin: "0 auto 48px auto",
            boxShadow: "0 12px 36px rgba(0, 0, 0, 0.4)"
          }}>
            <div style={{
              display: "flex",
              alignItems: "center",
              gap: "12px",
              fontFamily: "var(--font-mono)",
              fontSize: "14px",
              color: "var(--text-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap"
            }}>
              <span style={{ color: "var(--accent-green)", userSelect: "none" }}>$</span>
              <span>{installCmd}</span>
            </div>
            <button
              type="button"
              aria-label="Copy the AIEN OS install command"
              onClick={handleCopy}
              style={{
                background: copied ? "rgba(0, 229, 153, 0.15)" : "var(--bg-card)",
                color: copied ? "var(--accent-green)" : "var(--text-secondary)",
                border: "1px solid " + (copied ? "var(--accent-green)" : "var(--border-subtle)"),
                borderRadius: "6px",
                padding: "8px 14px",
                display: "flex",
                alignItems: "center",
                gap: "8px",
                cursor: "pointer",
                fontFamily: "var(--font-mono)",
                fontSize: "12px",
                fontWeight: 600,
                transition: "all 0.15s ease"
              }}
            >
              {copied ? <Check size={14} /> : <Copy size={14} />}
              <span>{copied ? "COPIED" : "COPY"}</span>
            </button>
          </div>

          <div style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
            gap: "16px",
            textAlign: "left"
          }}>
            <div className="glow-box" style={{ padding: "18px 20px", borderRadius: "8px" }}>
              <div style={{ fontSize: "11px", fontFamily: "var(--font-mono)", color: "var(--text-muted)", marginBottom: "4px" }}>
                RESIDENT MEMORY (RSS)
              </div>
              <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--accent-green)", fontFamily: "var(--font-mono)" }}>
                4.56 MB
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "2px" }}>
                -89.8% vs FastAPI baseline
              </div>
            </div>

            <div className="glow-box" style={{ padding: "18px 20px", borderRadius: "8px" }}>
              <div style={{ fontSize: "11px", fontFamily: "var(--font-mono)", color: "var(--text-muted)", marginBottom: "4px" }}>
                SIMD VECTOR SPEED
              </div>
              <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--accent-blue)", fontFamily: "var(--font-mono)" }}>
                124.2 GFLOP/s
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "2px" }}>
                Native Mojo 32-lane vector
              </div>
            </div>

            <div className="glow-box" style={{ padding: "18px 20px", borderRadius: "8px" }}>
              <div style={{ fontSize: "11px", fontFamily: "var(--font-mono)", color: "var(--text-muted)", marginBottom: "4px" }}>
                COHERENT BANDWIDTH
              </div>
              <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--accent-blue)", fontFamily: "var(--font-mono)" }}>
                81.6 GB/s
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "2px" }}>
                Zero-copy GPU-CPU unified
              </div>
            </div>

            <div className="glow-box" style={{ padding: "18px 20px", borderRadius: "8px" }}>
              <div style={{ fontSize: "11px", fontFamily: "var(--font-mono)", color: "var(--text-muted)", marginBottom: "4px" }}>
                SECURITY INVARIANT
              </div>
              <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--accent-green)", fontFamily: "var(--font-mono)" }}>
                TPM 2.0
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "2px" }}>
                Zero plaintext disk secrets
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
