use crate::error::Error;
pub use crate::overlay::{overlay_place, parse_opacity};
use crate::recipes;

#[derive(Debug, Clone)]
pub enum TrimEnd {
    To(String),
    Duration(String),
}

impl TrimEnd {
    pub fn exclusive(
        to: Option<String>,
        duration: Option<String>,
        op: &'static str,
    ) -> Result<Self, Error> {
        let to = to.filter(|s| !s.is_empty());
        let duration = duration.filter(|s| !s.is_empty());
        match (to, duration) {
            (Some(to), None) => Ok(Self::To(to)),
            (None, Some(duration)) => Ok(Self::Duration(duration)),
            (Some(_), Some(_)) => Err(Error::new(
                op,
                "conflicting_fields",
                "trim accepts only one of to or duration",
            )),
            (None, None) => Err(Error::new(
                op,
                "missing_field",
                "trim requires to or duration",
            )),
        }
    }

    pub fn validate_against(&self, from: &str, op: &'static str) -> Result<(), Error> {
        let from_s = parse_timestamp(from)
            .ok_or_else(|| Error::new(op, "bad_timestamp", format!("invalid timestamp: {from}")))?;
        match self {
            Self::To(to) => {
                let to_s = parse_timestamp(to).ok_or_else(|| {
                    Error::new(op, "bad_timestamp", format!("invalid timestamp: {to}"))
                })?;
                if from_s >= to_s {
                    return Err(Error::new(op, "bad_range", "from must be less than to"));
                }
            }
            Self::Duration(duration) => {
                let duration_s = parse_timestamp(duration).ok_or_else(|| {
                    Error::new(
                        op,
                        "bad_timestamp",
                        format!("invalid timestamp: {duration}"),
                    )
                })?;
                if duration_s <= 0.0 {
                    return Err(Error::new(
                        op,
                        "bad_range",
                        "duration must be greater than 0",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn ffmpeg_flag(&self) -> (&'static str, &str) {
        match self {
            Self::To(value) => ("-to", value),
            Self::Duration(value) => ("-t", value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeepRange {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub enum Op {
    Trim {
        input: String,
        from: String,
        end: TrimEnd,
        output: String,
        accurate: bool,
    },
    Concat {
        inputs: Vec<String>,
        output: String,
    },
    CutOut {
        input: String,
        from: String,
        to: String,
        output: String,
        accurate: bool,
    },
    Keep {
        input: String,
        ranges: Vec<KeepRange>,
        output: String,
        accurate: bool,
    },
    Resize {
        input: String,
        output: String,
        preset: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        fit: recipes::Fit,
    },
    Speed {
        input: String,
        output: String,
        factor: f64,
    },
    ExtractAudio {
        input: String,
        output: String,
        format: Option<String>,
    },
    ReplaceAudio {
        input: String,
        output: String,
        mute: bool,
        audio: Option<String>,
        mix: Option<String>,
    },
    Overlay {
        input: String,
        image: String,
        output: String,
        position: Option<String>,
        x: Option<i32>,
        y: Option<i32>,
        opacity: Option<f64>,
        span: Option<(String, String)>,
    },
    Compress {
        input: String,
        output: String,
        crf: u8,
        preset: String,
    },
    Convert {
        input: String,
        output: String,
    },
    Frame {
        input: String,
        at: String,
        output: String,
    },
    Frames {
        input: String,
        at: Vec<String>,
        every: Option<f64>,
        sheet: Option<String>,
        output: String,
    },
    Captions {
        input: String,
        srt: String,
        output: String,
    },
    Rotate {
        input: String,
        deg: recipes::RotateDeg,
        output: String,
    },
    Volume {
        input: String,
        db: f64,
        output: String,
    },
    Fade {
        input: String,
        fade_in: Option<f64>,
        fade_out: Option<f64>,
        output: String,
    },
    Text {
        input: String,
        text: String,
        position: recipes::TextPos,
        span: Option<(String, String)>,
        output: String,
    },
    Crop {
        input: String,
        insets: CropInsets,
        output: String,
    },
    Info {
        input: String,
    },
    Detect {
        input: String,
        kind: crate::detect::Kind,
    },
    Doctor,
}

impl Op {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Trim { .. } => "trim",
            Self::Concat { .. } => "concat",
            Self::CutOut { .. } => "cut-out",
            Self::Keep { .. } => "keep",
            Self::Resize { .. } => "resize",
            Self::Speed { .. } => "speed",
            Self::ExtractAudio { .. } => "extract-audio",
            Self::ReplaceAudio { .. } => "replace-audio",
            Self::Overlay { .. } => "overlay",
            Self::Compress { .. } => "compress",
            Self::Convert { .. } => "convert",
            Self::Frame { .. } => "frame",
            Self::Frames { .. } => "frames",
            Self::Captions { .. } => "captions",
            Self::Text { .. } => "text",
            Self::Fade { .. } => "fade",
            Self::Volume { .. } => "volume",
            Self::Rotate { .. } => "rotate",
            Self::Crop { .. } => "crop",
            Self::Info { .. } => "info",
            Self::Detect { .. } => "detect",
            Self::Doctor => "doctor",
        }
    }

    pub fn output(&self) -> Option<&str> {
        match self {
            Self::Trim { output, .. }
            | Self::Concat { output, .. }
            | Self::CutOut { output, .. }
            | Self::Keep { output, .. }
            | Self::Resize { output, .. }
            | Self::Speed { output, .. }
            | Self::ExtractAudio { output, .. }
            | Self::ReplaceAudio { output, .. }
            | Self::Overlay { output, .. }
            | Self::Compress { output, .. }
            | Self::Convert { output, .. }
            | Self::Frame { output, .. }
            | Self::Frames { output, .. }
            | Self::Captions { output, .. }
            | Self::Text { output, .. }
            | Self::Fade { output, .. }
            | Self::Volume { output, .. }
            | Self::Rotate { output, .. }
            | Self::Crop { output, .. } => Some(output),
            Self::Info { .. } | Self::Detect { .. } | Self::Doctor => None,
        }
    }

    pub fn inputs(&self) -> Vec<&str> {
        match self {
            Self::Trim { input, .. }
            | Self::CutOut { input, .. }
            | Self::Keep { input, .. }
            | Self::Resize { input, .. }
            | Self::Speed { input, .. }
            | Self::ExtractAudio { input, .. }
            | Self::Compress { input, .. }
            | Self::Convert { input, .. }
            | Self::Frame { input, .. }
            | Self::Frames { input, .. }
            | Self::Text { input, .. }
            | Self::Fade { input, .. }
            | Self::Volume { input, .. }
            | Self::Rotate { input, .. }
            | Self::Crop { input, .. }
            | Self::Info { input }
            | Self::Detect { input, .. } => vec![input],
            Self::Captions { input, srt, .. } => vec![input, srt],
            Self::Concat { inputs, .. } => inputs.iter().map(String::as_str).collect(),
            Self::ReplaceAudio {
                input, audio, mix, ..
            } => {
                let mut v = vec![input.as_str()];
                if let Some(a) = audio {
                    v.push(a);
                }
                if let Some(m) = mix {
                    v.push(m);
                }
                v
            }
            Self::Overlay { input, image, .. } => vec![input, image],
            Self::Doctor => vec![],
        }
    }

    pub fn from_json(step: &serde_json::Value) -> Result<Self, Error> {
        let op = step["op"].as_str().unwrap_or("");
        let req = |key: &str| -> Result<String, Error> {
            step[key]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    Error::new(
                        "run",
                        "missing_field",
                        format!("step missing required field: {key}"),
                    )
                })
        };
        match op {
            "trim" => {
                let from = req("from")?;
                let end = TrimEnd::exclusive(
                    json_string_or_number(&step["to"]),
                    json_string_or_number(&step["duration"]),
                    "run",
                )?;
                end.validate_against(&from, "run")?;
                Ok(Self::Trim {
                    input: req("input")?,
                    from,
                    end,
                    output: req("output")?,
                    accurate: step["accurate"].as_bool().unwrap_or(false),
                })
            }
            "concat" => {
                let inputs = step["inputs"]
                    .as_array()
                    .ok_or_else(|| Error::new("run", "missing_field", "concat requires inputs"))?;
                if inputs.len() < 2 {
                    return Err(Error::new(
                        "run",
                        "too_few_inputs",
                        "concat requires at least two inputs",
                    ));
                }
                Ok(Self::Concat {
                    inputs: inputs
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                    output: req("output")?,
                })
            }
            "cut-out" => {
                let from = req("from")?;
                let to = req("to")?;
                TrimEnd::To(to.clone()).validate_against(&from, "run")?;
                Ok(Self::CutOut {
                    input: req("input")?,
                    from,
                    to,
                    output: req("output")?,
                    accurate: step["accurate"].as_bool().unwrap_or(false),
                })
            }
            "keep" => {
                let raw = step["ranges"]
                    .as_array()
                    .ok_or_else(|| Error::new("run", "missing_field", "keep requires ranges"))?;
                if raw.is_empty() {
                    return Err(Error::new(
                        "run",
                        "bad_range",
                        "keep requires at least one range",
                    ));
                }
                let mut ranges = Vec::new();
                for value in raw {
                    let spec = value.as_str().ok_or_else(|| {
                        Error::new("run", "bad_range", "keep ranges must be strings")
                    })?;
                    ranges.push(parse_keep_range(spec, "run")?);
                }
                Ok(Self::Keep {
                    input: req("input")?,
                    ranges,
                    output: req("output")?,
                    accurate: step["accurate"].as_bool().unwrap_or(false),
                })
            }
            "resize" => {
                let _ = req("input")?;
                let _ = req("output")?;
                let preset = step["preset"].as_str().map(str::to_string);
                let width = step["width"].as_u64().map(|n| n as u32);
                let height = step["height"].as_u64().map(|n| n as u32);
                resize_size_pair(preset.as_deref(), width, height, "run")?;
                Ok(Self::Resize {
                    input: req("input")?,
                    output: req("output")?,
                    preset,
                    width,
                    height,
                    fit: parse_resize_fit(
                        step["fit"].as_str(),
                        step["stretch"].as_bool().unwrap_or(false),
                        "run",
                    )?,
                })
            }
            "speed" => {
                let factor = step["factor"]
                    .as_f64()
                    .ok_or_else(|| Error::new("run", "missing_field", "speed requires factor"))?;
                Ok(Self::Speed {
                    input: req("input")?,
                    output: req("output")?,
                    factor,
                })
            }
            "extract-audio" => Ok(Self::ExtractAudio {
                input: req("input")?,
                output: req("output")?,
                format: step["format"].as_str().map(str::to_string),
            }),
            "replace-audio" => {
                let (mute, audio, mix) = replace_audio_choice(
                    step["mute"].as_bool().unwrap_or(false),
                    step["audio"].as_str().map(str::to_string),
                    step["mix"].as_str().map(str::to_string),
                    "run",
                )?;
                Ok(Self::ReplaceAudio {
                    input: req("input")?,
                    output: req("output")?,
                    mute,
                    audio,
                    mix,
                })
            }
            "overlay" => {
                let place = overlay_place(
                    step["position"].as_str().map(str::to_string),
                    json_i32(&step["x"]),
                    json_i32(&step["y"]),
                    "run",
                )?;
                Ok(Self::Overlay {
                    input: req("input")?,
                    image: req("image")?,
                    output: req("output")?,
                    position: place.position,
                    x: place.x,
                    y: place.y,
                    opacity: parse_opacity(step["opacity"].as_f64(), "run")?,
                    span: text_span(
                        json_string_or_number(&step["from"]),
                        json_string_or_number(&step["to"]),
                        "run",
                    )?,
                })
            }
            "compress" => Ok(Self::Compress {
                input: req("input")?,
                output: req("output")?,
                crf: step["crf"].as_u64().unwrap_or(23) as u8,
                preset: step["preset"].as_str().unwrap_or("medium").to_string(),
            }),
            "convert" => Ok(Self::Convert {
                input: req("input")?,
                output: req("output")?,
            }),
            "rotate" => {
                let deg = step["deg"]
                    .as_u64()
                    .ok_or_else(|| Error::new("run", "missing_field", "rotate requires deg"))?;
                Ok(Self::Rotate {
                    input: req("input")?,
                    deg: parse_rotate_deg(deg as u32, "run")?,
                    output: req("output")?,
                })
            }
            "volume" => {
                let db = json_string_or_number(&step["db"])
                    .ok_or_else(|| Error::new("run", "missing_field", "volume requires db"))?;
                Ok(Self::Volume {
                    input: req("input")?,
                    db: parse_db("run", &db)?,
                    output: req("output")?,
                })
            }
            "fade" => {
                let (fade_in, fade_out) = fade_pair(
                    json_string_or_number(&step["in"]),
                    json_string_or_number(&step["out"]),
                    "run",
                )?;
                Ok(Self::Fade {
                    input: req("input")?,
                    fade_in,
                    fade_out,
                    output: req("output")?,
                })
            }
            "text" => Ok(Self::Text {
                input: req("input")?,
                text: req("text")?,
                position: parse_text_pos(step["position"].as_str(), "run")?,
                span: text_span(
                    json_string_or_number(&step["from"]),
                    json_string_or_number(&step["to"]),
                    "run",
                )?,
                output: req("output")?,
            }),
            "captions" => Ok(Self::Captions {
                input: req("input")?,
                srt: require_subtitle_file("run", req("srt")?)?,
                output: req("output")?,
            }),
            "crop" => Ok(Self::Crop {
                input: req("input")?,
                insets: crop_insets(
                    json_u32(&step["top"]),
                    json_u32(&step["bottom"]),
                    json_u32(&step["left"]),
                    json_u32(&step["right"]),
                    "run",
                )?,
                output: req("output")?,
            }),
            "frames" => Err(Error::new(
                "run",
                "unsupported_in_run",
                "frames is not valid inside ave run",
            )),
            "frame" => {
                let at = req("at")?;
                if parse_timestamp(&at).is_none() {
                    return Err(Error::new(
                        "run",
                        "bad_timestamp",
                        format!("invalid timestamp: {at}"),
                    ));
                }
                Ok(Self::Frame {
                    input: req("input")?,
                    at,
                    output: req("output")?,
                })
            }
            "info" | "doctor" | "detect" => Err(Error::new(
                "run",
                "unsupported_in_run",
                format!("{op} is not valid inside ave run"),
            )),
            "" => Err(Error::new("run", "unknown_op", "step missing op")),
            other => Err(Error::new(
                "run",
                "unknown_op",
                format!("unknown op: {other}"),
            )),
        }
    }

    pub fn resize_size(&self) -> Result<(u32, u32), Error> {
        let Self::Resize {
            preset,
            width,
            height,
            ..
        } = self
        else {
            return Err(Error::new(self.name(), "missing_preset", "not a resize op"));
        };
        resize_size_pair(preset.as_deref(), *width, *height, self.name())
    }
}

fn resize_size_pair(
    preset: Option<&str>,
    width: Option<u32>,
    height: Option<u32>,
    op: &'static str,
) -> Result<(u32, u32), Error> {
    match (preset, width, height) {
        (Some(preset), None, None) => recipes::preset_size(preset)
            .ok_or_else(|| Error::new(op, "unknown_preset", format!("unknown preset: {preset}"))),
        (None, Some(w), Some(h)) => Ok((w, h)),
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(Error::new(
            op,
            "conflicting_fields",
            "resize accepts only one of preset or width and height",
        )),
        _ => Err(Error::new(
            op,
            "missing_preset",
            "resize requires --preset or --width and --height",
        )),
    }
}

pub fn parse_timestamp(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if !raw.contains(':') {
        return parse_nonneg_finite(raw);
    }
    let parts: Vec<&str> = raw.split(':').collect();
    match parts.as_slice() {
        [minutes, seconds] => {
            let minutes: u64 = minutes.parse().ok()?;
            let seconds = parse_nonneg_finite(seconds)?;
            (seconds < 60.0).then_some(minutes as f64 * 60.0 + seconds)
        }
        [hours, minutes, seconds] => {
            let hours: u64 = hours.parse().ok()?;
            let minutes: u64 = minutes.parse().ok()?;
            let seconds = parse_nonneg_finite(seconds)?;
            (minutes < 60 && seconds < 60.0)
                .then_some(hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds)
        }
        _ => None,
    }
}

pub fn parse_keep_ranges(raw: &str, op: &'static str) -> Result<Vec<KeepRange>, Error> {
    let items: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        return Err(Error::new(
            op,
            "bad_range",
            "keep requires at least one range",
        ));
    }
    items
        .into_iter()
        .map(|item| parse_keep_range(item, op))
        .collect()
}

pub fn parse_keep_range(raw: &str, op: &'static str) -> Result<KeepRange, Error> {
    let (from, to) = raw
        .split_once('-')
        .ok_or_else(|| Error::new(op, "bad_range", format!("invalid range: {raw}")))?;
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        return Err(Error::new(op, "bad_range", format!("invalid range: {raw}")));
    }
    if parse_timestamp(from).is_none() {
        return Err(Error::new(
            op,
            "bad_timestamp",
            format!("invalid timestamp: {from}"),
        ));
    }
    if to != "end" && parse_timestamp(to).is_none() {
        return Err(Error::new(
            op,
            "bad_timestamp",
            format!("invalid timestamp: {to}"),
        ));
    }
    Ok(KeepRange {
        from: from.to_string(),
        to: to.to_string(),
    })
}

fn parse_nonneg_finite(raw: &str) -> Option<f64> {
    let n: f64 = raw.parse().ok()?;
    (n.is_finite() && n >= 0.0).then_some(n)
}

fn json_u32(value: &serde_json::Value) -> Option<u32> {
    value.as_u64().map(|n| n as u32)
}

fn json_i32(value: &serde_json::Value) -> Option<i32> {
    value
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .or_else(|| value.as_u64().and_then(|n| i32::try_from(n).ok()))
}

fn json_string_or_number(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value.as_str().filter(|s| !s.is_empty()) {
        return Some(s.to_string());
    }
    value.as_number().map(ToString::to_string)
}

pub fn replace_audio_choice(
    mute: bool,
    audio: Option<String>,
    mix: Option<String>,
    op: &'static str,
) -> Result<(bool, Option<String>, Option<String>), Error> {
    let count = usize::from(mute) + usize::from(audio.is_some()) + usize::from(mix.is_some());
    match count {
        0 => Err(Error::new(
            op,
            "missing_audio",
            "replace-audio requires mute, audio, or mix",
        )),
        1 => Ok((mute, audio, mix)),
        _ => Err(Error::new(
            op,
            "conflicting_flags",
            "replace-audio accepts only one of mute, audio, or mix",
        )),
    }
}

pub fn parse_at_list(
    at: Option<String>,
    every: Option<&str>,
    op: &'static str,
) -> Result<Vec<String>, Error> {
    match (
        at.filter(|s| !s.is_empty()),
        every.filter(|s| !s.is_empty()),
    ) {
        (Some(_), Some(_)) => Err(Error::new(
            op,
            "conflicting_fields",
            "frames accepts only one of --at or --every",
        )),
        (None, None) => Err(Error::new(
            op,
            "missing_field",
            "frames requires --at or --every",
        )),
        (None, Some(_)) => Ok(Vec::new()),
        (Some(raw), None) => {
            let stamps: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if stamps.is_empty() {
                return Err(Error::new(
                    op,
                    "missing_field",
                    "frames requires --at or --every",
                ));
            }
            for stamp in &stamps {
                if parse_timestamp(stamp).is_none() {
                    return Err(Error::new(
                        op,
                        "bad_timestamp",
                        format!("invalid timestamp: {stamp}"),
                    ));
                }
            }
            Ok(stamps)
        }
    }
}

pub fn parse_every(every: Option<String>, op: &'static str) -> Result<Option<f64>, Error> {
    let Some(raw) = every.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let secs = parse_timestamp(&raw)
        .ok_or_else(|| Error::new(op, "bad_timestamp", format!("invalid timestamp: {raw}")))?;
    if secs <= 0.0 {
        return Err(Error::new(op, "bad_range", "every must be greater than 0"));
    }
    Ok(Some(secs))
}

pub fn parse_fit(raw: Option<&str>, op: &'static str) -> Result<recipes::Fit, Error> {
    match raw.unwrap_or("pad") {
        "pad" => Ok(recipes::Fit::Pad),
        "crop" => Ok(recipes::Fit::Crop),
        "stretch" => Ok(recipes::Fit::Stretch),
        other => Err(Error::new(
            op,
            "unknown_fit",
            format!("unknown fit: {other}"),
        )),
    }
}

pub fn parse_resize_fit(
    fit: Option<&str>,
    stretch: bool,
    op: &'static str,
) -> Result<recipes::Fit, Error> {
    if stretch {
        return match fit {
            None | Some("stretch") => Ok(recipes::Fit::Stretch),
            Some(_) => Err(Error::new(
                op,
                "conflicting_fields",
                "resize accepts only one of stretch or fit",
            )),
        };
    }
    parse_fit(fit, op)
}

pub fn parse_text_pos(raw: Option<&str>, op: &'static str) -> Result<recipes::TextPos, Error> {
    match raw.unwrap_or("lower-third") {
        "lower-third" => Ok(recipes::TextPos::LowerThird),
        "center" => Ok(recipes::TextPos::Center),
        "top" => Ok(recipes::TextPos::Top),
        other => Err(Error::new(
            op,
            "unknown_position",
            format!("unknown position: {other}"),
        )),
    }
}

pub fn text_span(
    from: Option<String>,
    to: Option<String>,
    op: &'static str,
) -> Result<Option<(String, String)>, Error> {
    match (from.filter(|s| !s.is_empty()), to.filter(|s| !s.is_empty())) {
        (None, None) => Ok(None),
        (Some(from), Some(to)) => {
            let from_s = parse_timestamp(&from).ok_or_else(|| {
                Error::new(op, "bad_timestamp", format!("invalid timestamp: {from}"))
            })?;
            let to_s = parse_timestamp(&to).ok_or_else(|| {
                Error::new(op, "bad_timestamp", format!("invalid timestamp: {to}"))
            })?;
            if from_s >= to_s {
                return Err(Error::new(op, "bad_range", "from must be less than to"));
            }
            Ok(Some((from, to)))
        }
        _ => Err(Error::new(
            op,
            "missing_field",
            "text requires both from and to, or neither",
        )),
    }
}

pub fn parse_rotate_deg(deg: u32, op: &'static str) -> Result<recipes::RotateDeg, Error> {
    match deg {
        90 => Ok(recipes::RotateDeg::D90),
        180 => Ok(recipes::RotateDeg::D180),
        270 => Ok(recipes::RotateDeg::D270),
        _ => Err(Error::new(
            op,
            "bad_range",
            format!("rotate accepts 90, 180, or 270: {deg}"),
        )),
    }
}

pub fn parse_db(op: &'static str, raw: &str) -> Result<f64, Error> {
    let db: f64 = raw
        .parse()
        .map_err(|_| Error::new(op, "bad_range", format!("invalid db value: {raw}")))?;
    if !db.is_finite() {
        return Err(Error::new(
            op,
            "bad_range",
            format!("invalid db value: {raw}"),
        ));
    }
    Ok(db)
}

fn parse_fade_secs(raw: &str, op: &'static str) -> Result<f64, Error> {
    let secs = parse_timestamp(raw)
        .ok_or_else(|| Error::new(op, "bad_timestamp", format!("invalid timestamp: {raw}")))?;
    if secs <= 0.0 {
        return Err(Error::new(
            op,
            "bad_range",
            "fade duration must be greater than 0",
        ));
    }
    Ok(secs)
}

pub fn fade_pair(
    fade_in: Option<String>,
    fade_out: Option<String>,
    op: &'static str,
) -> Result<(Option<f64>, Option<f64>), Error> {
    let fade_in = fade_in.filter(|s| !s.is_empty());
    let fade_out = fade_out.filter(|s| !s.is_empty());
    if fade_in.is_none() && fade_out.is_none() {
        return Err(Error::new(
            op,
            "missing_field",
            "fade requires --in or --out",
        ));
    }
    Ok((
        fade_in
            .as_deref()
            .map(|v| parse_fade_secs(v, op))
            .transpose()?,
        fade_out
            .as_deref()
            .map(|v| parse_fade_secs(v, op))
            .transpose()?,
    ))
}

pub fn require_subtitle_file(op: &'static str, path: String) -> Result<String, Error> {
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "srt" | "vtt" => Ok(path),
        _ => Err(Error::new(
            op,
            "unknown_format",
            format!("captions require .srt or .vtt: {path}"),
        )),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CropInsets {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl CropInsets {
    pub fn validate_against(&self, width: u32, height: u32, op: &'static str) -> Result<(), Error> {
        if self.left.saturating_add(self.right) >= width
            || self.top.saturating_add(self.bottom) >= height
        {
            return Err(Error::new(op, "bad_range", "crop would empty the frame"));
        }
        Ok(())
    }

    pub fn filter(self) -> String {
        recipes::crop_filter(self.top, self.bottom, self.left, self.right)
    }
}

pub fn crop_insets(
    top: Option<u32>,
    bottom: Option<u32>,
    left: Option<u32>,
    right: Option<u32>,
    op: &'static str,
) -> Result<CropInsets, Error> {
    if top.is_none() && bottom.is_none() && left.is_none() && right.is_none() {
        return Err(Error::new(
            op,
            "missing_field",
            "crop requires --top, --bottom, --left, or --right",
        ));
    }
    Ok(CropInsets {
        top: top.unwrap_or(0),
        bottom: bottom.unwrap_or(0),
        left: left.unwrap_or(0),
        right: right.unwrap_or(0),
    })
}

pub fn require_output(op: &'static str, output: Option<String>) -> Result<String, Error> {
    output.ok_or_else(|| {
        Error::new(
            op,
            "missing_output",
            "mutating commands require -o / --output",
        )
    })
}
