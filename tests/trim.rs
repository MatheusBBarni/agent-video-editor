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
    assert!(!output.exists(), "dry-run must not write the output file");
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
    assert!(
        !output.exists(),
        "must not write output when input is missing"
    );
}

#[test]
fn trim_no_overwrite_fails_when_output_exists() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"input-bytes").unwrap();
    let output = dir.path().join("out.mp4");
    fs::write(&output, b"existing-output").unwrap();

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
            "--no-overwrite",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(fs::read(&output).unwrap(), b"existing-output");
}

#[test]
fn trim_accurate_dry_run_reencodes_instead_of_copy() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

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
            "--accurate",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(
        v["ffmpeg"],
        serde_json::json!([
            "ffmpeg",
            "-y",
            "-accurate_seek",
            "-ss",
            "30",
            "-to",
            "105",
            "-i",
            "in.mp4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "23",
            "-preset",
            "medium",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
            "out.mp4"
        ])
    );
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_fixture(path: &std::path::Path, duration_s: u32) {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=duration={duration_s}:size=320x240:rate=30"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=1000:duration={duration_s}"),
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn trim_accurate_write_lands_on_requested_window() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"), 5);

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "trim",
            "clip.mp4",
            "--from",
            "1",
            "--to",
            "2",
            "--accurate",
            "-o",
            "out.mp4",
        ])
        .assert()
        .success();

    assert!(
        dir.path().join("out.mp4").exists(),
        "accurate trim must write the output file"
    );

    let info = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", "out.mp4"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&info.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("info json");
    let duration = v["duration_s"].as_f64().expect("duration_s");
    let max_err = 1.0 / 30.0 + 0.01;
    assert!(
        (duration - 1.0).abs() < max_err,
        "expected ~1.0s accurate cut, got {duration}"
    );
}

#[test]
fn trim_writes_shorter_playable_file() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"), 5);

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "trim", "clip.mp4", "--from", "1", "--to", "3", "-o", "out.mp4",
        ])
        .assert()
        .success();

    assert!(
        dir.path().join("out.mp4").exists(),
        "trim must write the output file"
    );

    let info = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", "out.mp4"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&info.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("info json");
    let duration = v["duration_s"].as_f64().expect("duration_s");
    assert!(
        (1.5..2.5).contains(&duration),
        "expected ~2s cut, got {duration}"
    );
}
