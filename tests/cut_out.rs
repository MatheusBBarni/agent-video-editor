use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ffmpeg(args: &[&str]) {
    let status = std::process::Command::new("ffmpeg")
        .args(args)
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn write_secs(path: &std::path::Path, secs: u32) {
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
        "-c:a",
        "aac",
        "-pix_fmt",
        "yuv420p",
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
fn cut_out_dry_run_prints_trim_and_concat_passes() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_secs(&dir.path().join("in.mp4"), 5);

    let (ok, v) = ave_json(
        &dir,
        &[
            "cut-out",
            "in.mp4",
            "--from",
            "2",
            "--to",
            "4",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "cut-out");

    let passes = v["passes"]
        .as_array()
        .expect("cut-out dry-run must emit passes");
    assert!(
        passes.len() >= 3,
        "expected keep-head trim, keep-tail trim, and concat: {passes:?}"
    );

    let flat: Vec<&str> = passes
        .iter()
        .flat_map(|p| p.as_array().unwrap())
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        flat.windows(2).any(|w| w == ["-ss", "0"]) && flat.iter().any(|t| *t == "-to"),
        "passes must include a head trim: {flat:?}"
    );
    assert!(
        flat.windows(2).any(|w| w == ["-ss", "4"]),
        "passes must include a tail trim from --to: {flat:?}"
    );
    assert!(
        flat.windows(2).any(|w| w == ["-f", "concat"]),
        "passes must include concat: {flat:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());

    let mut leftover: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    leftover.sort();
    assert_eq!(
        leftover,
        ["in.mp4"],
        "dry-run must not write temps or output: {leftover:?}"
    );
}

#[test]
fn cut_out_unprobeable_dry_run_fails_ffprobe() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "cut-out",
            "in.mp4",
            "--from",
            "1",
            "--to",
            "2",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["ok"], false);
    assert_eq!(v["op"], "cut-out");
    assert_eq!(v["error"], "ffprobe_failed");
}

#[test]
fn cut_out_writes_kept_ranges_and_leaves_input() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.mp4");
    write_keyed_secs(&input, 5);
    let before = fs::read(&input).unwrap();

    let (ok, v) = ave_json(
        &dir,
        &[
            "cut-out", "in.mp4", "--from", "1", "--to", "3", "-o", "out.mp4",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["op"], "cut-out");
    let duration = v["duration_s"].as_f64().unwrap();
    assert!(
        (duration - 3.0).abs() <= 0.3,
        "expected ~3s leftover, got {duration}"
    );
    assert_eq!(fs::read(&input).unwrap(), before);
    assert!(dir.path().join("out.mp4").exists());

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
        "cut-out must not leave temps in cwd: {leftover:?}"
    );
}

#[test]
fn cut_out_from_after_to_is_bad_range() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"x").unwrap();
    let (ok, v) = ave_json(
        &dir,
        &[
            "cut-out",
            "in.mp4",
            "--from",
            "3",
            "--to",
            "1",
            "-o",
            "out.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["error"], "bad_range");
    assert_eq!(v["op"], "cut-out");
}

#[test]
fn cut_out_refuses_in_place() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"original").unwrap();
    let (ok, v) = ave_json(
        &dir,
        &[
            "cut-out",
            "in.mp4",
            "--from",
            "1",
            "--to",
            "2",
            "-o",
            "in.mp4",
            "--dry-run",
        ],
    );
    assert!(!ok);
    assert_eq!(v["error"], "in_place");
    assert_eq!(fs::read(dir.path().join("in.mp4")).unwrap(), b"original");
}

#[test]
fn cut_out_missing_output_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"x").unwrap();
    let (ok, v) = ave_json(&dir, &["cut-out", "in.mp4", "--from", "1", "--to", "2"]);
    assert!(!ok);
    assert_eq!(v["error"], "missing_output");
    assert_eq!(v["op"], "cut-out");
}

#[test]
fn run_cut_out_dry_run_matches_verb() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_secs(&dir.path().join("in.mp4"), 5);
    fs::write(
        dir.path().join("plan.json"),
        r#"{"steps":[{"op":"cut-out","input":"in.mp4","from":"1","to":"3","output":"out.mp4"}]}"#,
    )
    .unwrap();

    let (ok, v) = ave_json(&dir, &["run", "plan.json", "--dry-run"]);
    assert!(ok, "{v}");
    assert_eq!(v["op"], "run");
    assert_eq!(v["steps"][0]["op"], "cut-out");
    assert_eq!(v["steps"][0]["ok"], true);
    let passes = v["steps"][0]["passes"]
        .as_array()
        .expect("run cut-out dry-run must emit passes");
    assert!(passes.len() >= 3, "{passes:?}");
    assert!(!dir.path().join("out.mp4").exists());
}

#[test]
fn skill_uses_cut_out_for_middle_delete() {
    let root = env!("CARGO_MANIFEST_DIR");
    let skill = fs::read_to_string(format!("{root}/skills/ave/SKILL.md")).unwrap();
    let plans = fs::read_to_string(format!("{root}/skills/ave/references/plans.md")).unwrap();
    assert!(
        skill.contains("One hole → `cut-out`"),
        "skill must send a single hole to cut-out"
    );
    assert!(
        plans.contains("op\": \"cut-out\"") || plans.contains("cut-out in.mp4"),
        "plans must show cut-out, not two trims + concat"
    );
}

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
