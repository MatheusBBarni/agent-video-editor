use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn speed_factor_4_dry_run_chains_atempo() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "speed",
            "in.mp4",
            "--factor",
            "4",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "speed");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
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
