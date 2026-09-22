import React from "react";
import { Terminal } from "lucide-react";

export const Footer: React.FC = () => {
  return (
    <footer style={{
      borderTop: "1px solid var(--border-subtle)",
      background: "var(--bg-base)",
      padding: "48px 0 32px 0",
      fontSize: "13px",
      color: "var(--text-muted)"
    }}>
      <div className="container">
        <div style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          flexWrap: "wrap",
          gap: "32px",
          marginBottom: "40px"
        }}>
          <div style={{ maxWidth: "380px" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "12px" }}>
              <Terminal size={18} color="var(--accent-blue)" />
              <span style={{ fontWeight: 700, fontSize: "16px", color: "var(--text-primary)" }}>
                AIENOS
              </span>
            </div>
            <p style={{ lineHeight: 1.6, color: "var(--text-secondary)" }}>
              The GPU-Native Neural Operating Environment. Built for NVIDIA Grace Blackwell GB10, Apple Silicon, and commodity Linux architectures.
            </p>
          </div>

          <div style={{ display: "flex", gap: "48px", flexWrap: "wrap" }}>
            <div>
              <div style={{ fontWeight: 600, color: "var(--text-primary)", marginBottom: "12px", fontFamily: "var(--font-mono)", fontSize: "11px", letterSpacing: "0.05em" }}>
                ECOSYSTEM
              </div>
              <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: "8px" }}>
                <li><a href="https://github.com/aien-dev/aien-sovereign-core" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>AIEN Sovereign Core</a></li>
                <li><a href="https://github.com/aien-dev/spark-rsi" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>Spark RSI Engine</a></li>
                <li><a href="https://github.com/aien-dev/benchmarks" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>Performance Benchmarks</a></li>
                <li><a href="https://github.com/aien-dev/openclaw-rs" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>OpenClaw Supervisor</a></li>
              </ul>
            </div>

            <div>
              <div style={{ fontWeight: 600, color: "var(--text-primary)", marginBottom: "12px", fontFamily: "var(--font-mono)", fontSize: "11px", letterSpacing: "0.05em" }}>
                SOVEREIGN NETWORK
              </div>
              <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: "8px" }}>
                <li><a href="https://www.drakestapleton.com" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>Drake Stapleton</a></li>
                <li><a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>GitHub Organization</a></li>
                <li><a href="https://aienos.com/install.sh" style={{ color: "var(--accent-green)", fontFamily: "var(--font-mono)" }}>install.sh</a></li>
                <li><a href="mailto:aien@aienos.com" style={{ color: "var(--text-secondary)" }}>aien@aienos.com</a></li>
              </ul>
            </div>
          </div>
        </div>

        <div style={{
          borderTop: "1px solid var(--border-subtle)",
          paddingTop: "24px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "12px",
          fontFamily: "var(--font-mono)",
          fontSize: "11px"
        }}>
          <div>
            AIEN Sovereign Commoners &copy; 2026. Licensed under Apache-2.0 and SRCL-1.0.
          </div>
          <div style={{ display: "flex", gap: "16px" }}>
            <span>SECURE_TPM_ONLY</span>
            <span>ZERO_UNSOLICITED_TELEMETRY</span>
          </div>
        </div>
      </div>
    </footer>
  );
};
