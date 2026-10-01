use maud::{html, Markup};

/// Opening line of the closed-loop section. Kept as a constant so the
/// build verifier and the tests check the same words the page shows.
pub const INTRO_LEAD: &str = "AIENOS is an attempt to build persistent machine intelligence as a scientific process rather than a model invocation.";

pub const SLOGAN: &str =
    "Weights suggest. Programs explain. Verification decides. Evidence teaches. The Turing keeps score.";

/// The loop stages, in order. The last stage hands back to AIEN.
pub const STAGES: [(&str, &str); 8] = [
    ("Atlas", "awakens"),
    ("AIENOS", "persists"),
    ("AIEN", "suggests"),
    ("Omega", "formalizes and verifies"),
    ("FORGE", "realizes"),
    ("Reality", "execute and observe"),
    ("The Turing", "what was learned?"),
    ("Cortex", "remembers"),
];

/// Boundary enforcers. They sit around the loop and are not stages.
pub const ENFORCERS: [(&str, &str); 3] = [
    ("AEGIS", "Checks what the agents generate from outside the loop. It changes nothing and decides no policy."),
    ("ARGUS", "Observes what runs and can narrow one permission that is being misused. It never widens its own authority."),
    ("Capability authority", "Gives each part of the loop only the powers it holds a capability for, and nothing more."),
];

pub const QUESTIONS: [(&str, &str, Option<&str>); 3] = [
    ("Turing gain", "Did it learn anything?", None),
    ("Turing yield", "What did it cost?", Some("not started")),
    ("Search/verification gap", "Did it make future discovery easier?", None),
];

pub fn render_closed_loop() -> Markup {
    html! {
        section id="closed-loop" class="closed-loop" aria-label="The AIENOS closed loop" {
            div class="container" {
                div class="closed-loop-intro" {
                    div class="badge badge-green" style="margin-bottom: 12px;" {
                        "THE CLOSED LOOP"
                    }
                    p {
                        (INTRO_LEAD)
                        " A model suggests what to try. Programs explain each suggestion in a form that can be checked, and verification decides what is accepted. Evidence from reality is scored, and memory carries what was learned into the next round."
                    }
                }

                div class="closed-loop-frame" aria-label="Boundary enforcers around the loop" {
                    div class="closed-loop-frame-label" {
                        "Boundary enforcers (around the loop, not stages)"
                    }
                    ul class="closed-loop-enforcers" {
                        @for (name, guard) in ENFORCERS.iter() {
                            li {
                                strong { (name) }
                                " "
                                span { (guard) }
                            }
                        }
                    }

                    ol class="closed-loop-stages" aria-label="Loop stages, in order" {
                        @for (name, verb) in STAGES.iter() {
                            li {
                                span class="closed-loop-name" { (name) }
                                span class="closed-loop-verb" { (verb) }
                            }
                        }
                    }
                    p class="closed-loop-return" {
                        span aria-hidden="true" { "\u{21BA} " }
                        "back to AIEN"
                    }
                }

                div class="closed-loop-questions" {
                    h3 { "Three research questions" }
                    ul {
                        @for (name, question, note) in QUESTIONS.iter() {
                            li {
                                strong { (name) ":" }
                                " "
                                (question)
                                @if let Some(n) = note {
                                    " "
                                    em { "(" (n) ")" }
                                }
                            }
                        }
                    }
                }

                p class="closed-loop-slogan" { (SLOGAN) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_loop_section_has_required_parts() {
        let s = render_closed_loop().into_string();
        assert!(s.contains("aria-label=\"The AIENOS closed loop\""));
        assert!(s.contains(INTRO_LEAD));
        assert!(s.contains(SLOGAN));
        assert!(s.contains("<ol class=\"closed-loop-stages\""));
        assert!(s.contains("back to AIEN"));
        assert!(s.contains("not started"));
        for (name, _) in STAGES.iter() {
            assert!(s.contains(name), "missing stage {name}");
        }
        for (name, _) in ENFORCERS.iter() {
            assert!(s.contains(name), "missing enforcer {name}");
        }
        assert!(!s.contains('\u{2014}'), "em dash in closed-loop section");
        assert!(!s.contains('\u{2013}'), "en dash in closed-loop section");
    }

    #[test]
    fn stages_are_in_loop_order() {
        let s = render_closed_loop().into_string();
        let ol_start = s.find("<ol").expect("stages must be an ordered list");
        let ol_end = s[ol_start..].find("</ol>").expect("ol closes") + ol_start;
        let ol = &s[ol_start..ol_end];
        let mut last = 0;
        for (name, verb) in STAGES.iter() {
            let needle = format!("<span class=\"closed-loop-name\">{name}</span>");
            let at = ol[last..].find(&needle).unwrap_or_else(|| panic!("stage {name} out of order")) + last;
            assert!(ol[at..].contains(verb));
            last = at + needle.len();
        }
        // The loop starts at Atlas and the return marker comes after the list.
        assert_eq!(STAGES[0].0, "Atlas");
        assert_eq!(STAGES[7].0, "Cortex");
        assert!(s[ol_end..].contains("back to AIEN"));
    }

    #[test]
    fn enforcers_are_outside_the_stage_list() {
        let s = render_closed_loop().into_string();
        let ol_start = s.find("<ol").unwrap();
        let ol_end = s[ol_start..].find("</ol>").unwrap() + ol_start;
        let ol = &s[ol_start..ol_end];
        for (name, _) in ENFORCERS.iter() {
            assert!(!ol.contains(name), "enforcer {name} must not be a stage");
        }
    }
}
