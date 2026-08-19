mod common;

use common::ave_json;
use std::fs;

#[test]
fn rotate_90_dry_run_uses_transpose() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "rotate",
            "in.mp4",
            "--deg",
            "90",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "rotate");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    let vf = argv
        .windows(2)
        .find(|w| w[0] == "-vf")
        .map(|w| w[1])
        .expect("expected -vf");
    assert!(
        vf.contains("transpose="),
        "rotate 90 must re-encode with transpose=: {vf}"
    );
    assert!(
        !vf.contains("rotate="),
        "must not be metadata-only rotate=: {vf}"
    );
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn rotate_45_fails_bad_range() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "rotate",
            "in.mp4",
            "--deg",
            "45",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "rotate");
    assert_eq!(v["error"], "bad_range");
    assert!(v.get("ffmpeg").is_none());
    assert!(!dir.path().join("out.mp4").exists());
}
