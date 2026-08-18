use assert_cmd::Command;
use serde_json::Value;

#[test]
fn doctor_reports_ffmpeg_and_ffprobe_versions() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .args(["doctor"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");

    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "doctor");
    assert_eq!(v["ffmpeg_found"], true);
    assert_eq!(v["ffprobe_found"], true);

    let ffmpeg_version = v["ffmpeg_version"].as_str().expect("ffmpeg_version string");
    let ffprobe_version = v["ffprobe_version"]
        .as_str()
        .expect("ffprobe_version string");
    assert!(
        ffmpeg_version.chars().any(|c| c.is_ascii_digit()),
        "ffmpeg_version should contain a version number, got {ffmpeg_version}"
    );
    assert!(
        ffprobe_version.chars().any(|c| c.is_ascii_digit()),
        "ffprobe_version should contain a version number, got {ffprobe_version}"
    );
}
