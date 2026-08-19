mod common;

use common::ave_json;
use std::fs;

#[test]
fn frames_at_two_times_dry_run_lists_paths_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "frames",
            "in.mp4",
            "--at",
            "1,2",
            "-o",
            "review",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "frames");
    assert_eq!(v["output"], "review");
    let frames = v["frames"].as_array().expect("frames");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0]["at"], "1");
    assert_eq!(frames[0]["path"], "review/t-1.jpg");
    assert_eq!(frames[1]["at"], "2");
    assert_eq!(frames[1]["path"], "review/t-2.jpg");
    assert!(
        v.get("ffmpeg").is_some() || v.get("passes").is_some(),
        "dry-run must show argv: {v}"
    );
    assert!(!dir.path().join("review").exists());
    assert!(!dir.path().join("review/t-1.jpg").exists());
    assert!(!dir.path().join("review/t-2.jpg").exists());
}
