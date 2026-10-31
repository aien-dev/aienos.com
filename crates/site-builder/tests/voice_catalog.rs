//! Aien's pre-recorded voice clips (public/aien/voice): every clip in catalog.json exists, matches
//! its recorded sha256 and size, carries a license and source, and no unlisted audio ships.
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn voice_catalog_matches_every_shipped_clip() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../public/aien/voice");
    let cat: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("catalog.json")).expect("catalog.json")).expect("catalog parses");
    let clips = cat["clips"].as_array().expect("clips array");
    assert_eq!(cat["count"].as_u64(), Some(clips.len() as u64), "count field");
    assert!(clips.len() >= 500, "at least 500 clips, found {}", clips.len());
    let mut listed = BTreeSet::new();
    for c in clips {
        let id = c["id"].as_str().expect("id");
        let file = c["file"].as_str().expect("file");
        assert_eq!(file, format!("/aien/voice/{id}.mp3"), "{id}: first-party path");
        assert!(listed.insert(format!("{id}.mp3")), "{id}: duplicate id");
        let bytes = fs::read(dir.join(format!("{id}.mp3"))).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(bytes.len() as u64, c["bytes"].as_u64().expect("bytes"), "{id}: size");
        let sha: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(sha, c["sha256"].as_str().expect("sha256"), "{id}: sha256");
        assert!(c["license"].as_str().map_or(false, |l| !l.is_empty()), "{id}: license");
        assert!(c["source"]["voice_id"].as_str().is_some() && c["source"]["model"].as_str().is_some(), "{id}: source");
        let text = c["text"].as_str().expect("text");
        assert!(!text.contains('\u{2013}') && !text.contains('\u{2014}'), "{id}: no en or em dash");
    }
    let shipped: BTreeSet<String> = fs::read_dir(&dir)
        .expect("voice dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".mp3"))
        .collect();
    assert_eq!(shipped, listed, "every shipped clip is in the catalog and vice versa");
}
