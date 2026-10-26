use maud::{html, Markup};

pub const HEAD: &str = "<link rel=\"stylesheet\" href=\"/assets/logbook.css\"><script defer src=\"/js/logbook.js\"></script>";

fn dialog() -> Markup {
    html! {
        dialog id="aien-logbook" role="dialog" aria-modal="true" aria-labelledby="logbook-title" aria-describedby="logbook-intro logbook-consent" {
            h2 id="logbook-title" { "Sign the logbook?" }
            p id="logbook-intro" {
                "Hey, if you just clicked an ad, I just want you to know that if you didn't intend to click an ad, that's cool. Just click No and this popup will close. Nothing will be tracked by our X pixel. Your browser may not let us close a page we didn't open."
            }
            p id="logbook-consent" {
                "But if you did want to see it, do you mind if I track you and know you were here? Signing the logbook means consenting to the X conversion pixel. X receives information about your visit to help us reach the right people. It is not a public check-in."
            }
            div class="logbook-actions" {
                button id="logbook-no" type="button" autofocus { "No" }
                button id="logbook-sign" type="button" { "Sign the logbook" }
            }
            p { a href="/privacy/" { "Privacy, what X receives, and your choice" } }
        }
    }
}

/// Final shared document pass covers generated, hand-written and translated pages.
/// Idempotence prevents duplicate loaders if an already-enriched page is copied.
pub fn enrich(mut page: String) -> String {
    if !page.contains("src=\"/js/logbook.js\"") {
        let end = page.find("</head>").expect("logbook page has a head");
        page.insert_str(end, HEAD);
    }
    if !page.contains("href=\"/privacy/\"") {
        let link = "<p class=\"logbook-footer\"><a href=\"/privacy/\">Privacy &amp; the logbook</a></p>";
        let end = page.rfind("</footer>").or_else(|| page.rfind("</body>")).expect("logbook page has a body");
        page.insert_str(end, link);
    }
    if !page.contains("id=\"aien-logbook\"") {
        let end = page.rfind("</body>").expect("logbook page has a body");
        page.insert_str(end, &dialog().into_string());
    }
    page
}

pub fn render_privacy() -> Markup {
    html! {
        article id="logbook-privacy" class="logbook-privacy" {
            h1 { "Privacy & the logbook" }
            p { "We do not sell your information. We're open source and free with information about our work, not with your personal data. You can read this site without signing anything." }
            h2 { "Signing means consent to X tracking" }
            p { "Signing the logbook allows X conversion pixel rfrlz to load on this and later pages in this browser. It helps us understand whether our ads reach the right people and helps us broadcast our message to that audience. We do not collect names, project links or public check-ins here." }
            p { "Before you sign, we do not load the X pixel or send it information about your visit. No closes only the popup, not your tab, and refuses X tracking. We remember No and do not ask again automatically. Escape means No too. With JavaScript disabled, the pixel never loads." }
            h2 { "What X receives" }
            p { "After consent, your browser connects to X to load its script and send visit and conversion information. This can include the page URL, referrer, IP address, browser or device information and cookie identifiers. X may store cookies and process this information under its own policies. Our open source license does not apply to X's service." }
            p {
                "Read " a href="https://x.com/en/privacy" rel="noopener noreferrer" { "X's privacy policy" }
                " and " a href="https://business.x.com/en/help/campaign-measurement-and-analytics/conversion-tracking-for-websites.html" rel="noopener noreferrer" { "X's conversion tracking documentation" } "."
            }
            h2 { "What we keep" }
            p { "We do not store pixel tracking data on our own servers or operate a check-in database. X processes the pixel data and provides advertising measurement to us. The site and its hosting providers still receive normal web requests and may keep operational access logs; refusing the pixel does not remove those requests." }
            p { "Your browser stores only your yes or no choice under aien-logbook-consent in local storage, with a refusal also kept in this tab's session storage. That choice stays in this browser until you change it or clear site data. It does not identify you to us. If storage fails while stopping an active pixel, a #logbook-no address fragment also preserves refusal through the reload. If all browser storage is unavailable, we cannot remember your choice after you leave this page. Clearing site data or using another browser resets the choice; it never gives consent on its own." }
            p { "For consenting visitors only, we also keep the ad's twclid click identifier, if present in the landing URL, in browser local storage under aien-logbook-twclid. Refusing removes it where browser storage permits. It is reserved for a future server-side conversion flow; this site currently sends no server-side conversions. No email address, phone number or other identifier is collected for that purpose." }
            h2 { "Your choice" }
            p id="logbook-status" role="status" { "X tracking stays off unless you sign the logbook. JavaScript is needed to change this browser's choice." }
            div class="logbook-actions" {
                button type="button" data-logbook-open hidden { "Review my choice" }
                button type="button" data-logbook-stop hidden { "No, keep X tracking off" }
            }
            p { "You can refuse here at any time. If the pixel is already running, refusing reloads this page without it. This stops future pixel loads here; it cannot recall data already sent to X or remove X's own cookies. Use your browser settings or X's privacy controls for those." }
            p { "The public project logbook and community board are not built yet. Signing here does not publish anything. Any future public entries need a separate storage and privacy decision." }
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
        assert_eq!(page.matches("src=\"/js/logbook.js\"").count(), 1);
        assert_eq!(page.matches("id=\"aien-logbook\"").count(), 1);
        assert!(page.contains("role=\"dialog\""));
        assert!(page.contains("href=\"/privacy/\""));
        assert!(!page.contains("uwt.js"));
        assert!(!page.contains("twq("));
    }
}
