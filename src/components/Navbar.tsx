import React from "react";
import { Terminal } from "lucide-react";
import { GithubIcon } from "./Icons";

export const Navbar: React.FC = () => {
  return (
    <header style={{
      borderBottom: "1px solid var(--border-subtle)",
      background: "rgba(8, 9, 12, 0.85)",
      backdropFilter: "blur(12px)",
      position: "sticky",
      top: 0,
      zIndex: 50,
      padding: "16px 0"
    }}>
      <div className="container" style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "16px"
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <div style={{
            width: "32px",
            height: "32px",
            borderRadius: "6px",
            background: "linear-gradient(135deg, #131822 0%, #1e2638 100%)",
            border: "1px solid var(--accent-blue)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 0 12px var(--accent-blue-glow)"
          }}>
            <Terminal size={18} color="var(--accent-blue)" />
          </div>
          <div style={{ display: "flex", flexDirection: "column" }}>
            <span style={{
              fontWeight: 700,
              fontSize: "18px",
              letterSpacing: "-0.02em",
              color: "var(--text-primary)"
            }}>
              AIEN OS
            </span>
            <span style={{
              fontSize: "10px",
              fontFamily: "var(--font-mono)",
              color: "var(--text-muted)",
              letterSpacing: "0.08em"
            }}>
              NEURAL OPERATING ENVIRONMENT
            </span>
          </div>
        </div>

        <nav style={{
          display: "flex",
          alignItems: "center",
          gap: "24px",
          fontSize: "14px",
          fontWeight: 500
        }}>
          <a href="#terminal" style={{ color: "var(--text-secondary)" }}>Terminal</a>
          <a href="#benchmarks" style={{ color: "var(--text-secondary)" }}>Benchmarks</a>
          <a href="#architecture" style={{ color: "var(--text-secondary)" }}>Architecture</a>
          <a href="#ecosystem" style={{ color: "var(--text-secondary)" }}>Ecosystem</a>
          <a href="#install" style={{ color: "var(--text-secondary)" }}>Install</a>
          <a
            href="https://github.com/aien-dev"
            target="_blank"
            rel="noopener noreferrer"
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              color: "var(--text-primary)",
              padding: "6px 12px",
              borderRadius: "4px",
              border: "1px solid var(--border-subtle)",
              background: "var(--bg-surface)",
              fontFamily: "var(--font-mono)",
              fontSize: "12px"
            }}
          >
            <GithubIcon size={14} />
            <span>GitHub</span>
          </a>
        </nav>
      </div>
    </header>
  );
};
