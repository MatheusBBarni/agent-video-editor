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

#[test]
fn run_trim_duration_uses_t_not_to() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let plan = r#"{"steps":[{"op":"trim","input":"in.mp4","from":"10","duration":"5","output":"out.mp4"}]}"#;

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
    let argv = v["steps"][0]["ffmpeg"].as_array().expect("ffmpeg argv");
    assert!(
        argv.iter().any(|x| x == "-t"),
        "duration must become ffmpeg -t"
    );
    assert!(
        argv.iter().any(|x| x == "5"),
        "duration value must appear in argv"
    );
    assert!(
        !argv.iter().any(|x| x == "-to"),
        "duration must not emit ffmpeg -to"
    );
}

#[test]
fn run_dry_run_accepts_concat_after_trims() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{
          "steps": [
            {"op": "trim", "input": "in.mp4", "from": "0", "to": "10", "output": "a.mp4"},
            {"op": "trim", "input": "in.mp4", "from": "20", "to": "30", "output": "b.mp4"},
            {"op": "concat", "inputs": ["a.mp4", "b.mp4"], "output": "out.mp4"}
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
    assert_eq!(v["steps"].as_array().unwrap().len(), 3);
    assert_eq!(v["steps"][2]["op"], "concat");
}

#[test]
fn run_dry_run_accurate_trim_uses_accurate_seek_recipe() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{
          "steps": [
            {
              "op": "trim",
              "input": "in.mp4",
              "from": "1",
              "to": "2",
              "accurate": true,
              "output": "out.mp4"
            }
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
    assert_eq!(
        v["steps"][0]["ffmpeg"],
        serde_json::json!([
            "ffmpeg",
            "-y",
            "-accurate_seek",
            "-ss",
            "1",
            "-to",
            "2",
            "-i",
            "in.mp4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "23",
            "-preset",
            "medium",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
            "out.mp4"
        ])
    );
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

#[test]
fn run_trim_to_and_duration_conflict_fails_before_any_step() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{
          "steps": [
            {"op": "trim", "input": "in.mp4", "from": "0", "to": "10", "output": "a.mp4"},
            {"op": "trim", "input": "in.mp4", "from": "10", "to": "20", "duration": "5", "output": "b.mp4"}
          ]
        }"#,
    )
    .unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "plan.json", "--dry-run"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "conflicting_fields");
    assert!(
        v.get("steps").is_none(),
        "conflict must fail before running any step"
    );
    assert!(
        !dir.path().join("a.mp4").exists(),
        "must not write step 1 output when a later step conflicts"
    );
}

#[test]
fn run_trim_without_to_or_duration_fails_missing_field() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let plan = r#"{"steps":[{"op":"trim","input":"in.mp4","from":"10","output":"out.mp4"}]}"#;

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "-", "--dry-run"])
        .write_stdin(plan)
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "missing_field");
}

#[test]
fn run_trim_numeric_duration_uses_t_not_to() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let plan = r#"{"steps":[{"op":"trim","input":"in.mp4","from":"10","duration":5,"output":"out.mp4"}]}"#;

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
    let argv = v["steps"][0]["ffmpeg"].as_array().expect("ffmpeg argv");
    assert!(
        argv.iter().any(|x| x == "-t"),
        "numeric duration must become ffmpeg -t"
    );
    assert!(
        argv.iter().any(|x| x == "5" || x == "5.0"),
        "numeric duration must appear as 5 or 5.0"
    );
    assert!(
        !argv.iter().any(|x| x == "-to"),
        "numeric duration must not emit ffmpeg -to"
    );
}
