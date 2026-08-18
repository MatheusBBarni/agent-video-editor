use crate::error::Error;

#[derive(PartialEq, Eq)]
pub struct VideoShape {
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: String,
}

pub fn probe_json(ffprobe_bin: &str, input: &str) -> Option<serde_json::Value> {
    let output = std::process::Command::new(ffprobe_bin)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            input,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

pub fn media_meta(ffprobe_bin: &str, path: &str) -> (f64, u32, u32, u64) {
    let Some(probe) = probe_json(ffprobe_bin, path) else {
        return (0.0, 0, 0, 0);
    };
    let duration_s = probe["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| probe["format"]["duration"].as_f64())
        .unwrap_or(0.0);
    let size_bytes = probe["format"]["size"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| probe["format"]["size"].as_u64())
        .unwrap_or(0);
    let video = probe["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"));
    let width = video.and_then(|s| s["width"].as_u64()).unwrap_or(0) as u32;
    let height = video.and_then(|s| s["height"].as_u64()).unwrap_or(0) as u32;
    (duration_s, width, height, size_bytes)
}

pub fn probe_video(ffprobe_bin: &str, input: &str) -> Option<VideoShape> {
    let probe = probe_json(ffprobe_bin, input)?;
    let video = probe["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"))?;
    Some(VideoShape {
        codec: video["codec_name"].as_str().unwrap_or("").to_string(),
        width: video["width"].as_u64().unwrap_or(0) as u32,
        height: video["height"].as_u64().unwrap_or(0) as u32,
        fps: video["avg_frame_rate"].as_str().unwrap_or("").to_string(),
    })
}

pub fn tool_version(bin: &str) -> Option<String> {
    let output = std::process::Command::new(bin)
        .arg("-version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(2)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn ffprobe_argv(ffprobe_bin: &str, input: &str) -> Vec<String> {
    vec![
        ffprobe_bin.into(),
        "-v".into(),
        "quiet".into(),
        "-print_format".into(),
        "json".into(),
        "-show_format".into(),
        "-show_streams".into(),
        input.into(),
    ]
}

pub fn probe_or_err(ffprobe_bin: &str, input: &str) -> Result<serde_json::Value, Error> {
    let argv = ffprobe_argv(ffprobe_bin, input);
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| Error::new("info", "ffprobe_failed", e.to_string()))?;
    if !output.status.success() {
        return Err(Error::new(
            "info",
            "ffprobe_failed",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| Error::new("info", "ffprobe_failed", e.to_string()))
}
