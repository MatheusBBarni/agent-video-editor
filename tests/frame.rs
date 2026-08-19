use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn ave_json(dir: &tempfile::TempDir, args: &[&str]) -> (bool, Value) {
    let output = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    (output.status.success(), v)
}

fn argv_of(v: &Value) -> Vec<&str> {
    v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect()
}

#[test]
fn frame_at_one_dry_run_extracts_single_still() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "frame",
            "in.mp4",
            "--at",
            "1",
            "-o",
            "poster.jpg",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "frame");
    let argv = argv_of(&v);
    assert!(
        argv.windows(2)
            .any(|w| w == ["-frames:v", "1"] || w == ["-vframes", "1"]),
        "frame must request one video frame: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| *a == "1"),
        "seek time 1 must appear in argv: {argv:?}"
    );
    assert!(
        argv.contains(&"poster.jpg"),
        "output must appear in argv: {argv:?}"
    );
    assert!(!dir.path().join("poster.jpg").exists());
}

#[test]
fn frame_without_output_fails_missing_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(&dir, &["frame", "in.mp4", "--at", "1"]);
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "frame");
    assert_eq!(v["error"], "missing_output");
}
