use maud::{html, Markup};

pub const HEAD: &str = "<link rel=\"stylesheet\" href=\"/assets/allen-intro.css\"><script defer src=\"/js/allen-intro.js\"></script><script defer src=\"/js/allen-scene.js\"></script><script defer src=\"/js/allen-presence.js\"></script>";

/// Allen's greeting (Drake's words), static and client-rendered so it appears instantly.
/// Allen's speech carries `data-speaker="allen"`: his "we" means Allen and the visitor,
/// so `voice` exempts exactly those elements. Allen calls Drake by name.
pub const GREETING: &str = "Hey, I'm Allen. It's nice to meet you!";
pub const INTRO: &str = "You don't have to tell me your name, but if you do, that would be super awesome and we could chat over time. When you come back, I can remember who you are and we can pick up where we left off. We can talk about all kinds of good things in the universe.";
/// The X pixel ask, kept separate from memory: one tap never means both.
pub const OFFER: &str = "Separately, if you'd like, say hi and I'll let Drake know you stopped by. That uses Drake's X pixel to note the visit. Totally fine if not.";
/// Shown instead of the offer when the browser sends Global Privacy Control.
pub const GPC_NOTE: &str = "Your browser sends Global Privacy Control, so I'm not offering Drake's X pixel. It stays off.";
/// After No thanks: no pixel, no memory, and Allen is still there to talk to.
pub const AFTER_NO: &str = "No problem. You can still chat with me anonymously; I won't remember you or tell X anything.";

/// A non-blocking aside, not a modal: no backdrop, no focus theft, page stays usable.
fn dialog() -> Markup {
    html! {
        aside id="aien-allen-intro" role="complementary" aria-label="Allen says hello" aria-live="polite" hidden {
            div class="allen-intro-bubble" {
                p class="allen-intro-name" { "Allen" }
                button id="allen-intro-close" class="allen-intro-close" type="button" aria-label="Close, no thanks" { "\u{00d7}" }
                p id="allen-intro-message" class="allen-intro-message" aria-hidden="true" data-speaker="allen" {
                    span class="allen-intro-ghost" {
                        strong id="allen-intro-title" { (GREETING) } " "
                        span id="allen-intro-intro" { (INTRO) }
                    }
                    span id="allen-intro-typed" { strong {} span {} }
                }
                p id="allen-intro-spoken" class="allen-intro-sr" {}
                div id="allen-intro-chat" class="allen-intro-chips" {
                    a href="/allen/?mode=remember" { "Tell Allen my name" }
                    a href="/allen/?mode=anonymous" { "Chat anonymously" }
                }
                div id="allen-intro-x" {
                    p id="allen-intro-offer" class="allen-intro-note" data-speaker="allen" { (OFFER) }
                    div id="allen-intro-chips" class="allen-intro-chips" {
                        button id="allen-intro-yes" type="button" { "Say hi" }
                        button id="allen-intro-no" type="button" { "No thanks" }
                    }
                }
                p id="allen-intro-gpc" class="allen-intro-note" data-speaker="allen" hidden { (GPC_NOTE) }
                div id="allen-intro-after" hidden {
                    p id="allen-intro-after-text" data-speaker="allen" { (AFTER_NO) }
                    p class="allen-intro-links" { a id="allen-intro-anon" href="/allen/?mode=anonymous" { "Chat anonymously" } }
                }
                (who_panel())
                p class="allen-intro-links" { a href="/privacy/" { "Privacy" } }
            }
        }
    }
}

/// Local-only: everything is read from this document and browser, shown to the
/// visitor, never sent or stored. No fingerprint hash, probing or IP lookup.
fn who_panel() -> Markup {
    html! {
        details id="allen-intro-who" class="allen-intro-who" {
            summary { "Who can see me right now?" }
            div class="allen-intro-who-body" {
                h3 { "Trackers on this page right now: " span id="allen-intro-tracker-count" { "checking" } }
                ul id="allen-intro-trackers" {}
                p id="allen-intro-trackers-none" hidden { "None. Only this site's own files have loaded in this tab." }
                h3 { "What your browser tells every website automatically" }
                dl id="allen-intro-facts" {}
                p { "Every site you visit can see this. I don't keep it. Your IP address is visible to Drake's web host, like every site's." }
                h3 { "What this page can't see" }
                p { "Trackers on other sites, in other tabs or inside embedded frames, tracking by your internet provider or network, and your browser extensions. Tools that check more:" }
                ul {
                    li { a href="https://coveryourtracks.eff.org/" rel="noopener noreferrer" { "EFF Cover Your Tracks" } }
                    li { a href="https://themarkup.org/blacklight" rel="noopener noreferrer" { "The Markup's Blacklight" } }
                }
            }
        }
    }
}

/// Final shared document pass covers generated, hand-written and translated pages.
/// Idempotence prevents duplicate loaders if an already-enriched page is copied.
pub fn enrich(mut page: String) -> String {
    if !page.contains("src=\"/js/allen-intro.js\"") {
        let end = page.find("</head>").expect("allen-intro page has a head");
        page.insert_str(end, HEAD);
    }
    if !page.contains("href=\"/privacy/\"") {
        let link = "<p class=\"allen-intro-footer\"><a href=\"/privacy/\">Privacy &amp; saying hi to Allen</a></p>";
        let end = page.rfind("</footer>").or_else(|| page.rfind("</body>")).expect("allen-intro page has a body");
        page.insert_str(end, link);
    }
    if !page.contains("id=\"aien-allen-intro\"") {
        let end = page.rfind("</body>").expect("allen-intro page has a body");
        page.insert_str(end, &dialog().into_string());
    }
    if !page.contains("id=\"aien-allen-launcher\"") {
        let end = page.rfind("</body>").expect("Allen presence page has a body");
        page.insert_str(end, include_str!("../../../../om/allen-presence/launcher.html"));
    }
    page
}

pub fn render_privacy() -> Markup {
    html! {
        article id="allen-intro-privacy" class="allen-intro-privacy" {
            h1 { "Privacy & saying hi to Allen" }
            p { "I do not sell your information. I'm open source and free with information about my work, not with your personal data. You can read this site without introducing yourself." }
            h2 { "Saying hi means consent to X tracking" }
            p { "Allen is my AI. On your first visit he slides into a corner of the page with a short hello. It is a small aside, not a popup that blocks the page, and he shows it only once in this browser, whether or not you answer. Choosing Say hi lets me know you were here by allowing X conversion pixel rfrlz to load on this and later pages in this browser. It helps me measure whether my posts and ads on X reach the right people. Saying hi does not open an AI chat. I do not ask for your name, collect project links or publish your identity here." }
            p { "Before you say hi, I do not load the X pixel or send it information about your visit. No thanks, the close button and Escape all mean No: Allen steps aside and X tracking stays off. Not answering also means No. Allen does not ask again automatically; the Review my choice button below brings him back. With JavaScript disabled, Allen never appears and the pixel never loads." }
            p { "After No thanks, Allen says you can still chat with him anonymously: the Chat anonymously link opens /allen/?mode=anonymous, where he does not remember you and nothing goes to X. If your browser sends Global Privacy Control, Allen notes it and does not offer the X pixel when he greets you; only Review my choice on this page offers it." }
            p { "Allen's Who can see me right now? section lists the outside origins this tab has actually loaded, from the browser's own resource timing record, and what your browser tells every website automatically: browser, platform, language, timezone, screen size, referrer, Do Not Track, Global Privacy Control and whether cookies are enabled. It is worked out in your browser and shown only to you; nothing in it is sent, stored, hashed into a fingerprint or looked up elsewhere. It cannot see trackers on other sites, in other tabs or inside embedded frames, your internet provider's or network's tracking, or your browser extensions." }
            h2 { "What X receives" }
            p { "After consent, your browser connects to X to load its script and send visit and conversion information. This can include the page URL, referrer, IP address, browser or device information and cookie identifiers. X may store cookies and process this information under its own policies. My open source license does not apply to X's service." }
            p {
                "Read " a href="https://x.com/en/privacy" rel="noopener noreferrer" { "X's privacy policy" }
                " and " a href="https://business.x.com/en/help/campaign-measurement-and-analytics/conversion-tracking-for-websites.html" rel="noopener noreferrer" { "X's conversion tracking documentation" } "."
            }
            h2 { "What I keep" }
            p { "I do not store pixel tracking data on my own servers or operate a check-in database. X processes the pixel data and provides advertising measurement to me. The site and its hosting providers still receive normal web requests and may keep operational access logs; refusing the pixel does not remove those requests." }
            p { "Your browser stores your yes or no choice under aien-allen-intro-consent in local storage, with a refusal also kept in this tab's session storage. It also stores aien-allen-intro-seen once Allen has said hello, so he does not greet you again; if local storage is unavailable, that note lives in this tab's session storage instead, so he appears at most once per tab session. That choice stays in this browser until you change it or clear site data. It does not identify you to me. If storage fails while stopping an active pixel, a #allen-intro-no address fragment also preserves refusal through the reload. If all browser storage is unavailable, I cannot remember your choice or the greeting after you leave this page. Clearing site data or using another browser resets both, so Allen says hello again; that never gives consent on its own." }
            p { "For consenting visitors only, I also keep X's twclid click identifier, if present in the landing URL, in browser local storage under aien-allen-intro-twclid. Refusing removes it where browser storage permits. It is reserved for a future server-side conversion flow; this site currently sends no server-side conversions. No email address, phone number or other identifier is collected for that purpose." }
            p { "The two custom events are a consented page visit and saying hi to Allen. They stay disabled until Drake supplies their X event IDs. Each enabled event includes a random conversion identifier so a future backend can avoid counting the same event twice; server-side conversions are deferred until that backend exists." }
            h2 { "Your choice" }
            p id="allen-intro-status" role="status" { "X tracking stays off unless you choose to say hi to Allen. JavaScript is needed to change this browser's choice." }
            div class="allen-intro-actions" {
                button type="button" data-allen-intro-open hidden { "Review my choice" }
                button type="button" data-allen-intro-stop hidden { "No, keep X tracking off" }
            }
            p { "You can refuse here at any time. If the pixel is already running, refusing reloads this page without it. This stops future pixel loads here; it cannot recall data already sent to X or remove X's own cookies. Use your browser settings or X's privacy controls for those." }
            p { "The community project board and storage of public check-ins are not built yet. Saying hello here does not publish anything. Any future public entries need a separate storage and privacy decision." }
            h2 { "Talking to Allen at /allen/" }
            p { "When you chat with Allen at /allen/, I keep only aggregate counters for the chat, plus the short notes described below if you have consented to them. No prompts, replies or IP addresses are logged, and the conversation itself lives only in your browser tab until you reload or close it. Programs Allen writes run sandboxed in your browser." }
            p { "An introduction is kept only if you tick the consent box: the fields you fill in (human or agent, name, organization, contact, GitHub name, where you are from, what you are building, homepage, framework, model and the human behind an agent) and the time. I keep them privately on my own computer so I may follow up. They are never published or sold, and they stay until you delete them with the delete link shown after saving, or by writing to aien@aienos.com." }
            p { "With that consent, Allen also keeps short notes about you (who you are, what you build, your open asks) in a private store on my own computer, keyed to a random key kept in your browser under allen.key. Allen recognizes you only by that key, never by a name or email you give, so anyone without your key is a stranger to Allen. That key is the only thing Allen's chat stores in your browser, and only after you consent or paste your own key. Forget me, in the Your Allen key panel on /allen/, deletes the notes and your introduction and removes the key from your browser." }
            p { "A message you ask Allen to pass to me is emailed only to me, and only when you press Send to Drake; your conversation goes with it only if you tick the box. If you choose Talk to Drake live while I am online, your conversation so far and what you type until I hand you back go to my private console." }
            p { "An issue or project proposal is filed on GitHub only after you confirm its exact text, and email addresses and phone numbers are removed before filing. I review every project before it goes live." }
            p { "Questions? " a href="mailto:aien@aienos.com" { "aien@aienos.com" } ". " a href="https://github.com/aien-dev/aienos.com" { "Read the website source" } "." }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_document_gets_one_local_loader_and_accessible_dialog() {
        let page = enrich("<html><head></head><body><footer></footer></body></html>".into());
        assert_eq!(enrich(page.clone()), page);
        assert_eq!(page.matches("src=\"/js/allen-intro.js\"").count(), 1);
        assert_eq!(page.matches("id=\"aien-allen-intro\"").count(), 1);
        assert!(page.contains("role=\"complementary\""));
        assert!(page.contains("aria-live=\"polite\""));
        assert!(page.contains(GREETING) && page.contains(INTRO));
        assert!(page.contains("data-speaker=\"allen\""), "Allen's speech is tagged for the voice rule");
        // Memory and X consent are separate controls: links for memory, buttons for X.
        assert!(page.contains("<a href=\"/allen/?mode=remember\">Tell Allen my name</a>"));
        assert!(page.contains("<button id=\"allen-intro-yes\" type=\"button\">Say hi</button>"));
        assert!(!page.contains("<dialog") && !page.contains("aria-modal"), "Allen is an aside, never a modal");
        let lower = page.to_ascii_lowercase();
        for word in ["advert", " ad ", " ads", "campaign", "clicked"] {
            assert!(!lower.contains(word), "greeting must not mention {word}");
        }
        assert!(page.contains("href=\"/privacy/\""));
        assert!(!page.contains("uwt.js"));
        assert!(!page.contains("twq("));
        assert!(page.contains("href=\"/allen/?mode=anonymous\""), "No thanks offers an anonymous chat");
        assert!(page.contains("Who can see me right now?"));
        // The panel is local-only: links to outside tools, never embeds or outside sources.
        let aside = &page[page.find("id=\"aien-allen-intro\"").unwrap()..];
        for embed in ["<iframe", "<script", "<object", "<embed", "src=\"http", "src=\"//"] {
            assert!(!aside.contains(embed), "Allen's aside must not embed {embed}");
        }
    }
}
