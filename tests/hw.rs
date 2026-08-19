use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn dry_run_argv(args: &[&str]) -> (Value, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    let argv = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    (v, argv)
}

#[test]
fn hw_videotoolbox_resize_dry_run_swaps_encoder() {
    let (_v, argv) = dry_run_argv(&[
        "--hw",
        "videotoolbox",
        "resize",
        "in.mp4",
        "--preset",
        "square",
        "-o",
        "out.mp4",
        "--dry-run",
    ]);
    assert!(
        argv.iter().any(|a| a == "h264_videotoolbox"),
        "expected h264_videotoolbox: {argv:?}"
    );
    assert!(
        !argv.iter().any(|a| a == "libx264"),
        "videotoolbox must not keep libx264: {argv:?}"
    );
}

#[test]
fn hw_nvenc_resize_dry_run_swaps_encoder() {
    let (_v, argv) = dry_run_argv(&[
        "--hw",
        "nvenc",
        "resize",
        "in.mp4",
        "--preset",
        "square",
        "-o",
        "out.mp4",
        "--dry-run",
    ]);
    assert!(
        argv.iter().any(|a| a == "h264_nvenc"),
        "expected h264_nvenc: {argv:?}"
    );
}

#[test]
fn hw_none_or_omitted_keeps_libx264() {
    for args in [
        [
            "resize",
            "in.mp4",
            "--preset",
            "square",
            "-o",
            "out.mp4",
            "--dry-run",
        ]
        .as_slice(),
        [
            "--hw",
            "none",
            "resize",
            "in.mp4",
            "--preset",
            "square",
            "-o",
            "out.mp4",
            "--dry-run",
        ]
        .as_slice(),
    ] {
        let (_v, argv) = dry_run_argv(args);
        assert!(
            argv.iter().any(|a| a == "libx264"),
            "expected libx264: {argv:?}"
        );
    }
}

#[test]
fn hw_unknown_is_unknown_hw() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "--hw",
            "potato",
            "resize",
            "in.mp4",
            "--preset",
            "square",
            "-o",
            "out.mp4",
            "--dry-run",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "unknown_hw");
}

#[test]
fn hw_does_not_apply_to_copy_trim() {
    let (_v, argv) = dry_run_argv(&[
        "--hw",
        "videotoolbox",
        "trim",
        "in.mp4",
        "--from",
        "0",
        "--to",
        "1",
        "-o",
        "out.mp4",
        "--dry-run",
    ]);
    assert!(
        argv.windows(2).any(|w| w == ["-c", "copy"]),
        "copy trim must stay -c copy: {argv:?}"
    );
    assert!(
        !argv.iter().any(|a| a == "h264_videotoolbox"),
        "hw must not apply to copy trim: {argv:?}"
    );
}

#[test]
fn hw_videotoolbox_compress_keeps_quality_flag() {
    let (_v, argv) = dry_run_argv(&[
        "--hw",
        "videotoolbox",
        "compress",
        "in.mp4",
        "-o",
        "out.mp4",
        "--dry-run",
    ]);
    assert!(
        argv.iter().any(|a| a == "h264_videotoolbox"),
        "compress should use videotoolbox: {argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w[0] == "-q:v"),
        "videotoolbox must keep a quality flag: {argv:?}"
    );
    assert!(
        !argv.iter().any(|a| a == "-crf"),
        "videotoolbox rejects -crf so it must be substituted: {argv:?}"
    );
}

#[test]
fn hw_on_plan_applies_unless_cli_overrides() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(
        dir.path().join("plan.json"),
        r#"{"hw":"videotoolbox","steps":[{"op":"resize","input":"in.mp4","preset":"square","output":"out.mp4"}]}"#,
    )
    .unwrap();

    let from_plan = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["run", "plan.json", "--dry-run"])
        .assert()
        .success();
    let v: Value =
        serde_json::from_str(&String::from_utf8_lossy(&from_plan.get_output().stdout).trim())
            .unwrap();
    let argv: Vec<&str> = v["steps"][0]["ffmpeg"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.contains(&"h264_videotoolbox"),
        "plan hw should apply: {argv:?}"
    );

    let from_cli = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["--hw", "none", "run", "plan.json", "--dry-run"])
        .assert()
        .success();
    let v: Value =
        serde_json::from_str(&String::from_utf8_lossy(&from_cli.get_output().stdout).trim())
            .unwrap();
    let argv: Vec<&str> = v["steps"][0]["ffmpeg"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.contains(&"libx264"),
        "CLI --hw should win over plan: {argv:?}"
    );
}
