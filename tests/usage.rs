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
