mod common;

use common::{ave_json, ffmpeg_available};
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
