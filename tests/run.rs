use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn run_dry_run_allows_missing_intermediate_outputs() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{
          "steps": [
            {"op": "trim", "input": "in.mp4", "from": "0", "to": "10", "output": "a.mp4"},
            {"op": "trim", "input": "a.mp4", "from": "0", "to": "5", "output": "b.mp4"}
          ]
        }"#,
    )
    .unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "plan.json", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "run");
    let steps = v["steps"].as_array().expect("steps");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0]["ok"], true);
    assert_eq!(steps[0]["op"], "trim");
    assert_eq!(steps[1]["ok"], true);
    assert_eq!(steps[1]["op"], "trim");
    assert!(steps[1]["ffmpeg"].as_array().unwrap().contains(&Value::from("a.mp4")));
    assert!(!dir.path().join("a.mp4").exists());
    assert!(!dir.path().join("b.mp4").exists());
}
