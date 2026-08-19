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

#[test]
fn resize_tiktok_dry_run_scale_and_pads() {
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
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "resize");
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
        vf.contains("1080:1920"),
        "tiktok filter should target 1080x1920: {vf}"
    );
    assert!(
        vf.contains("force_original_aspect_ratio=decrease"),
        "should preserve aspect: {vf}"
    );
    assert!(vf.contains("pad="), "should pad: {vf}");
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_video_only_fixture(path: &std::path::Path) {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=320x240:rate=30",
            "-an",
            "-c:v",
            "libx264",
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

fn resize_dry_run_argv(dir: &tempfile::TempDir, input: &str) -> Vec<String> {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "resize",
            input,
            "--preset",
            "square",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn resize_video_only_dry_run_omits_audio_copy() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_video_only_fixture(&dir.path().join("in.mp4"));
    let argv = resize_dry_run_argv(&dir, "in.mp4");
    assert!(
        !argv.windows(2).any(|w| w == ["-c:a", "copy"]),
        "video-only resize must omit -c:a copy: {argv:?}"
    );
}

#[test]
fn resize_video_only_writes_output() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_video_only_fixture(&dir.path().join("in.mp4"));

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["resize", "in.mp4", "--preset", "square", "-o", "out.mp4"])
        .assert()
        .success();

    assert!(dir.path().join("out.mp4").exists());
}

#[test]
fn resize_without_dry_run_does_not_fake_success() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["resize", "in.mp4", "--preset", "tiktok", "-o", "out.mp4"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert!(!dir.path().join("out.mp4").exists());
}
