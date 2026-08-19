use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn compress_dry_run_uses_crf_23_medium() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["compress", "in.mp4", "-o", "out.mp4", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "compress");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.windows(2).any(|w| w == ["-crf", "23"]),
        "default crf 23: {argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w == ["-preset", "medium"]),
        "default preset medium: {argv:?}"
    );
    assert!(
        !argv.windows(2).any(|w| w == ["-c:a", "copy"]),
        "unprobeable compress dry-run omits -c:a copy: {argv:?}"
    );
}

#[test]
fn compress_dry_run_uses_shared_reencode_recipe() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["compress", "in.mp4", "-o", "out.mp4", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(argv.iter().any(|a| *a == "libx264"), "expected libx264: {argv:?}");
    assert!(argv.iter().any(|a| *a == "+faststart"), "expected +faststart: {argv:?}");
    assert!(
        argv.windows(2).any(|w| w == ["-crf", "23"]),
        "default crf 23: {argv:?}"
    );
}

