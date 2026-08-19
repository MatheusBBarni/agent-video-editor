mod common;

use assert_cmd::Command;
use common::{ffmpeg, require_ffmpeg};
use serde_json::Value;
use std::fs;

fn write_keyed_secs(path: &std::path::Path, secs: u32) {
    let video = format!("testsrc=duration={secs}:size=320x240:rate=30");
    let audio = format!("sine=frequency=1000:duration={secs}");
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        &video,
        "-f",
        "lavfi",
        "-i",
        &audio,
        "-c:v",
        "libx264",
        "-g",
        "1",
        "-keyint_min",
        "1",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        path.to_str().unwrap(),
    ]);
}

fn ave_json(dir: &tempfile::TempDir, args: &[&str]) -> (bool, Value) {
    let output = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    (output.status.success(), v)
}

#[test]
fn keep_dry_run_prints_trim_and_concat_passes() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_keyed_secs(&dir.path().join("in.mp4"), 10);

    let (ok, v) = ave_json(
        &dir,
        &[
            "keep",
            "in.mp4",
            "--ranges",
            "0-2,5-8",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "keep");
    let passes = v["passes"]
        .as_array()
        .expect("keep dry-run must emit passes");
    assert!(passes.len() >= 3, "expected two trims + concat: {passes:?}");
    let flat: Vec<&str> = passes
        .iter()
        .flat_map(|p| p.as_array().unwrap())
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        flat.windows(2).any(|w| w == ["-ss", "0"]),
        "first keep range starts at 0: {flat:?}"
    );
    assert!(
        flat.windows(2).any(|w| w == ["-ss", "5"]),
        "second keep range starts at 5: {flat:?}"
    );
    assert!(
        flat.windows(2).any(|w| w == ["-f", "concat"]),
        "multi-range keep must concat: {flat:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());
    let leftover: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        leftover,
        ["in.mp4"],
        "dry-run must not write temps: {leftover:?}"
    );
}

#[test]
fn keep_writes_selected_ranges() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.mp4");
    write_keyed_secs(&input, 10);
    let before = fs::read(&input).unwrap();

    let (ok, v) = ave_json(
        &dir,
        &["keep", "in.mp4", "--ranges", "0-2,5-8", "-o", "out.mp4"],
    );
    assert!(ok, "{v}");
    assert_eq!(v["op"], "keep");
    let duration = v["duration_s"].as_f64().unwrap();
    assert!(
        (duration - 5.0).abs() <= 0.3,
        "expected ~5s kept, got {duration}"
    );
    assert_eq!(fs::read(&input).unwrap(), before);
    let leftover: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        leftover.iter().all(|name| {
            name == "in.mp4"
                || name == "out.mp4"
                || (!name.ends_with(".ts") && !name.starts_with("ave-"))
        }),
        "keep must not leave temps in cwd: {leftover:?}"
    );
}

#[test]
fn keep_zero_to_end_is_not_an_error() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_keyed_secs(&dir.path().join("in.mp4"), 3);
    let (ok, v) = ave_json(
        &dir,
        &[
            "keep",
            "in.mp4",
            "--ranges",
            "0-end",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["op"], "keep");
}

#[test]
fn keep_inverted_or_overlapping_ranges_are_bad_range() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_keyed_secs(&dir.path().join("in.mp4"), 10);

    let (ok, inverted) = ave_json(
        &dir,
        &[
            "keep",
            "in.mp4",
            "--ranges",
            "3-1",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(inverted["error"], "bad_range");

    let (ok, overlap) = ave_json(
        &dir,
        &[
            "keep",
            "in.mp4",
            "--ranges",
            "0-2,1-4",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(overlap["error"], "bad_range");
}

#[test]
fn keep_refuses_in_place_and_missing_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"original").unwrap();

    let (ok, missing) = ave_json(&dir, &["keep", "in.mp4", "--ranges", "0-1"]);
    assert!(!ok);
    assert_eq!(missing["error"], "missing_output");
    assert_eq!(missing["op"], "keep");

    let (ok, inplace) = ave_json(
        &dir,
        &[
            "keep",
            "in.mp4",
            "--ranges",
            "0-1",
            "-o",
            "in.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(inplace["error"], "in_place");
    assert_eq!(fs::read(dir.path().join("in.mp4")).unwrap(), b"original");
}

#[test]
fn run_keep_dry_run_matches_verb() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_keyed_secs(&dir.path().join("in.mp4"), 10);
    fs::write(
        dir.path().join("plan.json"),
        r#"{"steps":[{"op":"keep","input":"in.mp4","ranges":["0-2","5-8"],"output":"out.mp4"}]}"#,
    )
    .unwrap();

    let (ok, v) = ave_json(&dir, &["run", "plan.json", "--dry-run"]);
    assert!(ok, "{v}");
    assert_eq!(v["steps"][0]["op"], "keep");
    assert_eq!(v["steps"][0]["ok"], true);
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn skill_sends_n_cuts_to_keep_ranges() {
    let root = env!("CARGO_MANIFEST_DIR");
    let skill = fs::read_to_string(format!("{root}/skills/ave/SKILL.md")).unwrap();
    assert!(
        skill.contains("keep --ranges") || skill.contains("`keep`"),
        "skill must send N cuts to keep --ranges"
    );
}
