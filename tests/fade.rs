mod common;

use common::ave_json;
use std::fs;

#[test]
fn fade_in_out_dry_run_emits_both_fades() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "fade",
            "in.mp4",
            "--in",
            "0.5",
            "--out",
            "0.5",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "fade");
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
    assert!(vf.contains("fade="), "fade must use fade=: {vf}");
    assert!(
        vf.contains("t=in") && vf.contains("d=0.5"),
        "fade in 0.5s: {vf}"
    );
    assert!(vf.contains("t=out"), "fade out must be present: {vf}");
    assert!(!dir.path().join("out.mp4").exists());
}
