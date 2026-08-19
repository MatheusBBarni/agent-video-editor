mod common;

use assert_cmd::Command;
use common::{ffmpeg_available, write_fixture, write_video_only_fixture};
use serde_json::Value;

#[test]
fn speed_factor_4_dry_run_chains_atempo() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("in.mp4"));
    let argv = speed_dry_run_argv(&dir, "in.mp4", "4");
    assert!(
        argv.iter().any(|a| a.contains("setpts=0.25*PTS")),
        "expected setpts=0.25*PTS in {argv:?}"
    );
    let atempo = argv
        .iter()
        .find(|a| a.contains("atempo="))
        .expect("expected atempo filter");
    assert!(
        atempo.contains("atempo=2.0,atempo=2.0") || atempo.matches("atempo=2").count() >= 2,
        "expected chained atempo for 4x: {atempo}"
    );
}

fn speed_dry_run_argv(dir: &tempfile::TempDir, input: &str, factor: &str) -> Vec<String> {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "speed",
            input,
            "--factor",
            factor,
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
fn speed_video_only_dry_run_omits_atempo() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_video_only_fixture(&dir.path().join("in.mp4"));
    let argv = speed_dry_run_argv(&dir, "in.mp4", "2");
    assert!(
        argv.iter().any(|a| a.contains("setpts=")),
        "expected setpts in {argv:?}"
    );
    assert!(
        !argv
            .iter()
            .any(|a| a.contains("atempo") || a == "-filter:a"),
        "video-only speed must omit atempo: {argv:?}"
    );
}

#[test]
fn speed_video_only_writes_output() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_video_only_fixture(&dir.path().join("in.mp4"));

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["speed", "in.mp4", "--factor", "2", "-o", "out.mp4"])
        .assert()
        .success();

    assert!(dir.path().join("out.mp4").exists());
}
