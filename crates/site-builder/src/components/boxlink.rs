//! Every box on the site is a link to its evidence on GitHub.
//!
//! A box is any card-like element: a machine card, a speed row, a receipt
//! card, a status row, a research card, a ladder step, a hub card, a /now/
//! card, queue row or timeline entry. Each one carries exactly one
//! `a.box-link` in its footer, reading "Evidence on GitHub" with an arrow.
//! The CSS stretches that anchor over the whole box (`.box-link::after`
//! with `inset: 0`), so the whole box is the click target while inline
//! links inside the box stay above it and keep working. There are no
//! nested anchors anywhere.
//!
//! When a box has no evidence of its own on GitHub, it links to the source
//! file in aien-dev/aienos.com that holds its text and says "Source on
//! GitHub" instead, so the footer never claims evidence it does not have.
//!
//! `BOX_CLASSES` is the list of classes the build treats as boxes. A new
//! component must add its card class here to be checked; `check_boxes`
//! fails the build if any box on any generated page lacks its link.

use maud::{html, Markup};

/// The classes that make an element a box. Every element carrying one of
/// these on a generated page must contain an `a.box-link` to GitHub.
pub const BOX_CLASSES: &[&str] = &[
    "mx-card",
    "mx-row",
    "glow-box",
    "lineage-card",
    "lineage-role",
    "status-row",
    "live-row",
    "receipt",
    "rs-card",
    "rs-step",
    "rs-impl-item",
    "hub-card",
    "narr-card",
    "blog-card",
    "now-card",
    "now-queue-row",
    "now-tl",
    "try-card",
];

/// Where the box link points.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// A receipt, run, pull request, commit or file that is the evidence.
    Evidence,
    /// The source file of this site that holds the box's text. Honest when
    /// no evidence exists on GitHub yet.
    Source,
}

pub const GITHUB: &str = "https://github.com/";
pub const SITE_REPO_BLOB: &str = "https://github.com/aien-dev/aienos.com/blob/main/";

/// The blob URL of a file in this repository, for boxes that have no
/// evidence of their own.
pub fn source_href(path: &str) -> String {
    format!("{SITE_REPO_BLOB}{path}")
}

/// The tree URL of a folder in this repository (a paper's folder).
pub fn source_dir_href(path: &str) -> String {
    format!("https://github.com/aien-dev/aienos.com/tree/main/{path}")
}

/// The footer affordance, which the CSS stretches over the whole box.
pub fn foot(href: &str, target: Target) -> Markup {
    let label = match target {
        Target::Evidence => "Evidence on GitHub",
        Target::Source => "Source on GitHub",
    };
    foot_labelled(href, label, target)
}

/// The same with a custom label, for boxes whose evidence has a better
/// name ("Ledger line on GitHub").
pub fn foot_labelled(href: &str, label: &str, target: Target) -> Markup {
    debug_assert!(href.starts_with(GITHUB), "box link must point at GitHub: {href}");
    let class = match target {
        Target::Evidence => "box-link",
        Target::Source => "box-link box-link-source",
    };
    html! {
        a class=(class) href=(href) target="_blank" rel="noopener noreferrer" {
            (label) " " span class="box-arrow" aria-hidden="true" { "\u{2197}" }
        }
    }
}

/// Read the value of the first `class="..."` attribute in `tag`.
fn class_value(tag: &str) -> Option<&str> {
    let i = tag.find("class=\"")? + 7;
    let rest = &tag[i..];
    let j = rest.find('"')?;
    Some(&rest[..j])
}

fn href_value(tag: &str) -> Option<&str> {
    let i = tag.find("href=\"")? + 6;
    let rest = &tag[i..];
    let j = rest.find('"')?;
    Some(&rest[..j])
}

/// Counts the boxes on a rendered page and checks that every one of them
/// contains an `a.box-link` whose href is on GitHub. A box's region runs
/// from its opening tag to the next box's opening tag (boxes do not nest),
/// so a box whose link is missing, or whose link points anywhere but
/// GitHub, is reported with the first words of its tag.
pub fn check_boxes(page: &str) -> Result<usize, String> {
    // Every opening tag, with its position.
    let mut tags: Vec<(usize, &str)> = Vec::new();
    let mut i = 0;
    while let Some(off) = page[i..].find('<') {
        let start = i + off;
        let Some(len) = page[start..].find('>') else { break };
        let tag = &page[start..start + len + 1];
        if !tag.starts_with("</") && !tag.starts_with("<!") {
            tags.push((start, tag));
        }
        i = start + len + 1;
    }
    let is_box = |tag: &str| {
        class_value(tag)
            .map(|c| c.split_whitespace().any(|t| BOX_CLASSES.contains(&t)))
            .unwrap_or(false)
    };
    let box_starts: Vec<usize> = (0..tags.len()).filter(|&k| is_box(tags[k].1)).collect();
    let mut problems = Vec::new();
    for (n, &k) in box_starts.iter().enumerate() {
        let end = box_starts.get(n + 1).copied().unwrap_or(tags.len());
        let link = tags[k + 1..end].iter().find(|(_, tag)| {
            tag.starts_with("<a ")
                && class_value(tag)
                    .map(|c| c.split_whitespace().any(|t| t == "box-link"))
                    .unwrap_or(false)
        });
        match link.map(|(_, tag)| href_value(tag)) {
            Some(Some(h)) if h.starts_with(GITHUB) => {}
            Some(other) => problems.push(format!("box {} has a box-link off GitHub: {other:?}", short(tags[k].1))),
            None => problems.push(format!("box without a GitHub link: {}", short(tags[k].1))),
        }
    }
    if problems.is_empty() {
        Ok(box_starts.len())
    } else {
        Err(problems.join("\n"))
    }
}

fn short(tag: &str) -> &str {
    let n = tag.char_indices().nth(90).map(|(i, _)| i).unwrap_or(tag.len());
    &tag[..n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foot_is_one_anchor_to_github() {
        let s = foot("https://github.com/aien-dev/omega-mv/pull/88", Target::Evidence).into_string();
        assert!(s.starts_with("<a class=\"box-link\" href=\"https://github.com/aien-dev/omega-mv/pull/88\""));
        assert!(s.contains("Evidence on GitHub"));
        assert!(s.contains('\u{2197}'));
        assert_eq!(s.matches("<a ").count(), 1);
        let src = foot(&source_href("crates/site-builder/src/components/status.rs"), Target::Source).into_string();
        assert!(src.contains("box-link-source") && src.contains("Source on GitHub"));
        assert!(src.contains("https://github.com/aien-dev/aienos.com/blob/main/crates/site-builder/src/components/status.rs"));
    }

    #[test]
    fn check_counts_linked_boxes() {
        let page = r#"<div class="x"><article class="mx-card" id="a"><h3>A</h3><a href="/x">in</a><a class="box-link" href="https://github.com/aien-dev/x">Evidence</a></article><li class="hub-card"><a class="box-link box-link-source" href="https://github.com/aien-dev/aienos.com/blob/main/f.rs">Source</a></li></div>"#;
        assert_eq!(check_boxes(page), Ok(2));
        assert_eq!(check_boxes("<p>no boxes</p>"), Ok(0));
    }

    #[test]
    fn check_rejects_a_box_without_a_link() {
        let page = r#"<article class="mx-card"><a href="https://github.com/aien-dev/x">plain</a></article><article class="rs-card"><a class="box-link" href="https://github.com/y">ok</a></article>"#;
        let err = check_boxes(page).unwrap_err();
        assert!(err.contains("box without a GitHub link"), "{err}");
        assert!(err.contains("mx-card"), "{err}");
        assert!(!err.contains("rs-card"), "{err}");
    }

    #[test]
    fn check_rejects_a_link_off_github() {
        let page = r#"<li class="now-tl"><a class="box-link" href="/now/">here</a></li>"#;
        let err = check_boxes(page).unwrap_err();
        assert!(err.contains("off GitHub"), "{err}");
        let page = r#"<li class="now-tl"><a class="box-link">no href</a></li>"#;
        assert!(check_boxes(page).is_err());
    }

    #[test]
    fn a_link_in_the_next_box_does_not_count_for_this_one() {
        let page = r#"<div class="glow-box"></div><div class="glow-box"><a class="box-link" href="https://github.com/z">z</a></div>"#;
        assert!(check_boxes(page).is_err());
    }
}
