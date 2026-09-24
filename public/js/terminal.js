// AIEN OS Interactive Neural Terminal Micro-Script
// Pure Vanilla JavaScript (<2 KB) - Zero Virtual DOM overhead

(function() {
  const RESPONSES = {
    sys_telemetry: {
      device: "NVIDIA Grace Blackwell GB10",
      compute_capability: "sm_121a",
      unified_memory_total_gb: 128.0,
      resident_kernel_model_gb: 64.3,
      dynamic_kv_cache_allocated_gb: 4.8,
      free_memory_gb: 58.9,
      gpu_temp_celsius: 45.0,
      power_draw_watts: 11.5,
      performance_state: "P0 (Maximum Performance)"
    },
    sys_alloc_kv: {
      syscall: "sys_alloc_kv",
      status: "SUCCESS",
      blocks_requested: 2048,
      block_size_tokens: 16,
      bytes_allocated: 33554432,
      alloc_duration_us: 1420,
      fragmentation_ratio: 0.000,
      page_table_mapping: "coherent_unified_sm_121a"
    },
    sys_bench: {
      syscall: "sys_simd_benchmark",
      compiler: "Mojo 1.0 (MLIR LLVM)",
      target: "aarch64-unknown-linux-gnu",
      scalar_throughput_gflops: 12.8,
      simd_32wide_throughput_gflops: 124.2,
      speedup_multiplier: "9.70x",
      coherent_memory_bandwidth_gbps: 81.6
    },
    sys_vault: {
      subsystem: "atlas-vault",
      hardware_key_substrate: "/dev/tpmrm0",
      algorithm: "ECDSA P-256 (NIST SP 800-90A)",
      disk_secrets_found: 0,
      policy_status: "SECURE_TPM_ONLY",
      memory_redaction_filter: "ACTIVE ([REDACTED_BY_ATLAS_VAULT])"
    }
  };

  const HELP_TEXT = "Supported Neural Syscalls:\n  sys_telemetry - Inspect GB10 thermals, power, and unified VRAM\n  sys_alloc_kv   - Allocate PagedAttention neural memory blocks\n  sys_bench      - Run Mojo 1.0 32-wide SIMD kernel benchmark\n  sys_vault      - Validate hardware TPM 2.0 key vault security\n  clear          - Clear terminal display";

  function initTerminal() {
    const output = document.getElementById("terminal-output");
    const input = document.getElementById("terminal-input");
    const form = document.getElementById("terminal-form");
    if (!output || !input || !form) return;

    function appendEntry(type, text) {
      const line = document.createElement("div");
      line.className = "terminal-log-line " + type;
      if (type === "cmd") {
        line.innerHTML = '<span class="prompt-symbol">$ </span>' + escapeHtml(text);
      } else if (type === "resp") {
        line.innerHTML = '<pre class="terminal-resp">' + escapeHtml(text) + '</pre>';
      } else {
        line.innerHTML = '<span class="sys-msg">' + escapeHtml(text) + '</span>';
      }
      output.appendChild(line);
      output.scrollTop = output.scrollHeight;
    }

    function escapeHtml(str) {
      return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    function runCommand(rawCmd) {
      const cmd = rawCmd.trim();
      if (!cmd) return;
      appendEntry("cmd", cmd);

      const normalized = cmd.toLowerCase();
      if (normalized === "clear") {
        output.innerHTML = "";
        return;
      }
      if (normalized === "help") {
        appendEntry("resp", HELP_TEXT);
        return;
      }

      let respKey = null;
      if (normalized === "sys_telemetry" || normalized === "telemetry" || normalized === "status") {
        respKey = "sys_telemetry";
      } else if (normalized === "sys_alloc_kv" || normalized === "alloc_kv") {
        respKey = "sys_alloc_kv";
      } else if (normalized === "sys_bench" || normalized === "bench" || normalized === "simd") {
        respKey = "sys_bench";
      } else if (normalized === "sys_vault" || normalized === "vault") {
        respKey = "sys_vault";
      }

      if (respKey) {
        appendEntry("resp", JSON.stringify(RESPONSES[respKey], null, 2));
      } else {
        appendEntry("resp", "Unrecognized neural syscall: '" + cmd + "'. Type 'help' for supported syscalls.");
      }
    }

    form.addEventListener("submit", function(e) {
      e.preventDefault();
      const val = input.value;
      input.value = "";
      runCommand(val);
    });

    document.querySelectorAll("[data-cmd]").forEach(function(btn) {
      btn.addEventListener("click", function() {
        const cmd = btn.getAttribute("data-cmd");
        if (cmd) runCommand(cmd);
      });
    });

    const clearBtn = document.getElementById("terminal-clear-btn");
    if (clearBtn) {
      clearBtn.addEventListener("click", function() {
        output.innerHTML = "";
      });
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initTerminal);
  } else {
    initTerminal();
  }
})();
