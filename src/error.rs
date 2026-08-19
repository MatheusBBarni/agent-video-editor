use serde::Serialize;

#[derive(Debug)]
pub struct Error {
    pub op: &'static str,
    pub code: &'static str,
    pub message: String,
}

impl Error {
    pub fn new(op: &'static str, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            op,
            code,
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
pub struct Envelope {
    pub ok: bool,
    pub op: &'static str,
    pub output: String,
    pub duration_s: f64,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub ffmpeg: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passes: Option<Vec<Vec<String>>>,
}

#[derive(Serialize)]
pub struct FailEnvelope {
    pub ok: bool,
    pub op: &'static str,
    pub error: &'static str,
    pub message: String,
}

#[derive(Serialize)]
pub struct RunEnvelope {
    pub ok: bool,
    pub op: &'static str,
    pub steps: Vec<Envelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_step: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct InfoEnvelope {
    pub ok: bool,
    pub op: &'static str,
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
    pub ffmpeg: Vec<String>,
}

#[derive(Serialize)]
pub struct DoctorEnvelope {
    pub ok: bool,
    pub op: &'static str,
    pub ffmpeg_found: bool,
    pub ffprobe_found: bool,
    pub ffmpeg_version: String,
    pub ffprobe_version: String,
}

pub fn print_json(value: &impl Serialize) {
    println!("{}", serde_json::to_string(value).expect("json"));
}

pub fn fail(err: Error) -> ! {
    print_json(&FailEnvelope {
        ok: false,
        op: err.op,
        error: err.code,
        message: err.message,
    });
    std::process::exit(1);
}
