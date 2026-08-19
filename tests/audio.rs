mod common;

use assert_cmd::Command;
use common::{require_ffmpeg, write_clip, write_video_only_fixture};
use serde_json::Value;
use std::fs;

#[test]
fn extract_audio_mp3_dry_run_disables_video() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_clip(&dir.path().join("in.mp4"));

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["extract-audio", "in.mp4", "-o", "audio.mp3", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "extract-audio");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(argv.contains(&"-vn"), "expected -vn in {argv:?}");
    assert!(
        argv.contains(&"libmp3lame"),
        "mp3 extract should use libmp3lame: {argv:?}"
    );
    assert!(!dir.path().join("audio.mp3").exists());
}

#[test]
fn replace_audio_mute_dry_run_drops_audio() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "replace-audio",
            "in.mp4",
            "--mute",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "replace-audio");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(argv.contains(&"-an"), "mute should drop audio: {argv:?}");
    assert!(
        argv.windows(2).any(|w| w == ["-c:v", "copy"]),
        "mute should copy video: {argv:?}"
    );
}

#[test]
fn replace_audio_file_dry_run_maps_new_track() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(dir.path().join("voice.mp3"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "replace-audio",
            "in.mp4",
            "--audio",
            "voice.mp3",
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
    assert!(argv.contains(&"voice.mp3"));
    assert!(argv.contains(&"1:a:0"), "should map new audio: {argv:?}");
}

#[test]
fn extract_audio_video_only_fails_no_audio() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_video_only_fixture(&dir.path().join("in.mp4"));

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["extract-audio", "in.mp4", "-o", "a.mp3"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "no_audio");
}

#[test]
fn replace_audio_mute_and_audio_is_conflicting_flags() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(dir.path().join("a.mp3"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "replace-audio",
            "in.mp4",
            "--mute",
            "--audio",
            "a.mp3",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "conflicting_flags");
    assert!(!dir.path().join("out.mp4").exists());
}
