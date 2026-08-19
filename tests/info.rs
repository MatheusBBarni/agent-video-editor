mod common;

use assert_cmd::Command;
use common::{ffmpeg, require_ffmpeg, write_fixture};
use serde_json::Value;
use std::path::Path;

fn write_rotated_fixture(path: &Path) {
    let src = path
        .parent()
        .expect("fixture parent")
        .join("src-unrotated.mp4");
    write_fixture(&src, 2.0, (320, 240), true);
    ffmpeg(&[
        "-y",
        "-display_rotation",
        "90",
        "-i",
        src.to_str().unwrap(),
        "-c",
        "copy",
        path.to_str().unwrap(),
    ]);
}

fn info_json(dir: &tempfile::TempDir, input: &str) -> Value {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["info", input])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    serde_json::from_str(stdout.trim()).expect("stdout must be JSON")
}

#[test]
fn info_reports_duration_and_resolution() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"), 2.0, (320, 240), true);
    let v = info_json(&dir, "clip.mp4");

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
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"), 2.0, (320, 240), true);
    let v = info_json(&dir, "clip.mp4");

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
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("silent.mp4"), 2.0, (320, 240), false);
    let v = info_json(&dir, "silent.mp4");

    assert_eq!(v["ok"], true);
    assert_eq!(v["has_audio"], false);
    assert_eq!(v["audio_codec"], "");
    assert_eq!(v["has_video"], true);
}

#[test]
fn info_swaps_display_size_when_rotate_is_90() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_rotated_fixture(&dir.path().join("rotated.mp4"));
    let v = info_json(&dir, "rotated.mp4");

    assert_eq!(v["ok"], true);
    assert_eq!(v["rotate_deg"], 90);
    assert_eq!(v["width"], 320);
    assert_eq!(v["height"], 240);
    assert_eq!(v["display_width"], 240);
    assert_eq!(v["display_height"], 320);
}
