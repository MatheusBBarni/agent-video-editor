mod common;

use common::ave_json;
use std::fs;

#[test]
fn volume_db_minus_six_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "volume",
            "in.mp4",
            "--db",
            "-6",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "volume");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.iter().any(|a| a.contains("volume=-6dB")),
        "volume filter must be volume=-6dB: {argv:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());
}
