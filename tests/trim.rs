use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn trim_dry_run_prints_copy_recipe_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.mp4");
    let output = dir.path().join("out.mp4");
    fs::write(&input, b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "trim",
            "in.mp4",
            "--from",
            "30",
            "--to",
            "105",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "trim");
    assert_eq!(v["output"], "out.mp4");
    assert_eq!(
        v["ffmpeg"],
        serde_json::json!([
            "ffmpeg", "-y", "-ss", "30", "-to", "105", "-i", "in.mp4", "-c", "copy", "out.mp4"
        ])
    );
    assert!(
        !output.exists(),
        "dry-run must not write the output file"
    );
}

#[test]
fn trim_without_output_fails_with_json_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["trim", "in.mp4", "--from", "30", "--to", "105"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "trim");
}

#[test]
fn trim_refuses_in_place_and_leaves_input_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.mp4");
    fs::write(&input, b"original-bytes").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "trim",
            "in.mp4",
            "--from",
            "30",
            "--to",
            "105",
            "-o",
            "in.mp4",
            "--dry-run",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(fs::read(&input).unwrap(), b"original-bytes");
}

#[test]
fn trim_missing_input_fails_and_writes_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.mp4");

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "trim",
            "in.mp4",
            "--from",
            "30",
            "--to",
            "105",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert!(!output.exists(), "must not write output when input is missing");
}
