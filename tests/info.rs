use assert_cmd::Command;
use serde_json::Value;
use std::process::Command as StdCommand;

fn ffmpeg_available() -> bool {
    StdCommand::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_video_only_fixture(path: &std::path::Path) {
    let status = StdCommand::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=30",
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

fn write_fixture(path: &std::path::Path) {
    let status = StdCommand::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=2",
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
fn info_reports_duration_and_resolution() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("clip.mp4");
    write_fixture(&input);

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", "clip.mp4"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "info");
    assert_eq!(v["width"], 320);
    assert_eq!(v["height"], 240);
    let duration = v["duration_s"].as_f64().expect("duration_s");
    assert!(
        (1.5..2.5).contains(&duration),
        "expected ~2s duration, got {duration}"
    );
}

#[test]
fn info_reports_codecs_fps_audio_and_unrotated_display() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("clip.mp4");
    write_fixture(&input);

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", "clip.mp4"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    let video_codec = v["video_codec"].as_str().expect("video_codec");
    assert!(
        video_codec.starts_with("h264"),
        "expected h264 codec, got {video_codec}"
    );
    assert_eq!(v["audio_codec"], "aac");
    assert_eq!(v["has_video"], true);
    assert_eq!(v["has_audio"], true);
    let fps = v["fps"].as_str().expect("fps");
    assert!(
        fps == "30/1" || fps == "30",
        "expected 30/1 or 30 fps, got {fps}"
    );
    assert_eq!(v["rotate_deg"], 0);
    assert_eq!(v["width"], 320);
    assert_eq!(v["height"], 240);
    assert_eq!(v["display_width"], 320);
    assert_eq!(v["display_height"], 240);
    let duration = v["duration_s"].as_f64().expect("duration_s");
    assert!(
        (1.5..2.5).contains(&duration),
        "expected ~2s duration, got {duration}"
    );
}

#[test]
fn info_reports_empty_audio_on_video_only_file() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("silent.mp4");
    write_video_only_fixture(&input);

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", "silent.mp4"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    assert_eq!(v["ok"], true);
    assert_eq!(v["has_audio"], false);
    assert_eq!(v["audio_codec"], "");
    assert_eq!(v["has_video"], true);
}

fn write_rotated_fixture(path: &std::path::Path) {
    let dir = path.parent().expect("fixture parent");
    let src = dir.join("src-unrotated.mp4");
    write_fixture(&src);
    let status = StdCommand::new("ffmpeg")
        .args([
            "-y",
            "-display_rotation",
            "90",
            "-i",
            src.to_str().unwrap(),
            "-c",
            "copy",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn ffmpeg rotate remux");
    assert!(
        status.status.success(),
        "ffmpeg rotate remux failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn info_swaps_display_size_when_rotate_is_90() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("rotated.mp4");
    write_rotated_fixture(&input);

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", "rotated.mp4"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    assert_eq!(v["ok"], true);
    assert_eq!(v["rotate_deg"], 90);
    assert_eq!(v["width"], 320);
    assert_eq!(v["height"], 240);
    assert_eq!(v["display_width"], 240);
    assert_eq!(v["display_height"], 320);
}
