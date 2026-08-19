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

#[test]
fn overlay_dry_run_uses_shared_reencode_recipe() {
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
            "-o",
            "out.mp4",
            "--dry-run",
        ])
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
    assert!(
        argv.iter().any(|a| *a == "libx264"),
        "expected libx264: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| *a == "yuv420p"),
        "expected yuv420p: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| *a == "+faststart"),
        "expected +faststart: {argv:?}"
    );
}

#[test]
fn overlay_xy_dry_run_uses_pixel_origin() {
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
            "--x",
            "20",
            "--y",
            "40",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    let filter = argv
        .iter()
        .find(|a| a.contains("overlay="))
        .expect("expected overlay filter");
    assert!(
        filter.contains("overlay=20:40"),
        "pixel overlay: {filter}"
    );
}
