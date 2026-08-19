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
fn captions_srt_dry_run_uses_subtitles_filter() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(dir.path().join("sub.srt"), b"1\n00:00:00,000 --> 00:00:01,000\nHi\n").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "captions",
            "in.mp4",
            "--srt",
            "sub.srt",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "captions");
    let argv = argv_of(&v);
    let vf = argv
        .windows(2)
        .find(|w| w[0] == "-vf")
        .map(|w| w[1])
        .expect("expected -vf");
    assert!(
        vf.contains("subtitles="),
        "captions must burn subtitles=: {vf}"
    );
    assert!(vf.contains("sub.srt"), "srt path must be in the filter: {vf}");
    assert!(!dir.path().join("out.mp4").exists());
}
