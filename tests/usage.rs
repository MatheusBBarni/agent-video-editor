use assert_cmd::Command;
use serde_json::Value;

#[test]
fn unknown_flag_on_trim_is_json_usage() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .args(["trim", "--nope"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "usage");
    assert!(
        v["op"] == "trim" || v["op"] == "ave",
        "op should be trim or ave, got {}",
        v["op"]
    );
}

#[test]
fn missing_subcommand_is_json_usage() {
    let assert = Command::cargo_bin("ave").unwrap().assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "usage");
    assert_eq!(v["op"], "ave");
}

#[test]
fn help_stays_human_and_succeeds() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        serde_json::from_str::<Value>(stdout.trim()).is_err(),
        "help must not be a JSON object"
    );
    assert!(
        stdout.contains("Usage") || stdout.contains("ave"),
        "help should mention Usage or ave: {stdout}"
    );
}

#[test]
fn version_stays_human_and_prints_package_version() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        serde_json::from_str::<Value>(stdout.trim()).is_err(),
        "--version must not be a JSON object"
    );
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "--version should include the Cargo.toml version: {stdout}"
    );
}

#[test]
fn changelog_has_shipped_and_unreleased() {
    let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/CHANGELOG.md"));
    assert!(
        text.contains("0.1.0"),
        "CHANGELOG.md must mention shipped 0.1.0"
    );
    assert!(
        text.contains("Unreleased"),
        "CHANGELOG.md must have an Unreleased section"
    );
}

#[test]
fn ffmpeg_failure_message_is_short_and_not_a_banner() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["resize", "in.mp4", "--preset", "square", "-o", "out.mp4"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "ffmpeg_failed");
    let message = v["message"].as_str().expect("message");
    assert!(
        message.len() <= 2048,
        "ffmpeg message must be <= 2048 bytes, got {}",
        message.len()
    );
    assert!(
        !message.starts_with("ffmpeg version"),
        "message must not be the ffmpeg banner: {message}"
    );
}
