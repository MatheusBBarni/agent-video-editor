mod common;

use common::{ave_json, ffmpeg_available, write_video_only_fixture};
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

#[test]
fn detect_unknown_kind_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(&dir, &["detect", "clip.mp4", "--kind", "nope"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["error"], "unknown_kind");
}

#[test]
fn detect_missing_input_fails() {
    let dir = tempfile::tempdir().unwrap();

    let (ok, v) = ave_json(&dir, &["detect", "missing.mp4", "--kind", "silence"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["error"], "missing_input");
}

#[test]
fn detect_silence_dry_run_prints_silencedetect_and_empty_segments() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &["detect", "clip.mp4", "--kind", "silence", "--dry-run"],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["kind"], "silence");
    assert_eq!(v["input"], "clip.mp4");
    let segments = v["segments"].as_array().expect("segments");
    assert!(segments.is_empty(), "dry-run segments must be empty: {v}");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.iter().any(|a| a.contains("silencedetect")),
        "argv must contain silencedetect: {argv:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());
}

fn ffmpeg(args: &[&str]) {
    let status = StdCommand::new("ffmpeg")
        .args(args)
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn write_silence_gap_fixture(path: &Path) {
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=duration=3:size=320x240:rate=30",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=1000:duration=3,volume=enable='between(t,1,2)':volume=0",
        "-c:v",
        "libx264",
        "-c:a",
        "aac",
        "-pix_fmt",
        "yuv420p",
        path.to_str().unwrap(),
    ]);
}

#[test]
fn detect_silence_finds_known_gap() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_silence_gap_fixture(&dir.path().join("clip.mp4"));

    let (ok, v) = ave_json(&dir, &["detect", "clip.mp4", "--kind", "silence"]);
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["kind"], "silence");
    let segments = v["segments"].as_array().expect("segments");
    assert!(
        !segments.is_empty(),
        "expected at least one silence segment: {v}"
    );
    let overlaps_gap = segments.iter().any(|seg| {
        let start = seg["start_s"].as_f64().expect("start_s");
        let end = seg["end_s"].as_f64().expect("end_s");
        assert!(start < end, "start_s must be < end_s: {seg}");
        assert_eq!(seg["kind"], "silence");
        start < 2.0 && end > 1.0
    });
    assert!(
        overlaps_gap,
        "expected a segment overlapping silence at 1s-2s: {v}"
    );
}

#[test]
fn detect_silence_on_video_only_fails_no_audio() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_video_only_fixture(&dir.path().join("clip.mp4"));

    let (ok, v) = ave_json(&dir, &["detect", "clip.mp4", "--kind", "silence"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["error"], "no_audio");
}

#[test]
fn detect_unsupported_in_run() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{"steps":[{"op":"detect","input":"clip.mp4","kind":"silence"}]}"#,
    )
    .unwrap();

    let (ok, v) = ave_json(&dir, &["run", "plan.json", "--dry-run"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "unsupported_in_run");
}

#[test]
fn detect_black_dry_run_prints_blackdetect() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &["detect", "clip.mp4", "--kind", "black", "--dry-run"],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["kind"], "black");
    let segments = v["segments"].as_array().expect("segments");
    assert!(segments.is_empty(), "dry-run segments must be empty: {v}");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.iter().any(|a| a.contains("blackdetect")),
        "argv must contain blackdetect: {argv:?}"
    );
}

#[test]
fn detect_scenes_dry_run_prints_scdet() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("clip.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &["detect", "clip.mp4", "--kind", "scenes", "--dry-run"],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "detect");
    assert_eq!(v["kind"], "scenes");
    let segments = v["segments"].as_array().expect("segments");
    assert!(segments.is_empty(), "dry-run segments must be empty: {v}");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.iter().any(|a| a.contains("scdet")),
        "argv must contain scdet: {argv:?}"
    );
}

fn write_black_then_white_fixture(path: &Path) {
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=320x240:d=1:r=30",
        "-f",
        "lavfi",
        "-i",
        "color=c=white:s=320x240:d=1:r=30",
        "-filter_complex",
        "[0:v][1:v]concat=n=2:v=1:a=0",
        "-an",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        path.to_str().unwrap(),
    ]);
}

#[test]
fn detect_black_finds_known_gap() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_black_then_white_fixture(&dir.path().join("clip.mp4"));

    let (ok, v) = ave_json(&dir, &["detect", "clip.mp4", "--kind", "black"]);
    assert!(ok, "{v}");
    assert_eq!(v["kind"], "black");
    let segments = v["segments"].as_array().expect("segments");
    let overlaps = segments.iter().any(|seg| {
        let start = seg["start_s"].as_f64().expect("start_s");
        let end = seg["end_s"].as_f64().expect("end_s");
        assert!(start < end, "start_s must be < end_s: {seg}");
        assert_eq!(seg["kind"], "black");
        start < 1.0 && end > 0.0
    });
    assert!(overlaps, "expected a black segment overlapping 0s-1s: {v}");
}

#[test]
fn detect_scenes_splits_known_cut() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_black_then_white_fixture(&dir.path().join("clip.mp4"));

    let (ok, v) = ave_json(&dir, &["detect", "clip.mp4", "--kind", "scenes"]);
    assert!(ok, "{v}");
    assert_eq!(v["kind"], "scenes");
    let segments = v["segments"].as_array().expect("segments");
    assert!(
        segments.len() >= 2,
        "expected scene spans around the cut: {v}"
    );
    for seg in segments {
        let start = seg["start_s"].as_f64().expect("start_s");
        let end = seg["end_s"].as_f64().expect("end_s");
        assert!(start < end, "start_s must be < end_s: {seg}");
        assert_eq!(seg["kind"], "scenes");
    }
}
