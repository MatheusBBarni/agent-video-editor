use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn convert_gif_dry_run_is_two_pass_and_leaves_no_palette() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["convert", "in.mp4", "-o", "out.gif", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "convert");
    assert!(
        stdout.contains("palettegen"),
        "gif convert must plan palettegen: {stdout}"
    );
    assert!(
        stdout.contains("paletteuse"),
        "gif convert must plan paletteuse: {stdout}"
    );
    assert!(
        !dir.path().join("palette.png").exists(),
        "dry-run must not leave palette.png"
    );
    assert!(!dir.path().join("out.gif").exists());
}
