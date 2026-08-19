mod common;

use common::{ave_json, require_ffmpeg, write_clip};
use std::fs;

#[test]
fn crop_bottom_40_dry_run_uses_locked_crop_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "crop",
            "in.mp4",
            "--bottom",
            "40",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "crop");
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
    assert!(vf.contains("crop="), "crop must use crop=: {vf}");
    assert!(
        vf.contains("ih-40"),
        "bottom 40 must lock height to ih-40: {vf}"
    );
    assert!(!vf.contains("pad="), "crop must not pad: {vf}");
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn crop_without_edges_fails_missing_field() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(&dir, &["crop", "in.mp4", "-o", "out.mp4", "--dry-run"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "crop");
    assert_eq!(v["error"], "missing_field");
    assert!(v.get("ffmpeg").is_none());
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn crop_bottom_larger_than_height_fails_bad_range() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_clip(&dir.path().join("in.mp4"));

    let (ok, v) = ave_json(
        &dir,
        &[
            "crop",
            "in.mp4",
            "--bottom",
            "240",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok, "{v}");
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "crop");
    assert_eq!(v["error"], "bad_range");
    assert!(v.get("ffmpeg").is_none());
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn crop_refuses_in_place_and_missing_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"original").unwrap();

    let (ok, missing) = ave_json(&dir, &["crop", "in.mp4", "--bottom", "40"]);
    assert!(!ok);
    assert_eq!(missing["error"], "missing_output");
    assert_eq!(missing["op"], "crop");

    let (ok, inplace) = ave_json(
        &dir,
        &[
            "crop",
            "in.mp4",
            "--bottom",
            "40",
            "-o",
            "in.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(inplace["error"], "in_place");
    assert_eq!(fs::read(dir.path().join("in.mp4")).unwrap(), b"original");
}

#[test]
fn run_crop_dry_run_accepts_bottom() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{"steps":[{"op":"crop","input":"in.mp4","bottom":40,"output":"out.mp4"}]}"#,
    )
    .unwrap();

    let (ok, v) = ave_json(&dir, &["run", "plan.json", "--dry-run"]);
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["steps"][0]["op"], "crop");
    assert_eq!(v["steps"][0]["ok"], true);
    let argv: Vec<&str> = v["steps"][0]["ffmpeg"]
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
    assert!(vf.contains("crop="), "run crop must use crop=: {vf}");
    assert!(
        vf.contains("ih-40"),
        "run crop bottom 40 must lock height to ih-40: {vf}"
    );
    assert!(!vf.contains("pad="), "run crop must not pad: {vf}");
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn crop_copy_only_dry_run_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "crop",
            "in.mp4",
            "--bottom",
            "40",
            "-o",
            "out.mp4",
            "--copy-only",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "crop");
    assert_eq!(v["error"], "copy_only");
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn crop_unprobeable_real_run_fails_ffprobe() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(&dir, &["crop", "in.mp4", "--bottom", "40", "-o", "out.mp4"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "crop");
    assert_eq!(v["error"], "ffprobe_failed");
    assert!(!dir.path().join("out.mp4").exists());
}
