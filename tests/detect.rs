mod common;

use common::ave_json;
use std::fs;

#[test]
fn detect_unknown_kind_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(&dir, &["detect", "clip.mp4", "--kind", "nope"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["error"], "unknown_kind");
}

#[test]
fn detect_missing_input_fails() {
    let dir = tempfile::tempdir().unwrap();

    let (ok, v) = ave_json(&dir, &["detect", "missing.mp4", "--kind", "silence"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["error"], "missing_input");
}
