use crate::error::Error;

#[derive(PartialEq, Eq)]
pub struct VideoShape {
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: String,
}

pub struct MediaInfo {
    pub duration_s: f64,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub video_codec: String,
    pub audio_codec: String,
    pub fps: String,
    pub has_video: bool,
    pub has_audio: bool,
    pub rotate_deg: u32,
    pub display_width: u32,
    pub display_height: u32,
}

pub fn media_info_from_probe(probe: &serde_json::Value) -> MediaInfo {
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
    let streams = probe["streams"].as_array();
    let video = streams.and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"));
    let audio = streams.and_then(|streams| streams.iter().find(|s| s["codec_type"] == "audio"));
    let width = video.and_then(|s| s["width"].as_u64()).unwrap_or(0) as u32;
    let height = video.and_then(|s| s["height"].as_u64()).unwrap_or(0) as u32;
    let rotate_deg = video.map(rotate_deg).unwrap_or(0);
    let (display_width, display_height) = display_size(width, height, rotate_deg);
    MediaInfo {
        duration_s,
        width,
        height,
        size_bytes,
        video_codec: video
            .and_then(|s| s["codec_name"].as_str())
            .unwrap_or("")
            .to_string(),
        audio_codec: audio
            .and_then(|s| s["codec_name"].as_str())
            .unwrap_or("")
            .to_string(),
        fps: video
            .and_then(|s| s["avg_frame_rate"].as_str())
            .unwrap_or("")
            .to_string(),
        has_video: video.is_some(),
        has_audio: audio.is_some(),
        rotate_deg,
        display_width,
        display_height,
    }
}

fn rotate_deg(video: &serde_json::Value) -> u32 {
    if let Some(deg) = json_number(&video["tags"]["rotate"]) {
        return snap_rotation(deg);
    }
    let Some(side_data) = video["side_data_list"].as_array() else {
        return 0;
    };
    side_data
        .iter()
        .find(|sd| {
            sd["side_data_type"]
                .as_str()
                .is_some_and(|ty| ty.eq_ignore_ascii_case("Display Matrix"))
        })
        .and_then(|sd| json_number(&sd["rotation"]))
        .map(snap_rotation)
        .unwrap_or(0)
}

fn display_size(width: u32, height: u32, rotate_deg: u32) -> (u32, u32) {
    match rotate_deg {
        90 | 270 => (height, width),
        _ => (width, height),
    }
}

fn snap_rotation(deg: f64) -> u32 {
    if !deg.is_finite() {
        return 0;
    }
    let norm = ((deg % 360.0) + 360.0) % 360.0;
    ((norm / 90.0).round() as u32 % 4) * 90
}

fn json_number(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
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
