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
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
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
    assert!(
        passes.len() >= 3,
        "expected two trims + concat: {passes:?}"
    );
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
