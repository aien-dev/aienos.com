//! Agent replies often quote the human they answer. Human text is never
//! published, so any span of an agent reply that repeats a recent human message
//! from the same session is withheld before the reply reaches the privacy screen.
//!
//! Matching ignores case, spacing, punctuation and quote markup: both sides are
//! reduced to their letters and digits, and a span matches when at least
//! `WINDOW` consecutive letters or digits appear in a human message. `WINDOW`
//! letters or digits are roughly 40 characters of ordinary prose.
use std::{collections::{HashSet, VecDeque}, hash::{DefaultHasher, Hash, Hasher}};

pub const WINDOW: usize = 32;
/// Human messages kept per session for matching.
const KEEP: usize = 200;
/// Letters and digits kept per human message for matching.
const MESSAGE_CHARS: usize = 65536;
/// A reply with fewer letters or digits left after elision is dropped.
const MIN_LEFT: usize = 16;
pub const WITHHELD: &str = "[quoted human message withheld]";

/// Letters and digits, lowercased, each with the byte range of its source char.
fn normalized(text: &str) -> Vec<(char, usize, usize)> {
    let mut out = Vec::new();
    for (start, c) in text.char_indices() {
        if !c.is_alphanumeric() { continue; }
        let end = start + c.len_utf8();
        for lower in c.to_lowercase() { out.push((lower, start, end)); }
    }
    out
}
fn shingle(chars: &[(char, usize, usize)]) -> u64 {
    let mut h = DefaultHasher::new();
    for (c, _, _) in chars { c.hash(&mut h); }
    h.finish()
}

#[derive(Default)]
pub struct Quotes { messages: VecDeque<Vec<u64>>, shingles: HashSet<u64> }
impl Quotes {
    pub fn add(&mut self, human: &str) {
        let mut chars = normalized(human);
        chars.truncate(MESSAGE_CHARS);
        if chars.len() < WINDOW { return; }
        let hashes: Vec<u64> = chars.windows(WINDOW).map(shingle).collect();
        self.shingles.extend(hashes.iter().copied());
        self.messages.push_back(hashes);
        if self.messages.len() > KEEP {
            self.messages.pop_front();
            self.shingles = self.messages.iter().flatten().copied().collect();
        }
    }
    /// The reply with every quoted human span replaced, or None when too little
    /// of the agent's own text remains to be worth publishing.
    pub fn elide(&self, reply: &str) -> Option<String> {
        let chars = normalized(reply);
        let mut spans: Vec<(usize, usize)> = Vec::new();
        if !self.shingles.is_empty() && chars.len() >= WINDOW {
            for (i, window) in chars.windows(WINDOW).enumerate() {
                if !self.shingles.contains(&shingle(window)) { continue; }
                let (start, end) = (chars[i].1, chars[i + WINDOW - 1].2);
                match spans.last_mut() { Some(last) if start <= last.1 => last.1 = last.1.max(end), _ => spans.push((start, end)) }
            }
        }
        if spans.is_empty() { return Some(reply.to_owned()); }
        let mut out = String::with_capacity(reply.len());
        let mut at = 0;
        for (start, end) in spans { out.push_str(&reply[at..start]); out.push_str(WITHHELD); at = end; }
        out.push_str(&reply[at..]);
        let left = normalized(&out.replace(WITHHELD, "")).len();
        (left >= MIN_LEFT).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const HUMAN: &str = "Please take my chat logs off the website. I've changed my mind.";
    #[test] fn elides_a_verbatim_quote_and_keeps_the_agents_own_text() {
        let mut q = Quotes::default(); q.add(HUMAN);
        let reply = format!("You asked: \"{HUMAN}\" I removed every human message from the public feed and kept agent replies.");
        let out = q.elide(&reply).unwrap();
        assert!(!out.contains("chat logs off the website"), "{out}");
        assert!(out.contains(WITHHELD) && out.contains("I removed every human message"), "{out}");
    }
    #[test] fn matching_ignores_case_spacing_and_blockquote_markup() {
        let mut q = Quotes::default(); q.add(HUMAN);
        let reply = "> PLEASE take my chat\n> logs off the  website.\n\nDone: the receiver now refuses human records, and the page shows agents only.";
        let out = q.elide(reply).unwrap();
        assert!(!out.to_lowercase().contains("logs off the"), "{out}");
    }
    #[test] fn short_overlaps_and_unrelated_text_survive() {
        let mut q = Quotes::default(); q.add(HUMAN);
        let reply = "I changed my mind about the cache layout and rebuilt the website index.";
        assert_eq!(q.elide(reply).as_deref(), Some(reply));
    }
    #[test] fn a_reply_that_is_only_a_quote_is_dropped() {
        let mut q = Quotes::default(); q.add(HUMAN);
        assert!(q.elide(&format!("\"{HUMAN}\" Okay.")).is_none());
    }
    #[test] fn old_messages_age_out() {
        let mut q = Quotes::default(); q.add(HUMAN);
        for i in 0..KEEP { q.add(&format!("filler message number {i:04} with enough letters to count as human text")); }
        let reply = format!("{HUMAN} and some agent words after it to keep");
        assert_eq!(q.elide(&reply).as_deref(), Some(reply.as_str()));
    }
}
