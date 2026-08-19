mod common;

use common::ave_json;
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
