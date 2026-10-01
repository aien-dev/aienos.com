use maud::{html, Markup, PreEscaped};

pub fn render_research() -> Markup {
    let papers = [
        (
            "Computing Machinery and Understanding",
            "The formal paper. The Turing is defined, the measurement protocol is frozen, and the first measurements are published with sealed data and receipts anyone can recompute.",
            "Formal paper",
            "/research/computing-machinery-and-understanding",
            "Read the paper",
        ),
        (
            "Computing Machinery and Understanding: Emergence",
            "The companion paper: how understanding emerges in a system, and what the measure says about it.",
            "Companion paper",
            "/research/computing-machinery-and-understanding-emergence",
            "Read the companion",
        ),
        (
            "The Stapleton Doctrine",
            "The philosophy behind the architecture, stated as a position paper: build on physics, own the foundation, prove every claim.",
            "Position paper",
            "/research/the-stapleton-doctrine",
            "Read the doctrine",
        ),
        (
            "The Post-LLM Case",
            "The case for building past the large language model era, and what an owned machine changes about who gets to think.",
            "Essay",
            "/post-llm-case/",
            "Read the essay",
        ),
    ];

    html! {
        section id="research" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle);" {
            div class="container" {
                div style="margin-bottom: 40px; max-width: 760px;" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "PUBLISHED RESEARCH"
                    }
                    h2 style="font-size: 32px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary);" {
                        "Do not take our word for it. Check the receipts."
                    }
                    p style="color: var(--text-secondary); margin-top: 8px; line-height: 1.6;" {
                        "Most AI asks you to trust a demo and a promise. This project publishes its science in the open instead: a unit for measuring machine understanding, the papers behind it, and the data anyone can recheck. It starts with the Turing."
                    }
                }

                // Featured: the Turing, with the playable demo.
                div class="glow-box" style="padding: 32px; border-radius: 8px; margin-bottom: 24px;" {
                    div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 32px;" {
                        div {
                            div class="badge badge-blue" style="margin-bottom: 12px; font-size: 10px;" {
                                "THE TURING (T)"
                            }
                            h3 style="font-size: 24px; font-weight: 700; color: var(--text-primary); margin-bottom: 10px;" {
                                "A unit for machine understanding"
                            }
                            p style="color: var(--text-secondary); font-size: 15px; line-height: 1.6; margin-bottom: 12px;" {
                                "One Turing is one bit of net held-out description-length improvement: a short explanation that predicts things it has never seen earns Turings. Guessing earns nothing. Memorizing earns nothing, because the test data is new."
                            }
                            p style="color: var(--text-secondary); font-size: 15px; line-height: 1.6; margin-bottom: 20px;" {
                                "It is the meter this whole project runs on, and the first real measurement is already on the books."
                            }
                            div style="display: flex; gap: 12px; flex-wrap: wrap;" {
                                a href="/turing/" style="background: var(--accent-green); color: var(--bg-base); border-radius: 4px; padding: 8px 16px; font-family: var(--font-mono); font-size: 13px; font-weight: 700;" {
                                    "Explore the Turing"
                                }
                                a href="/research/computing-machinery-and-understanding" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 8px 16px; font-family: var(--font-mono); font-size: 13px; font-weight: 600;" {
                                    "Read the formal paper"
                                }
                            }
                        }

                        // The game: guess the next bit, earn Turings.
                        div id="turing-game" style="background: var(--bg-base); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 20px;" {
                            div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;" {
                                span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em;" {
                                    "Try it: earn your first Turings"
                                }
                                span id="tg-round" style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);" {
                                    "Round 0 / 12"
                                }
                            }
                            p style="font-size: 13px; color: var(--text-secondary); line-height: 1.5; margin-bottom: 14px;" {
                                "A hidden rule is generating this stream of bits. Watch it, find the pattern, and call the next bit before it lands."
                            }
                            div id="tg-stream" aria-live="polite" style="font-family: var(--font-mono); font-size: 20px; font-weight: 700; letter-spacing: 0.35em; color: var(--text-primary); min-height: 30px; margin-bottom: 16px; word-break: break-all;" {
                                ""
                            }
                            div style="display: flex; gap: 12px; margin-bottom: 16px;" {
                                button type="button" id="tg-zero" style="flex: 1; background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 6px; padding: 12px; font-family: var(--font-mono); font-size: 18px; font-weight: 700; cursor: pointer;" {
                                    "0"
                                }
                                button type="button" id="tg-one" style="flex: 1; background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-subtle); border-radius: 6px; padding: 12px; font-family: var(--font-mono); font-size: 18px; font-weight: 700; cursor: pointer;" {
                                    "1"
                                }
                            }
                            div style="display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap;" {
                                div style="font-family: var(--font-mono); font-size: 14px; color: var(--text-primary);" {
                                    "Score: "
                                    span id="tg-score" style="color: var(--accent-green); font-weight: 700;" { "0 T" }
                                }
                                button type="button" id="tg-reset" style="background: transparent; color: var(--text-muted); border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 12px; font-family: var(--font-mono); font-size: 12px; cursor: pointer;" {
                                    "New stream"
                                }
                            }
                            p id="tg-msg" aria-live="polite" style="font-size: 13px; color: var(--text-secondary); line-height: 1.5; margin-top: 12px; min-height: 20px;" {
                                "Each call you get right earns 1 T, each miss costs 1 T. A coin flip averages zero."
                            }
                        }
                    }
                }

                // The papers.
                div class="responsive-card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 24px;" {
                    @for (title, desc, badge, url, cta) in &papers {
                        div class="glow-box" style="padding: 28px; border-radius: 8px; display: flex; flex-direction: column; justify-content: space-between; gap: 20px;" {
                            div {
                                div style="margin-bottom: 12px;" {
                                    span class="badge badge-blue" style="font-size: 10px;" {
                                        (badge)
                                    }
                                }
                                h3 style="font-size: 18px; font-weight: 700; color: var(--text-primary); margin-bottom: 8px;" {
                                    a href=(url) style="color: inherit;" {
                                        (title)
                                    }
                                }
                                p style="color: var(--text-secondary); font-size: 14px; line-height: 1.6; margin: 0;" {
                                    (desc)
                                }
                            }
                            div style="padding-top: 16px; border-top: 1px solid var(--border-subtle); font-family: var(--font-mono); font-size: 12px;" {
                                a href=(url) style="color: var(--accent-green); font-weight: 600;" {
                                    (cta) " →"
                                }
                            }
                        }
                    }
                }

                p style="margin-top: 24px; font-size: 14px; color: var(--text-secondary);" {
                    "Everything, including protocols, sealed experiment profiles, and source files: "
                    a href="/research/" style="color: var(--accent-green); font-weight: 600;" {
                        "the full research index"
                    }
                    (PreEscaped("&nbsp;·&nbsp;"))
                    a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer" style="color: var(--accent-green); font-weight: 600;" {
                        "the public code and receipts on GitHub"
                    }
                }
            }
        }
    }
}
