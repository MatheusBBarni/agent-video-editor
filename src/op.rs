use crate::error::Error;
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
    Resize {
        input: String,
        output: String,
        preset: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
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
    Info {
        input: String,
    },
    Doctor,
}

impl Op {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Trim { .. } => "trim",
            Self::Concat { .. } => "concat",
            Self::CutOut { .. } => "cut-out",
            Self::Resize { .. } => "resize",
            Self::Speed { .. } => "speed",
            Self::ExtractAudio { .. } => "extract-audio",
            Self::ReplaceAudio { .. } => "replace-audio",
            Self::Overlay { .. } => "overlay",
            Self::Compress { .. } => "compress",
            Self::Convert { .. } => "convert",
            Self::Info { .. } => "info",
            Self::Doctor => "doctor",
        }
    }

    pub fn output(&self) -> Option<&str> {
        match self {
            Self::Trim { output, .. }
            | Self::Concat { output, .. }
            | Self::CutOut { output, .. }
            | Self::Resize { output, .. }
            | Self::Speed { output, .. }
            | Self::ExtractAudio { output, .. }
            | Self::ReplaceAudio { output, .. }
            | Self::Overlay { output, .. }
            | Self::Compress { output, .. }
            | Self::Convert { output, .. } => Some(output),
            Self::Info { .. } | Self::Doctor => None,
        }
    }

    pub fn inputs(&self) -> Vec<&str> {
        match self {
            Self::Trim { input, .. }
            | Self::CutOut { input, .. }
            | Self::Resize { input, .. }
            | Self::Speed { input, .. }
            | Self::ExtractAudio { input, .. }
            | Self::Compress { input, .. }
            | Self::Convert { input, .. }
            | Self::Info { input } => vec![input],
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
            "resize" => {
                let _ = req("input")?;
                let _ = req("output")?;
                if step["preset"].as_str().is_none()
                    && (step["width"].as_u64().is_none() || step["height"].as_u64().is_none())
                {
                    return Err(Error::new(
                        "run",
                        "missing_field",
                        "resize requires preset or width and height",
                    ));
                }
                Ok(Self::Resize {
                    input: req("input")?,
                    output: req("output")?,
                    preset: step["preset"].as_str().map(str::to_string),
                    width: step["width"].as_u64().map(|n| n as u32),
                    height: step["height"].as_u64().map(|n| n as u32),
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
            "overlay" => Ok(Self::Overlay {
                input: req("input")?,
                image: req("image")?,
                output: req("output")?,
                position: step["position"].as_str().map(str::to_string),
            }),
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
            "info" | "doctor" => Err(Error::new(
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
        if let Some(preset) = preset {
            return recipes::preset_size(preset).ok_or_else(|| {
                Error::new(
                    self.name(),
                    "unknown_preset",
                    format!("unknown preset: {preset}"),
                )
            });
        }
        match (width, height) {
            (Some(w), Some(h)) => Ok((*w, *h)),
            _ => Err(Error::new(
                self.name(),
                "missing_preset",
                "resize requires --preset or --width and --height",
            )),
        }
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

fn parse_nonneg_finite(raw: &str) -> Option<f64> {
    let n: f64 = raw.parse().ok()?;
    (n.is_finite() && n >= 0.0).then_some(n)
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

pub fn require_output(op: &'static str, output: Option<String>) -> Result<String, Error> {
    output.ok_or_else(|| {
        Error::new(
            op,
            "missing_output",
            "mutating commands require -o / --output",
        )
    })
}
