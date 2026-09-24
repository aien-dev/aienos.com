use maud::{html, Markup};

pub fn render_terminal() -> Markup {
    html! {
        section id="terminal" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle); background: var(--bg-surface);" {
            div class="container" {
                div style="margin-bottom: 32px;" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "LIVE INTERACTIVE ENVIRONMENT"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Neural Syscall Dispatcher"
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; max-width: 680px;" {
                        "Execute direct cognitive syscalls against simulated Grace Blackwell silicon. Zero runtime interpreter lag."
                    }
                }

                div class="terminal-shell" style="border: 1px solid var(--border-subtle); border-radius: 8px; background: var(--bg-base); box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5); overflow: hidden;" {
                    div style="padding: 12px 16px; background: rgba(14, 18, 25, 0.95); border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: space-between; gap: 16px;" {
                        div style="display: flex; align-items: center; gap: 8px;" {
                            span style="width: 10px; height: 10px; border-radius: 50%; background: #ef4444;" {}
                            span style="width: 10px; height: 10px; border-radius: 50%; background: #f59e0b;" {}
                            span style="width: 10px; height: 10px; border-radius: 50%; background: #10b981;" {}
                            span class="terminal-title" style="margin-left: 12px; font-family: var(--font-mono); font-size: 12px; color: var(--text-muted);" {
                                "AIEN OS REPL v0.4.0 (sm_121a · aarch64 · 128GB LPDDR5x)"
                            }
                        }
                        div class="terminal-status" style="display: flex; align-items: center; gap: 6px; font-family: var(--font-mono); font-size: 11px; color: var(--accent-green);" {
                            span style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-green); box-shadow: 0 0 6px var(--accent-green);" {}
                            "COHERENT_ACTIVE"
                        }
                    }

                    div style="padding: 10px 16px; background: rgba(14, 18, 25, 0.6); border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 8px; flex-wrap: wrap;" {
                        span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); text-transform: uppercase;" {
                            "Quick Triggers:"
                        }
                        button type="button" data-cmd="sys_telemetry" style="background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 4px 10px; font-family: var(--font-mono); font-size: 12px; cursor: pointer;" {
                            "sys_telemetry"
                        }
                        button type="button" data-cmd="sys_alloc_kv" style="background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 4px 10px; font-family: var(--font-mono); font-size: 12px; cursor: pointer;" {
                            "sys_alloc_kv"
                        }
                        button type="button" data-cmd="sys_bench" style="background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 4px 10px; font-family: var(--font-mono); font-size: 12px; cursor: pointer;" {
                            "sys_bench"
                        }
                        button type="button" data-cmd="sys_vault" style="background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 4px 10px; font-family: var(--font-mono); font-size: 12px; cursor: pointer;" {
                            "sys_vault"
                        }
                        button type="button" id="terminal-clear-btn" style="margin-left: auto; background: transparent; color: var(--text-muted); border: none; font-family: var(--font-mono); font-size: 11px; cursor: pointer;" {
                            "[clear]"
                        }
                    }

                    div id="terminal-output" style="padding: 20px; min-height: 280px; max-height: 440px; overflow-y: auto; font-family: var(--font-mono); font-size: 13px; line-height: 1.6;" {
                        div class="terminal-log-line sys" {
                            span class="sys-msg" style="color: var(--accent-blue);" {
                                "[*] AIEN OS Neural Kernel v0.4.0 initialized on GB10 (aarch64)."
                            }
                        }
                        div class="terminal-log-line sys" {
                            span class="sys-msg" style="color: var(--accent-blue);" {
                                "[*] Unified coherent address space: 128 GB LPDDR5x online."
                            }
                        }
                        div class="terminal-log-line sys" {
                            span class="sys-msg" style="color: var(--accent-blue);" {
                                "[*] Hardware TPM 2.0 vault locked. Zero disk secrets invariant active."
                            }
                        }
                        div class="terminal-log-line sys" {
                            span class="sys-msg" style="color: var(--text-muted);" {
                                "[*] Type a syscall below or click one of the quick trigger buttons above."
                            }
                        }
                    }

                    form id="terminal-form" style="padding: 12px 16px; background: rgba(14, 18, 25, 0.95); border-top: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 8px;" {
                        span style="color: var(--accent-green); font-family: var(--font-mono); font-size: 14px; font-weight: 700;" { "$" }
                        input id="terminal-input" type="text" placeholder="Type a syscall (sys_telemetry, sys_alloc_kv, sys_bench, sys_vault, help)..." autocomplete="off" style="flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-family: var(--font-mono); font-size: 13px;" {}
                        button type="submit" style="background: var(--accent-green); color: var(--bg-base); border: none; border-radius: 4px; padding: 6px 14px; font-family: var(--font-mono); font-size: 12px; font-weight: 700; cursor: pointer;" {
                            "RUN"
                        }
                    }
                }
            }
        }
    }
}
