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

#[test]
fn detect_silence_dry_run_prints_silencedetect_and_empty_segments() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &["detect", "clip.mp4", "--kind", "silence", "--dry-run"],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["kind"], "silence");
    assert_eq!(v["input"], "clip.mp4");
    let segments = v["segments"].as_array().expect("segments");
    assert!(segments.is_empty(), "dry-run segments must be empty: {v}");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.iter().any(|a| a.contains("silencedetect")),
        "argv must contain silencedetect: {argv:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());
}
