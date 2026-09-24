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
              A sovereign agent and inference runtime in native Rust. Primary reference hardware is NVIDIA DGX Spark (Grace Blackwell GB10); portable components also target Apple Silicon and Linux x86_64.
            </p>
          </div>

          <div style={{ display: "flex", gap: "48px", flexWrap: "wrap" }}>
            <div>
              <div style={{ fontWeight: 600, color: "var(--text-primary)", marginBottom: "12px", fontFamily: "var(--font-mono)", fontSize: "11px", letterSpacing: "0.05em" }}>
                ECOSYSTEM
              </div>
              <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: "8px" }}>
                <li><a href="https://github.com/aien-dev/aien-sovereign-core" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>AIEN Neural Runtime</a></li>
                <li><a href="https://github.com/aien-dev/benchmarks" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>Benchmarks</a></li>
                <li><a href="https://github.com/aien-dev/aien-protocols" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>AIEN Protocols</a></li>
                <li><a href="https://github.com/aien-dev/aegis-runtime" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>AEGIS Runtime</a></li>
              </ul>
            </div>

            <div>
              <div style={{ fontWeight: 600, color: "var(--text-primary)", marginBottom: "12px", fontFamily: "var(--font-mono)", fontSize: "11px", letterSpacing: "0.05em" }}>
                PROJECT
              </div>
              <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: "8px" }}>
                <li><a href="https://www.drakestapleton.com" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>Drake Stapleton</a></li>
                <li><a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>GitHub</a></li>
                <li><a href="https://github.com/aien-dev/aien-sovereign-core/blob/main/COVENANT.md" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)" }}>Covenant</a></li>
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
            &copy; 2026 Drake Stapleton and AIEN Contributors. Code licensed under{" "}
            <a href="https://github.com/aien-dev/aien-sovereign-core/blob/main/LICENSE" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)", textDecoration: "underline" }}>
              Apache-2.0 with LLVM Exception
            </a>.
            {" "}The <a href="https://github.com/aien-dev/aien-sovereign-core/blob/main/COVENANT.md" target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-secondary)", textDecoration: "underline" }}>covenant</a> states values only and grants or restricts no rights.
          </div>
        </div>
      </div>
    </footer>
  );
};
