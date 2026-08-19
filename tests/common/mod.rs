use assert_cmd::Command as Ave;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ffmpeg(args: &[&str]) {
    let status = Command::new("ffmpeg")
        .args(args)
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

pub fn write_fixture(path: &Path) {
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=duration=1:size=320x240:rate=30",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=1000:duration=1",
        "-c:v",
        "libx264",
        "-c:a",
        "aac",
        "-pix_fmt",
        "yuv420p",
        path.to_str().unwrap(),
    ]);
}

pub fn ave_json(dir: &tempfile::TempDir, args: &[&str]) -> (bool, Value) {
    let output = Ave::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    (output.status.success(), v)
}

pub fn write_video_only_fixture(path: &Path) {
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=duration=1:size=320x240:rate=30",
        "-an",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        path.to_str().unwrap(),
    ]);
}
