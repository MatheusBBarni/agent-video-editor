mod common;

use common::ave_json;
use std::fs;

#[test]
fn text_lower_third_dry_run_uses_locked_drawtext() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "text",
            "in.mp4",
            "--text",
            "Hello",
            "--position",
            "lower-third",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "text");
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
    assert!(vf.contains("drawtext="), "text must use drawtext: {vf}");
    assert!(vf.contains("text='Hello'"), "locked text payload: {vf}");
    assert!(vf.contains("fontsize=48"), "locked fontsize: {vf}");
    assert!(
        vf.contains("x=(w-text_w)/2"),
        "locked horizontal center: {vf}"
    );
    assert!(vf.contains("y=h-th-80"), "locked lower-third y: {vf}");
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn text_from_without_to_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "text",
            "in.mp4",
            "--text",
            "Hello",
            "--from",
            "1",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "text");
    assert_eq!(v["error"], "missing_field");
}
