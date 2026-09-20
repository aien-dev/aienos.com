import React, { useState } from "react";
import { Terminal as TerminalIcon, Play, RefreshCw, Cpu, Database, Shield, Zap } from "lucide-react";

interface LogEntry {
  type: "cmd" | "resp" | "sys";
  text: string;
}

export const InteractiveTerminal: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([
    { type: "sys", text: "[*] AIEN OS Neural Kernel v0.4.0 initialized on GB10 (aarch64)." },
    { type: "sys", text: "[*] Unified coherent address space: 128 GB LPDDR5x online." },
    { type: "sys", text: "[*] Hardware TPM 2.0 vault locked. Zero disk secrets invariant active." },
    { type: "sys", text: "[*] Type a syscall below or click one of the quick trigger buttons." }
  ]);
  const [input, setInput] = useState("");

  const executeCommand = (cmd: string) => {
    const trimmed = cmd.trim().toLowerCase();
    const newLogs: LogEntry[] = [...logs, { type: "cmd", text: `$ ${cmd}` }];

    if (trimmed === "sys_telemetry" || trimmed === "telemetry" || trimmed === "status") {
      newLogs.push({
        type: "resp",
        text: JSON.stringify({
          device: "NVIDIA Grace Blackwell GB10",
          compute_capability: "sm_121a",
          unified_memory_total_gb: 128.0,
          resident_kernel_model_gb: 64.3,
          dynamic_kv_cache_allocated_gb: 4.8,
          free_memory_gb: 58.9,
          gpu_temp_celsius: 45.0,
          power_draw_watts: 11.5,
          performance_state: "P0 (Maximum Performance)"
        }, null, 2)
      });
    } else if (trimmed === "sys_alloc_kv" || trimmed === "alloc_kv") {
      newLogs.push({
        type: "resp",
        text: JSON.stringify({
          syscall: "sys_alloc_kv",
          status: "SUCCESS",
          blocks_requested: 2048,
          block_size_tokens: 16,
          bytes_allocated: 33554432,
          alloc_duration_us: 1420,
          fragmentation_ratio: 0.000,
          page_table_mapping: "coherent_unified_sm_121a"
        }, null, 2)
      });
    } else if (trimmed === "sys_bench" || trimmed === "bench" || trimmed === "simd") {
      newLogs.push({
        type: "resp",
        text: JSON.stringify({
          syscall: "sys_simd_benchmark",
          compiler: "Mojo 1.0 (MLIR LLVM)",
          target: "aarch64-unknown-linux-gnu",
          scalar_throughput_gflops: 12.8,
          simd_32wide_throughput_gflops: 124.2,
          speedup_multiplier: "9.70x",
          coherent_memory_bandwidth_gbps: 81.6
        }, null, 2)
      });
    } else if (trimmed === "sys_vault" || trimmed === "vault") {
      newLogs.push({
        type: "resp",
        text: JSON.stringify({
          subsystem: "atlas-vault",
          hardware_key_substrate: "/dev/tpmrm0",
          algorithm: "ECDSA P-256 (NIST SP 800-90A)",
          disk_secrets_found: 0,
          policy_status: "SECURE_TPM_ONLY",
          memory_redaction_filter: "ACTIVE ([REDACTED_BY_ATLAS_VAULT])"
        }, null, 2)
      });
    } else if (trimmed === "help") {
      newLogs.push({
        type: "resp",
        text: "Supported Neural Syscalls:\n  sys_telemetry - Inspect GB10 thermals, power, and unified VRAM\n  sys_alloc_kv   - Allocate PagedAttention neural memory blocks\n  sys_bench      - Run Mojo 1.0 32-wide SIMD kernel benchmark\n  sys_vault      - Validate hardware TPM 2.0 key vault security\n  clear          - Clear terminal display"
      });
    } else if (trimmed === "clear") {
      setLogs([]);
      setInput("");
      return;
    } else {
      newLogs.push({
        type: "resp",
        text: `Unknown command: . Type help for available neural syscalls.`
      });
    }

    setLogs(newLogs);
    setInput("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && input.trim()) {
      executeCommand(input);
    }
  };

  return (
    <section id="terminal" style={{ padding: "80px 0", borderBottom: "1px solid var(--border-subtle)" }}>
      <div className="container">
        <div style={{ marginBottom: "32px" }}>
          <div className="badge badge-blue" style={{ marginBottom: "12px" }}>
            <TerminalIcon size={12} />
            <span>INTERACTIVE KERNEL CONSOLE</span>
          </div>
          <h2 style={{ fontSize: "32px", fontWeight: 700, letterSpacing: "-0.02em", color: "var(--text-primary)" }}>
            Live Neural Syscall Interface
          </h2>
          <p style={{ color: "var(--text-secondary)", marginTop: "8px", maxWidth: "680px" }}>
            Execute compiled system calls directly against the AIEN neural virtual machine definitions.
          </p>
        </div>

        {/* Quick Action Trigger Buttons */}
        <div style={{
          display: "flex",
          gap: "10px",
          marginBottom: "16px",
          flexWrap: "wrap"
        }}>
          <button
            onClick={() => executeCommand("sys_telemetry")}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              padding: "8px 14px",
              background: "var(--bg-surface)",
              border: "1px solid var(--border-subtle)",
              borderRadius: "6px",
              color: "var(--text-primary)",
              fontFamily: "var(--font-mono)",
              fontSize: "12px",
              cursor: "pointer"
            }}
          >
            <Cpu size={14} color="var(--accent-green)" />
            <span>sys_telemetry</span>
          </button>

          <button
            onClick={() => executeCommand("sys_alloc_kv")}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              padding: "8px 14px",
              background: "var(--bg-surface)",
              border: "1px solid var(--border-subtle)",
              borderRadius: "6px",
              color: "var(--text-primary)",
              fontFamily: "var(--font-mono)",
              fontSize: "12px",
              cursor: "pointer"
            }}
          >
            <Database size={14} color="var(--accent-blue)" />
            <span>sys_alloc_kv</span>
          </button>

          <button
            onClick={() => executeCommand("sys_bench")}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              padding: "8px 14px",
              background: "var(--bg-surface)",
              border: "1px solid var(--border-subtle)",
              borderRadius: "6px",
              color: "var(--text-primary)",
              fontFamily: "var(--font-mono)",
              fontSize: "12px",
              cursor: "pointer"
            }}
          >
            <Zap size={14} color="var(--accent-amber)" />
            <span>sys_bench</span>
          </button>

          <button
            onClick={() => executeCommand("sys_vault")}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              padding: "8px 14px",
              background: "var(--bg-surface)",
              border: "1px solid var(--border-subtle)",
              borderRadius: "6px",
              color: "var(--text-primary)",
              fontFamily: "var(--font-mono)",
              fontSize: "12px",
              cursor: "pointer"
            }}
          >
            <Shield size={14} color="var(--accent-green)" />
            <span>sys_vault</span>
          </button>

          <button
            onClick={() => executeCommand("clear")}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              padding: "8px 14px",
              background: "transparent",
              border: "1px solid var(--border-subtle)",
              borderRadius: "6px",
              color: "var(--text-muted)",
              fontFamily: "var(--font-mono)",
              fontSize: "12px",
              cursor: "pointer",
              marginLeft: "auto"
            }}
          >
            <RefreshCw size={12} />
            <span>clear</span>
          </button>
        </div>

        {/* Terminal Window */}
        <div style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border-subtle)",
          borderRadius: "8px",
          overflow: "hidden",
          boxShadow: "0 20px 48px rgba(0, 0, 0, 0.5)"
        }}>
          {/* Terminal Window Header */}
          <div style={{
            background: "#0c0f16",
            padding: "12px 18px",
            borderBottom: "1px solid var(--border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between"
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <span style={{ width: "10px", height: "10px", borderRadius: "50%", background: "#ef4444" }} />
              <span style={{ width: "10px", height: "10px", borderRadius: "50%", background: "#f59e0b" }} />
              <span style={{ width: "10px", height: "10px", borderRadius: "50%", background: "#10b981" }} />
              <span style={{ marginLeft: "12px", fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--text-muted)" }}>
                aien@spark-gb10: ~ (neural_syscall_v0.4.0)
              </span>
            </div>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--accent-green)" }}>
              COHERENT BUS READY
            </span>
          </div>

          {/* Terminal Output Area */}
          <div style={{
            padding: "20px",
            minHeight: "300px",
            maxHeight: "440px",
            overflowY: "auto",
            fontFamily: "var(--font-mono)",
            fontSize: "13px",
            lineHeight: 1.6
          }}>
            {logs.map((entry, idx) => (
              <div key={idx} style={{
                marginBottom: "8px",
                color: entry.type === "cmd"
                  ? "var(--text-primary)"
                  : entry.type === "sys"
                  ? "var(--text-muted)"
                  : "var(--accent-green)",
                whiteSpace: "pre-wrap"
              }}>
                {entry.text}
              </div>
            ))}
          </div>

          {/* Terminal Input Line */}
          <div style={{
            borderTop: "1px solid var(--border-subtle)",
            background: "#0a0d13",
            padding: "12px 20px",
            display: "flex",
            alignItems: "center",
            gap: "10px"
          }}>
            <span style={{ color: "var(--accent-green)", fontFamily: "var(--font-mono)", fontSize: "14px" }}>$</span>
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Type sys_telemetry, sys_alloc_kv, sys_bench, or sys_vault..."
              style={{
                flex: 1,
                background: "transparent",
                border: "none",
                outline: "none",
                fontFamily: "var(--font-mono)",
                fontSize: "13px",
                color: "var(--text-primary)"
              }}
            />
            <button
              onClick={() => input.trim() && executeCommand(input)}
              style={{
                background: "var(--bg-card)",
                border: "1px solid var(--border-subtle)",
                borderRadius: "4px",
                color: "var(--text-secondary)",
                padding: "4px 10px",
                fontFamily: "var(--font-mono)",
                fontSize: "11px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "4px"
              }}
            >
              <Play size={10} />
              <span>RUN</span>
            </button>
          </div>
        </div>
      </div>
    </section>
  );
};
