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
