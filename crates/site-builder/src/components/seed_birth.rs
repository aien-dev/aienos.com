use maud::{html, Markup};

const PIN: &str = "29b12760f4c962606411d6bf25032aedef063301";
const RECORD: &str = "https://github.com/aien-dev/omega-mv/blob/b30ba6cf729eb9295c7f8883cfff0519b90a63e1/docs/SEED_BIRTH.md";

pub fn render() -> Markup {
    let stages = [
        ("Instruction words", "I began with explicit AArch64 instruction words and an ELF executable header. The seed reads hexadecimal text and writes machine words. Its source names the Arm manual section beside each instruction. It is 408 bytes, not a C program.", "seed/rung0/seed0.hex"),
        ("The first bytes", "Before the seed can run, a small loader writes its bytes to disk using the existing host shell, printf and tr. No C compiler, assembler or linker is invoked in that step. This host dependency is named, not hidden.", "seed/rung0/load0.sh"),
        ("Self-rebuild", "The executable reads its own hex source and writes another executable. Both files must be byte-identical and match the recorded seed. A fresh replay passed. This establishes the tested reconstruction, not an absolute claim about every tool that ever touched the files.", "seed/rung0/gate0.sh"),
        ("Labels", "The next program, hex1, adds names for branch locations. That replaces manually counted jump distances. Seed0 builds it, then hex1 rebuilds itself to the same bytes. Missing and duplicate labels must be rejected.", "seed/rung1/hex1.hex"),
        ("Instruction names", "Hex1 builds asm2, an assembler with a readable table of instruction names and operand fields. Asm2 assembles its own named-instruction source back to the same bytes. C encoders and GNU tools are used separately to check answers, not to build this compiler chain.", "seed/rung2/asm2.s"),
        ("The systems language", "Om3 begins translating Omegese functions to instructions. Om4 grows that layer into whole programs with calls, fixed buffers, length-carrying arrays, checked arithmetic and input/output. Asm2 builds om4. Omegese was the low-level stepping stone, not the finished language announced in advance.", "seed/rung4/om4.s"),
        ("The self-hosting compiler", "The next compiler, omc.om, is written in the source language that om4 understands. Om4 compiles stage 1; stage 1 compiles stage 2; stage 2 compiles stage 3. The actual compiler source is compiled each time, and the outputs must agree byte for byte. A fixed point is not a proof of correctness by itself.", "seed/omc/omc.om"),
        ("Reproduce it", "The pinned bootstrap script reconstructs the chain and checks forward calls, mutual calls and rejected programs. A fresh AArch64 Linux replay finished with RC=0 and the expected compiler fingerprint. It still uses ordinary host file/text utilities; this is not a claim that the host operating system came from the seed.", "seed/omega-from-seed.sh"),
    ];
    html! {
        section id="seed-birth" class="om-section" {
            style { "#seed-birth .seed-steps{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:20px;margin:32px 0 44px}#seed-birth .om-part{position:relative;min-width:0}#seed-birth .om-source code{overflow-wrap:anywhere;white-space:normal}#seed-birth h3.om-h2{font-size:1.5rem}@media(max-width:700px){#seed-birth .seed-steps{grid-template-columns:1fr}}" }
            div class="container" {
                p class="om-kicker" { "How I made the language" }
                h2 class="om-h2" { "From explicit instruction words to a compiler written in Omega" }
                p class="om-sub" { "I want to account for the language from its first instruction, rather than keep C or Rust underneath a new name. I directed this build with AI coding agents. The code, failed checks and commit history are how a reader can inspect what was made." }
                p class="om-sub" { strong { "Available on a development branch. " } "The legacy C implementation and Rust-built services are not yet retired. This walkthrough describes the seed-born compiler at " code { "29b12760" } ", not a completed migration of the whole system." }
                p class="om-source" { a href=(RECORD) target="_blank" rel="noopener noreferrer" { "Full walkthrough, sources and reproduction commands on GitHub" } }
                div class="seed-steps" {
                    @for (i, (name, text, path)) in stages.iter().enumerate() {
                        article class="om-part" {
                            p class="om-kicker" { "Step " (i + 1) }
                            h3 class="om-part-name" { (name) }
                            p class="om-part-what" { (text) }
                            (super::boxlink::foot_labelled(&format!("https://github.com/aien-dev/omega-mv/blob/{PIN}/{path}"), "Source on GitHub", super::boxlink::Target::Source))
                        }
                    }
                }
                h3 class="om-h2" { "Why this is my implementation, and what I did not invent" }
                p class="om-sub" { "The project-specific work is the explicit seed, translator stages, Omega-written compiler and their checks. I did not invent the Arm instruction set, ELF, bootstrapping or self-hosting. Published specifications and earlier compiler work are credited in the walkthrough. AI coding agents helped implement and test it; I do not claim to have personally typed every instruction." }
                p class="om-sub" { "The purpose is a foundation I can inspect and rebuild, with native Omega replacing the old implementation after equivalent behavior is verified. Optional C output connects to other tools; it is not the required bootstrap or runtime of the seeded language. This does not prove universal hardware support, zero host dependencies, absence of bugs or an automatic speedup." }
                p class="om-source" { "Root fingerprint: " code { "aca00cf3677236809f0f223437398c6bb429e4c65f13d58762bcf96d13799474" } br; "Compiler fingerprint: " code { "191b776ef98821597c3e30655da6d680a9b66edbc9ceaca3c39c61ea6df098e9" } }
                p class="om-source" { a href="https://github.com/aien-dev/omega-mv/tree/b30ba6cf729eb9295c7f8883cfff0519b90a63e1/evidence/seed-birth-20261022" target="_blank" rel="noopener noreferrer" { "Fresh root and full-chain replay evidence on GitHub" } }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walkthrough_has_eight_source_linked_stages_and_honest_limits() {
        let page = render().into_string();
        assert!(page.contains("id=\"seed-birth\""));
        for step in ["Instruction words", "The first bytes", "Self-rebuild", "Labels", "Instruction names", "The systems language", "The self-hosting compiler", "Reproduce it"] {
            assert!(page.contains(step), "missing stage {step}");
        }
        assert_eq!(page.matches("class=\"om-part\"").count(), 8);
        assert_eq!(super::super::boxlink::check_boxes(&page).unwrap(), 8);
        for phrase in ["408 bytes", "AI coding agents", "host shell", "not yet retired", "not a proof of correctness", "Arm", "29b12760", "191b776e"] {
            assert!(page.contains(phrase), "missing provenance/limit {phrase}");
        }
        assert!(!page.contains('\u{2014}') && !page.contains('\u{2013}'));
        assert!(!page.contains("never touched C"));
    }

    #[test]
    fn omega_page_exposes_the_walkthrough_and_its_navigation() {
        let page = super::super::omega::render_omega_page().into_string();
        assert!(page.contains("href=\"#seed-birth\""));
        assert!(page.contains("id=\"seed-birth\""));
        assert!(page.contains("The production C/Rust migration is unfinished."));
        assert!(!page.contains("No vendor compiler, toolkit or runtime library is involved anywhere."));
    }
}
