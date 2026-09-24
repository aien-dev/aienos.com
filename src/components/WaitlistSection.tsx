import React, { useState } from "react";
import { BellRing, Check, Mail, ArrowRight } from "lucide-react";

const WAITLIST_ENDPOINT = import.meta.env.VITE_WAITLIST_ENDPOINT ||
  "https://spark.tail987627.ts.net/aienos-waitlist/api/signup";
const CONSENT_VERSION = "2026-09-23-v1";

export const WaitlistSection: React.FC = () => {
  const [email, setEmail] = useState("");
  const [consent, setConsent] = useState(false);
  const [website, setWebsite] = useState("");
  const [status, setStatus] = useState<"idle" | "sending" | "done" | "error">("idle");
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!consent || status === "sending" || status === "done") return;
    setStatus("sending");
    setError("");
    try {
      const res = await fetch(WAITLIST_ENDPOINT, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ email: email.trim(), consent, consentVersion: CONSENT_VERSION, website })
      });
      if (!res.ok) {
        setError(res.status === 429 ? "Please try again later." : "We could not save your email. Please try again.");
        setStatus("error");
        return;
      }
      setStatus("done");
    } catch {
      setError("The signup service is unavailable. Please try again shortly.");
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
              <span>Received. We will email you when AIEN early access opens.</span>
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
                  placeholder="you@example.com"
                  disabled={status === "sending"}
                  maxLength={254}
                  autoComplete="email"
                  name="email"
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
                    cursor: "text"
                  }}
                />
              </div>
              <div aria-hidden="true" style={{ position: "absolute", width: "1px", height: "1px", overflow: "hidden", clipPath: "inset(50%)" }}>
                <label htmlFor="waitlist-website">Website</label>
                <input id="waitlist-website" name="website" type="text" tabIndex={-1} autoComplete="off" value={website} onChange={(e) => setWebsite(e.target.value)} />
              </div>
              <label style={{ display: "flex", gap: "10px", alignItems: "flex-start", textAlign: "left", width: "100%", color: "var(--text-secondary)", fontSize: "13px", lineHeight: 1.5 }}>
                <input type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} required disabled={status === "sending"} style={{ marginTop: "3px" }} />
                <span>Use my email only to announce AIEN early access. Store my address and signup time for that purpose.</span>
              </label>
              <button
                type="submit"
                disabled={status === "sending"}
                style={{
                  padding: "14px 24px",
                  borderRadius: "8px",
                  border: "1px solid var(--accent-green)",
                  background: "rgba(0, 229, 153, 0.12)",
                  color: "var(--accent-green)",
                  fontWeight: 700,
                  fontSize: "14px",
                  cursor: "pointer",
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
            <p role="alert" style={{ color: "#f59e0b", fontSize: "13px", marginTop: "12px" }}>
              {error}
            </p>
          )}

          <p style={{
            color: "var(--text-muted)",
            fontSize: "12px",
            marginTop: "20px",
            fontFamily: "var(--font-mono)"
          }}>
            One early access announcement. No marketing list. Your email stays private.
          </p>
        </div>
      </div>
    </section>
  );
};
