use assert_cmd::Command;
use serde_json::Value;

#[test]
fn schema_prints_json_schema_for_run_plans() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .args(["schema"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert!(v.get("type").is_some(), "schema needs type: {v}");
    assert!(v.get("$schema").is_some(), "schema needs $schema: {v}");
    assert!(
        v.pointer("/properties/steps").is_some(),
        "schema needs properties.steps: {v}"
    );
}

#[test]
fn schema_rejects_unknown_op() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .args(["schema"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    let ops = v
        .pointer("/properties/steps/items/properties/op/enum")
        .and_then(|e| e.as_array())
        .expect("schema must list allowed ops");
    let allowed: Vec<&str> = ops.iter().filter_map(|x| x.as_str()).collect();
    assert!(
        !allowed.contains(&"nope"),
        "schema must reject unknown op nope: {allowed:?}"
    );
    assert!(
        allowed.contains(&"trim"),
        "schema should allow trim: {allowed:?}"
    );
}
