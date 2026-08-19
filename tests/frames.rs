mod common;

use common::{ave_json, require_ffmpeg, write_clip};
use std::fs;

#[test]
fn frames_at_two_times_dry_run_lists_paths_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "frames",
            "in.mp4",
            "--at",
            "1,2",
            "-o",
            "review",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "frames");
    assert_eq!(v["output"], "review");
    let frames = v["frames"].as_array().expect("frames");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0]["at"], "1");
    assert_eq!(frames[0]["path"], "review/t-1.jpg");
    assert_eq!(frames[1]["at"], "2");
    assert_eq!(frames[1]["path"], "review/t-2.jpg");
    assert!(
        v.get("ffmpeg").is_some() || v.get("passes").is_some(),
        "dry-run must show argv: {v}"
    );
    assert!(!dir.path().join("review").exists());
    assert!(!dir.path().join("review/t-1.jpg").exists());
    assert!(!dir.path().join("review/t-2.jpg").exists());
    assert!(
        v.get("sheet").is_none(),
        "no --sheet means no sheet key: {v}"
    );
}

#[test]
fn frames_at_and_every_conflict() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let (ok, v) = ave_json(
        &dir,
        &[
            "frames",
            "in.mp4",
            "--at",
            "1",
            "--every",
            "30",
            "-o",
            "review",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["error"], "conflicting_fields");
    assert_eq!(v["op"], "frames");
}

#[test]
fn frames_missing_at_and_every_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let (ok, v) = ave_json(&dir, &["frames", "in.mp4", "-o", "review", "--dry-run"]);
    assert!(!ok);
    assert_eq!(v["error"], "missing_field");
    assert_eq!(v["op"], "frames");
}

#[test]
fn frames_sheet_dry_run_includes_sheet_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let (ok, v) = ave_json(
        &dir,
        &[
            "frames",
            "in.mp4",
            "--at",
            "1",
            "--sheet",
            "sheet.jpg",
            "-o",
            "review",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["sheet"], "sheet.jpg");
    assert!(!dir.path().join("sheet.jpg").exists());
}

#[test]
fn frames_missing_output_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let (ok, v) = ave_json(&dir, &["frames", "in.mp4", "--at", "1"]);
    assert!(!ok);
    assert_eq!(v["error"], "missing_output");
    assert_eq!(v["op"], "frames");
}

#[test]
fn frames_copy_only_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let (ok, v) = ave_json(
        &dir,
        &[
            "--copy-only",
            "frames",
            "in.mp4",
            "--at",
            "1",
            "-o",
            "review",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["error"], "copy_only");
    assert_eq!(v["op"], "frames");
}

#[test]
fn frames_unsupported_in_run() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{"steps":[{"op":"frames","input":"in.mp4","at":"1","output":"review"}]}"#,
    )
    .unwrap();
    let (ok, v) = ave_json(&dir, &["run", "plan.json", "--dry-run"]);
    assert!(!ok);
    assert_eq!(v["error"], "unsupported_in_run");
}

#[test]
fn frames_every_30_on_one_second_writes_one_still() {
    if !require_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    write_clip(&dir.path().join("in.mp4"));
    let (ok, v) = ave_json(&dir, &["frames", "in.mp4", "--every", "30", "-o", "review"]);
    assert!(ok, "{v}");
    let frames = v["frames"].as_array().expect("frames");
    assert_eq!(frames.len(), 1, "floor(1/30)+1 stills: {v}");
    assert_eq!(frames[0]["at"], "0");
    assert_eq!(frames[0]["path"], "review/t-0.jpg");
    assert!(dir.path().join("review/t-0.jpg").exists());
}
