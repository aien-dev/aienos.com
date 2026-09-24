use maud::{html, Markup, PreEscaped};

pub fn render_waitlist() -> Markup {
    let waitlist_endpoint = "https://spark.tail987627.ts.net/aienos-waitlist/api/signup";
    let consent_version = "2026-09-23-v1";

    html! {
        section id="waitlist" style="padding: 80px 0; border-bottom: 1px solid var(--border-subtle); background: var(--bg-base); position: relative;" {
            div class="container" {
                div class="glow-box" style="max-width: 760px; margin: 0 auto; padding: 48px 40px; border-radius: 12px; text-align: center; background: var(--bg-surface);" {
                    div class="badge badge-green" style="margin-bottom: 16px; display: inline-flex; align-items: center; gap: 6px;" {
                        (PreEscaped(r#"<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"></path><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"></path><path d="M4 2C2.8 3.7 2 5.7 2 8"></path><path d="M22 8c0-2.3-.8-4.3-2-6"></path></svg>"#))
                        span { "COMING SOON" }
                    }

                    h2 style="font-size: clamp(28px, 4vw, 40px); font-weight: 800; letter-spacing: -0.02em; color: var(--text-primary); margin-bottom: 16px;" {
                        "AIEN is coming."
                    }

                    p style="color: var(--text-secondary); line-height: 1.6; max-width: 600px; margin: 0 auto 32px auto; font-size: 16px;" {
                        "AIENOS is in active development. AIEN, the intelligence layer built on the sovereign stack, is on its way. Join the waitlist and be first to know when early access opens."
                    }

                    div id="waitlist-success" style="display: none; align-items: center; justify-content: center; gap: 10px; padding: 16px 24px; border-radius: 8px; background: rgba(0, 229, 153, 0.08); border: 1px solid var(--accent-green); color: var(--accent-green); font-weight: 600; max-width: 480px; margin: 0 auto;" {
                        (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>"#))
                        span { "Received. We will email you when AIEN early access opens." }
                    }

                    form id="waitlist-form" style="display: flex; gap: 12px; max-width: 480px; margin: 0 auto; flex-wrap: wrap; justify-content: center;" {
                        div style="position: relative; flex: 1 1 260px; display: flex; align-items: center;" {
                            div style="position: absolute; left: 14px; color: var(--text-muted); pointer-events: none; display: flex; align-items: center;" {
                                (PreEscaped(r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="16" x="2" y="4" rx="2"></rect><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"></path></svg>"#))
                            }
                            input
                                id="waitlist-email"
                                type="email"
                                required
                                placeholder="you@example.com"
                                maxlength="254"
                                autocomplete="email"
                                name="email"
                                aria-label="Email address for the AIEN waitlist"
                                style="width: 100%; padding: 14px 14px 14px 40px; border-radius: 8px; border: 1px solid var(--border-subtle); background: var(--bg-base); color: var(--text-primary); font-size: 14px; font-family: var(--font-sans); outline: none; cursor: text;"
                            {}
                        }

                        div aria-hidden="true" style="position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%);" {
                            label for="waitlist-website" { "Website" }
                            input id="waitlist-website" name="website" type="text" tabindex="-1" autocomplete="off" {}
                        }

                        label style="display: flex; gap: 10px; align-items: flex-start; text-align: left; width: 100%; color: var(--text-secondary); font-size: 13px; line-height: 1.5;" {
                            input id="waitlist-consent" type="checkbox" required style="margin-top: 3px;" {}
                            span { "Use my email only to announce AIEN early access. Store my address and signup time for that purpose." }
                        }

                        button
                            id="waitlist-submit"
                            type="submit"
                            style="padding: 14px 24px; border-radius: 8px; border: 1px solid var(--accent-green); background: rgba(0, 229, 153, 0.12); color: var(--accent-green); font-weight: 700; font-size: 14px; cursor: pointer; display: flex; align-items: center; gap: 8px; font-family: var(--font-sans); white-space: nowrap;"
                        {
                            span id="waitlist-submit-text" { "NOTIFY ME" }
                            (PreEscaped(r#"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"></line><polyline points="12 5 19 12 12 19"></polyline></svg>"#))
                        }
                    }

                    p id="waitlist-error" role="alert" style="display: none; color: #f59e0b; font-size: 13px; margin-top: 12px;" {}

                    p style="color: var(--text-muted); font-size: 12px; margin-top: 20px; font-family: var(--font-mono);" {
                        "One early access announcement. No marketing list. Your email stays private."
                    }
                }
            }

            script {
                (PreEscaped(format!(r#"
                (function() {{
                    const form = document.getElementById('waitlist-form');
                    const emailInput = document.getElementById('waitlist-email');
                    const consentInput = document.getElementById('waitlist-consent');
                    const websiteInput = document.getElementById('waitlist-website');
                    const submitBtn = document.getElementById('waitlist-submit');
                    const submitText = document.getElementById('waitlist-submit-text');
                    const successDiv = document.getElementById('waitlist-success');
                    const errorP = document.getElementById('waitlist-error');
                    const endpoint = '{}';
                    const consentVer = '{}';

                    if (!form) return;

                    form.addEventListener('submit', async function(e) {{
                        e.preventDefault();
                        if (!consentInput.checked) return;

                        submitBtn.disabled = true;
                        submitText.textContent = 'JOINING...';
                        errorP.style.display = 'none';

                        try {{
                            const res = await fetch(endpoint, {{
                                method: 'POST',
                                headers: {{
                                    'Content-Type': 'application/json',
                                    'Accept': 'application/json'
                                }},
                                body: JSON.stringify({{
                                    email: emailInput.value.trim(),
                                    consent: true,
                                    consentVersion: consentVer,
                                    website: websiteInput ? websiteInput.value : ''
                                }})
                            }});

                            if (!res.ok) {{
                                const msg = res.status === 429
                                    ? 'Please try again later.'
                                    : 'We could not save your email. Please try again.';
                                errorP.textContent = msg;
                                errorP.style.display = 'block';
                                submitBtn.disabled = false;
                                submitText.textContent = 'NOTIFY ME';
                                return;
                            }}

                            form.style.display = 'none';
                            successDiv.style.display = 'flex';
                        }} catch (err) {{
                            errorP.textContent = 'The signup service is unavailable. Please try again shortly.';
                            errorP.style.display = 'block';
                            submitBtn.disabled = false;
                            submitText.textContent = 'NOTIFY ME';
                        }}
                    }});
                }})();
                "#, waitlist_endpoint, consent_version)))
            }
        }
    }
}
