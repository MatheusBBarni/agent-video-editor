use crate::error::Error;
use crate::recipes;

#[derive(Debug, Clone)]
pub enum Op {
    Trim {
        input: String,
        from: String,
        to: Option<String>,
        duration: Option<String>,
        output: String,
        accurate: bool,
    },
    Concat {
        inputs: Vec<String>,
        output: String,
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
                let _ = req("input")?;
                let _ = req("from")?;
                let to = step["to"].as_str().filter(|s| !s.is_empty()).map(str::to_string);
                let duration = json_string_or_number(&step["duration"]);
                if to.is_none() && duration.is_none() {
                    return Err(Error::new(
                        "run",
                        "missing_field",
                        "trim requires to or duration",
                    ));
                }
                if to.is_some() && duration.is_some() {
                    return Err(Error::new(
                        "run",
                        "conflicting_fields",
                        "trim accepts only one of to or duration",
                    ));
                }
                let _ = req("output")?;
                Ok(Self::Trim {
                    input: req("input")?,
                    from: req("from")?,
                    to,
                    duration,
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
                let mute = step["mute"].as_bool().unwrap_or(false);
                let audio = step["audio"].as_str().map(str::to_string);
                let mix = step["mix"].as_str().map(str::to_string);
                if !mute && audio.is_none() && mix.is_none() {
                    return Err(Error::new(
                        "run",
                        "missing_audio",
                        "replace-audio requires mute, audio, or mix",
                    ));
                }
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
            "info" => Ok(Self::Info {
                input: req("input")?,
            }),
            "doctor" => Ok(Self::Doctor),
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

fn json_string_or_number(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value.as_str().filter(|s| !s.is_empty()) {
        return Some(s.to_string());
    }
    value.as_number().map(ToString::to_string)
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
