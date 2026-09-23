import React, { useState } from "react";
import { BellRing, Check, Mail, ArrowRight } from "lucide-react";

// ---------------------------------------------------------------------------
// Email capture endpoint. This site is static, so submissions POST here.
//
// To activate: create a free form at https://formspree.io (about two minutes),
// then paste the endpoint below, e.g. "https://formspree.io/f/xabc1234".
// Until then the form renders in a disabled "opening soon" state.
// ---------------------------------------------------------------------------
const WAITLIST_ENDPOINT = "";

export const WaitlistSection: React.FC = () => {
  const [email, setEmail] = useState("");
  const [status, setStatus] = useState<"idle" | "sending" | "done" | "error">("idle");
  const active = WAITLIST_ENDPOINT.length > 0;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!active || status === "sending" || status === "done") return;
    setStatus("sending");
    try {
      const res = await fetch(WAITLIST_ENDPOINT, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ email, source: "aienos.com waitlist" })
      });
      setStatus(res.ok ? "done" : "error");
    } catch {
      setStatus("error");
    }
  };

  return (
    <section id="waitlist" style={{
      padding: "80px 0",
      borderBottom: "1px solid var(--border-subtle)",
      background: "var(--bg-base)",
      position: "relative"
    }}>
      <div className="container">
        <div className="glow-box" style={{
          maxWidth: "760px",
          margin: "0 auto",
          padding: "48px 40px",
          borderRadius: "12px",
          textAlign: "center",
          background: "var(--bg-surface)"
        }}>
          <div className="badge badge-green" style={{ marginBottom: "16px" }}>
            <BellRing size={12} />
            <span>COMING SOON</span>
          </div>

          <h2 style={{
            fontSize: "clamp(28px, 4vw, 40px)",
            fontWeight: 800,
            letterSpacing: "-0.02em",
            color: "var(--text-primary)",
            marginBottom: "16px"
          }}>
            AIEN is coming.
          </h2>

          <p style={{
            color: "var(--text-secondary)",
            lineHeight: 1.6,
            maxWidth: "600px",
            margin: "0 auto 32px auto",
            fontSize: "16px"
          }}>
            AIENOS is in active development. AIEN, the intelligence layer built on
            the sovereign stack, is on its way. Join the waitlist and be first to
            know when early access opens.
          </p>

          {status === "done" ? (
            <div style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "10px",
              padding: "16px 24px",
              borderRadius: "8px",
              background: "rgba(0, 229, 153, 0.08)",
              border: "1px solid var(--accent-green)",
              color: "var(--accent-green)",
              fontWeight: 600,
              maxWidth: "480px",
              margin: "0 auto"
            }}>
              <Check size={18} />
              <span>You are on the list. Watch your inbox.</span>
            </div>
          ) : (
            <form onSubmit={handleSubmit} style={{
              display: "flex",
              gap: "12px",
              maxWidth: "480px",
              margin: "0 auto",
              flexWrap: "wrap",
              justifyContent: "center"
            }}>
              <div style={{
                position: "relative",
                flex: "1 1 260px",
                display: "flex",
                alignItems: "center"
              }}>
                <Mail size={16} style={{
                  position: "absolute",
                  left: "14px",
                  color: "var(--text-muted)",
                  pointerEvents: "none"
                }} />
                <input
                  type="email"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder={active ? "you@example.com" : "Waitlist opening soon"}
                  disabled={!active}
                  aria-label="Email address for the AIEN waitlist"
                  style={{
                    width: "100%",
                    padding: "14px 14px 14px 40px",
                    borderRadius: "8px",
                    border: "1px solid var(--border-subtle)",
                    background: "var(--bg-base)",
                    color: "var(--text-primary)",
                    fontSize: "14px",
                    fontFamily: "var(--font-sans)",
                    outline: "none",
                    opacity: active ? 1 : 0.6,
                    cursor: active ? "text" : "not-allowed"
                  }}
                />
              </div>
              <button
                type="submit"
                disabled={!active || status === "sending"}
                style={{
                  padding: "14px 24px",
                  borderRadius: "8px",
                  border: "1px solid var(--accent-green)",
                  background: active ? "rgba(0, 229, 153, 0.12)" : "var(--bg-card)",
                  color: active ? "var(--accent-green)" : "var(--text-muted)",
                  fontWeight: 700,
                  fontSize: "14px",
                  cursor: active ? "pointer" : "not-allowed",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  fontFamily: "var(--font-sans)",
                  whiteSpace: "nowrap"
                }}
              >
                {status === "sending" ? "JOINING..." : (
                  <>
                    <span>NOTIFY ME</span>
                    <ArrowRight size={14} />
                  </>
                )}
              </button>
            </form>
          )}

          {status === "error" && (
            <p style={{ color: "#f59e0b", fontSize: "13px", marginTop: "12px" }}>
              Something went wrong. Try again, or reach us at aien@aienos.com.
            </p>
          )}

          <p style={{
            color: "var(--text-muted)",
            fontSize: "12px",
            marginTop: "20px",
            fontFamily: "var(--font-mono)"
          }}>
            One email when early access opens. No spam, no telemetry, unsubscribe anytime.
          </p>
        </div>
      </div>
    </section>
  );
};
