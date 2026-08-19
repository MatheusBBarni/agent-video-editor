mod common;

use assert_cmd::Command;
use common::{require_ffmpeg, write_clip, write_video_only_fixture};
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

#[test]
fn resize_square_dry_run_uses_shared_reencode_recipe() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let argv = resize_dry_run_argv(&dir, "in.mp4");
    assert!(
        argv.iter().any(|a| a == "libx264"),
        "expected libx264: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "yuv420p"),
        "expected yuv420p: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "+faststart"),
        "expected +faststart on mp4: {argv:?}"
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
    if !require_ffmpeg() {
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
    if !require_ffmpeg() {
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
fn resize_with_audio_dry_run_keeps_audio_copy() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_clip(&dir.path().join("in.mp4"));
    let argv = resize_dry_run_argv(&dir, "in.mp4");
    assert!(
        argv.windows(2).any(|w| w == ["-c:a", "copy"]),
        "resize with audio must keep -c:a copy: {argv:?}"
    );
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

fn resize_vf(dir: &tempfile::TempDir, args: &[&str]) -> String {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
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
    argv.windows(2)
        .find(|w| w[0] == "-vf")
        .map(|w| w[1].to_string())
        .expect("expected -vf")
}

#[test]
fn resize_tiktok_fit_crop_dry_run_fills_without_pad() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let vf = resize_vf(
        &dir,
        &[
            "resize",
            "in.mp4",
            "--preset",
            "tiktok",
            "--fit",
            "crop",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(
        vf.contains("1080:1920"),
        "tiktok crop should target 1080x1920: {vf}"
    );
    assert!(
        vf.contains("force_original_aspect_ratio=increase"),
        "crop should fill the frame: {vf}"
    );
    assert!(vf.contains("crop="), "crop fit should crop: {vf}");
    assert!(!vf.contains("pad="), "crop fit must not pad: {vf}");
}

#[test]
fn resize_tiktok_fit_pad_dry_run_keeps_pad_recipe() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let vf = resize_vf(
        &dir,
        &[
            "resize",
            "in.mp4",
            "--preset",
            "tiktok",
            "--fit",
            "pad",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(
        vf.contains("1080:1920"),
        "tiktok pad should target 1080x1920: {vf}"
    );
    assert!(
        vf.contains("force_original_aspect_ratio=decrease"),
        "pad should letterbox: {vf}"
    );
    assert!(vf.contains("pad="), "pad fit should pad: {vf}");
    assert!(!vf.contains("crop="), "pad fit must not crop: {vf}");
}

#[test]
fn resize_fit_stretch_width_height_dry_run_scales_without_pad() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let vf = resize_vf(
        &dir,
        &[
            "resize",
            "in.mp4",
            "--fit",
            "stretch",
            "--width",
            "640",
            "--height",
            "360",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(
        vf.contains("640:360"),
        "stretch should scale to 640x360: {vf}"
    );
    assert!(!vf.contains("pad="), "stretch must not pad: {vf}");
    assert!(!vf.contains("crop="), "stretch must not crop: {vf}");
    assert!(
        !vf.contains("force_original_aspect_ratio"),
        "stretch should not preserve aspect: {vf}"
    );
}

#[test]
fn resize_width_height_dry_run_scales_and_pads() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let vf = resize_vf(
        &dir,
        &[
            "resize",
            "in.mp4",
            "--width",
            "640",
            "--height",
            "360",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(
        vf.contains("640:360"),
        "explicit size should target 640x360: {vf}"
    );
    assert!(vf.contains("pad="), "default resize should pad: {vf}");
}

#[test]
fn resize_preset_and_size_conflict() {
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
            "--width",
            "640",
            "--height",
            "360",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "conflicting_fields");
}

#[test]
fn resize_preset_stretch_dry_run_scales_without_pad() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let vf = resize_vf(
        &dir,
        &[
            "resize",
            "in.mp4",
            "--preset",
            "tiktok",
            "--stretch",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(
        vf.contains("1080:1920"),
        "tiktok stretch should target 1080x1920: {vf}"
    );
    assert!(!vf.contains("pad="), "--stretch must not pad: {vf}");
    assert!(
        !vf.contains("force_original_aspect_ratio"),
        "--stretch should not preserve aspect: {vf}"
    );
}

#[test]
fn verbose_resize_dry_run_stays_json_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "--verbose",
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
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must stay JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "resize");
}

#[test]
fn progress_resize_writes_jsonl_on_stderr() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_clip(&dir.path().join("in.mp4"));

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "--progress",
            "resize",
            "in.mp4",
            "--preset",
            "square",
            "-o",
            "out.mp4",
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let found = stderr.lines().any(|line| {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return false;
        };
        v.get("progress").is_some() || v.get("time_s").is_some()
    });
    assert!(found, "expected JSONL progress on stderr: {stderr}");
}
