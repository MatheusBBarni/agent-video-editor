use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn overlay_top_right_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(dir.path().join("logo.png"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "overlay",
            "in.mp4",
            "--image",
            "logo.png",
            "--position",
            "top-right",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "overlay");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(argv.contains(&"logo.png"));
    let filter = argv
        .iter()
        .find(|a| a.contains("overlay="))
        .expect("expected overlay filter");
    assert!(
        filter.contains("overlay=W-w-10:10"),
        "top-right overlay: {filter}"
    );
}
