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
    assert!(
        steps[1]["ffmpeg"]
            .as_array()
            .unwrap()
            .contains(&Value::from("a.mp4"))
    );
    assert!(!dir.path().join("a.mp4").exists());
    assert!(!dir.path().join("b.mp4").exists());
}

#[test]
fn run_reads_plan_from_stdin() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let plan =
        r#"{"steps":[{"op":"trim","input":"in.mp4","from":"0","to":"10","output":"a.mp4"}]}"#;

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "-", "--dry-run"])
        .write_stdin(plan)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["steps"][0]["op"], "trim");
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_fixture(path: &std::path::Path) {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=3:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=3",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn run_stops_on_failure_and_keeps_earlier_output() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"));
    fs::write(
        dir.path().join("plan.json"),
        r#"{
          "steps": [
            {"op": "trim", "input": "clip.mp4", "from": "0", "to": "2", "output": "a.mp4"},
            {"op": "trim", "input": "missing.mp4", "from": "0", "to": "1", "output": "b.mp4"},
            {"op": "trim", "input": "clip.mp4", "from": "0", "to": "1", "output": "c.mp4"}
          ]
        }"#,
    )
    .unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "plan.json"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["failed_step"], 1);
    assert!(
        dir.path().join("a.mp4").exists(),
        "step 1 output must be kept"
    );
    assert!(!dir.path().join("c.mp4").exists(), "step 3 must not run");
}

#[test]
fn run_unknown_op_fails_before_any_step() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"));
    fs::write(
        dir.path().join("plan.json"),
        r#"{
          "steps": [
            {"op": "trim", "input": "clip.mp4", "from": "0", "to": "2", "output": "a.mp4"},
            {"op": "nope", "input": "clip.mp4", "output": "b.mp4"}
          ]
        }"#,
    )
    .unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "plan.json"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert!(
        !dir.path().join("a.mp4").exists(),
        "unknown op must fail before running any step"
    );
}
