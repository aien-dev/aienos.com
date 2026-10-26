use maud::{html, Markup};

pub const HEAD: &str = "<link rel=\"stylesheet\" href=\"/assets/allen-intro.css\"><script defer src=\"/js/allen-intro.js\"></script>";

fn dialog() -> Markup {
    html! {
        dialog id="aien-allen-intro" role="dialog" aria-modal="true" aria-labelledby="allen-intro-title" aria-describedby="allen-intro-intro allen-intro-consent" {
            h2 id="allen-intro-title" { "Say hello to Allen?" }
            p id="allen-intro-intro" {
                "Hey, you just clicked an ad. I just want you to know that if you didn't intend to click an ad, that's cool. Just click No and this popup will close. Nothing will be tracked by my X pixel. Your browser may not let me close a page I didn't open."
            }
            p id="allen-intro-consent" {
                "But if you did want to see it, do you mind if I track you and know you were here? Allen is my AI. Introduce yourself to Allen by saying hello below. Saying hello means you consent to the X conversion pixel, so X receives information about your visit and helps me reach the right people. This doesn't open a chat or publish your identity."
            }
            div class="allen-intro-actions" {
                button id="allen-intro-no" type="button" autofocus { "No" }
                button id="allen-intro-yes" type="button" { "Introduce yourself to Allen" }
            }
            p { a href="/privacy/" { "Privacy, what X receives, and your choice" } }
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
        let link = "<p class=\"allen-intro-footer\"><a href=\"/privacy/\">Privacy &amp; saying hello to Allen</a></p>";
        let end = page.rfind("</footer>").or_else(|| page.rfind("</body>")).expect("allen-intro page has a body");
        page.insert_str(end, link);
    }
    if !page.contains("id=\"aien-allen-intro\"") {
        let end = page.rfind("</body>").expect("allen-intro page has a body");
        page.insert_str(end, &dialog().into_string());
    }
    page
}

pub fn render_privacy() -> Markup {
    html! {
        article id="allen-intro-privacy" class="allen-intro-privacy" {
            h1 { "Privacy & saying hello to Allen" }
            p { "I do not sell your information. I'm open source and free with information about my work, not with your personal data. You can read this site without introducing yourself." }
            h2 { "Saying hello means consent to X tracking" }
            p { "Allen is Drake's AI. Choosing Introduce yourself to Allen lets me know you were here by allowing X conversion pixel rfrlz to load on this and later pages in this browser. It helps me understand whether my ads reach the right people and helps me broadcast my message to that audience. It does not open an AI chat. I do not ask for your name, collect project links or publish your identity here." }
            p { "Before you say hello, I do not load the X pixel or send it information about your visit. No closes only the popup, not your tab, and refuses X tracking. I remember No and do not ask again automatically. Escape means No too. With JavaScript disabled, the pixel never loads." }
            h2 { "What X receives" }
            p { "After consent, your browser connects to X to load its script and send visit and conversion information. This can include the page URL, referrer, IP address, browser or device information and cookie identifiers. X may store cookies and process this information under its own policies. My open source license does not apply to X's service." }
            p {
                "Read " a href="https://x.com/en/privacy" rel="noopener noreferrer" { "X's privacy policy" }
                " and " a href="https://business.x.com/en/help/campaign-measurement-and-analytics/conversion-tracking-for-websites.html" rel="noopener noreferrer" { "X's conversion tracking documentation" } "."
            }
            h2 { "What I keep" }
            p { "I do not store pixel tracking data on my own servers or operate a check-in database. X processes the pixel data and provides advertising measurement to me. The site and its hosting providers still receive normal web requests and may keep operational access logs; refusing the pixel does not remove those requests." }
            p { "Your browser stores only your yes or no choice under aien-allen-intro-consent in local storage, with a refusal also kept in this tab's session storage. That choice stays in this browser until you change it or clear site data. It does not identify you to me. If storage fails while stopping an active pixel, a #allen-intro-no address fragment also preserves refusal through the reload. If all browser storage is unavailable, I cannot remember your choice after you leave this page. Clearing site data or using another browser resets the choice; it never gives consent on its own." }
            p { "For consenting visitors only, I also keep the ad's twclid click identifier, if present in the landing URL, in browser local storage under aien-allen-intro-twclid. Refusing removes it where browser storage permits. It is reserved for a future server-side conversion flow; this site currently sends no server-side conversions. No email address, phone number or other identifier is collected for that purpose." }
            p { "The two custom events are a consented page visit and introduced to Allen. They stay disabled until Drake supplies their X event IDs. Each enabled event includes a random conversion identifier so a future backend can avoid counting the same event twice; server-side conversions are deferred until that backend exists." }
            h2 { "Your choice" }
            p id="allen-intro-status" role="status" { "X tracking stays off unless you choose to introduce yourself to Allen. JavaScript is needed to change this browser's choice." }
            div class="allen-intro-actions" {
                button type="button" data-allen-intro-open hidden { "Review my choice" }
                button type="button" data-allen-intro-stop hidden { "No, keep X tracking off" }
            }
            p { "You can refuse here at any time. If the pixel is already running, refusing reloads this page without it. This stops future pixel loads here; it cannot recall data already sent to X or remove X's own cookies. Use your browser settings or X's privacy controls for those." }
            p { "The community project board and storage of public check-ins are not built yet. Saying hello here does not publish anything. Any future public entries need a separate storage and privacy decision." }
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
        assert!(page.contains("role=\"dialog\""));
        assert!(page.contains("href=\"/privacy/\""));
        assert!(!page.contains("uwt.js"));
        assert!(!page.contains("twq("));
    }
}
