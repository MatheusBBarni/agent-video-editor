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

pub fn ffmpeg_gate(available: bool, require: bool) -> bool {
    if available {
        return true;
    }
    if require {
        panic!("ffmpeg required");
    }
    eprintln!("skipping: ffmpeg not on PATH");
    false
}

pub fn require_ffmpeg() -> bool {
    ffmpeg_gate(
        ffmpeg_available(),
        std::env::var_os("AVE_REQUIRE_FFMPEG").is_some(),
    )
}

pub fn ffmpeg(args: &[&str]) {
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

pub fn write_fixture(path: &Path, duration_s: f64, size: (u32, u32), with_audio: bool) {
    let video = format!(
        "testsrc=duration={duration_s}:size={}x{}:rate=30",
        size.0, size.1
    );
    if with_audio {
        let audio = format!("sine=frequency=1000:duration={duration_s}");
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
    } else {
        ffmpeg(&[
            "-y",
            "-f",
            "lavfi",
            "-i",
            &video,
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ]);
    }
}

pub fn write_clip(path: &Path) {
    write_fixture(path, 1.0, (320, 240), true);
}

pub fn write_video_only_fixture(path: &Path) {
    write_fixture(path, 1.0, (320, 240), false);
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
