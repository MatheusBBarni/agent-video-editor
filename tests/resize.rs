use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn copy_only_resize_dry_run_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "resize",
            "in.mp4",
            "--preset",
            "tiktok",
            "-o",
            "out.mp4",
            "--copy-only",
            "--dry-run",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
}
