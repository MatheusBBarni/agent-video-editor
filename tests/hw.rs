use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn dry_run_argv(args: &[&str]) -> (Value, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    let argv = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    (v, argv)
}

#[test]
fn hw_videotoolbox_resize_dry_run_swaps_encoder() {
    let (_v, argv) = dry_run_argv(&[
        "--hw",
        "videotoolbox",
        "resize",
        "in.mp4",
        "--preset",
        "square",
        "-o",
        "out.mp4",
        "--dry-run",
    ]);
    assert!(
        argv.iter().any(|a| a == "h264_videotoolbox"),
        "expected h264_videotoolbox: {argv:?}"
    );
    assert!(
        !argv.iter().any(|a| a == "libx264"),
        "videotoolbox must not keep libx264: {argv:?}"
    );
}
