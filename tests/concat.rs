use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn concat_dry_run_uses_concat_demuxer_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.mp4"), b"a").unwrap();
    fs::write(dir.path().join("b.mp4"), b"b").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["concat", "a.mp4", "b.mp4", "-o", "out.mp4", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "concat");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.windows(2).any(|w| w == ["-f", "concat"]),
        "expected concat demuxer in {argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w == ["-safe", "0"]),
        "expected -safe 0 in {argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w == ["-c", "copy"]),
        "expected stream copy in {argv:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());

    let leftover: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        leftover.len(),
        2,
        "dry-run must not leave concat list or output: {leftover:?}"
    );
}
