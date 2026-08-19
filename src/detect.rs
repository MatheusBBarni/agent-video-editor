use crate::error::{DetectEnvelope, DetectSegment, Error};
use crate::exec::{Ctx, Outcome};
use crate::op::Op;
use crate::recipes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Silence,
    Black,
    Scenes,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Silence => "silence",
            Self::Black => "black",
            Self::Scenes => "scenes",
        }
    }
}

pub fn parse_kind(raw: &str) -> Result<Kind, Error> {
    match raw {
        "silence" => Ok(Kind::Silence),
        "black" => Ok(Kind::Black),
        "scenes" => Ok(Kind::Scenes),
        other => Err(Error::new(
            "detect",
            "unknown_kind",
            format!("unknown kind: {other}"),
        )),
    }
}

pub fn execute(op: &Op, ctx: &Ctx) -> Result<Outcome, Error> {
    let Op::Detect { input, kind } = op else {
        return Err(Error::new("detect", "internal", "not a detect op"));
    };
    if !std::path::Path::new(input).exists() {
        return Err(Error::new(
            "detect",
            "missing_input",
            format!("input not found: {input}"),
        ));
    }
    if *kind == Kind::Silence
        && crate::probe::probed_has_audio(&ctx.ffprobe, input) == Some(false)
    {
        return Err(Error::new(
            "detect",
            "no_audio",
            format!("input has no audio stream: {input}"),
        ));
    }
    let argv = recipes::with_bin(detect_argv(input, *kind), &ctx.ffmpeg);
    let segments = if ctx.dry_run {
        Vec::new()
    } else {
        parse_segments(*kind, &run_detect(&argv)?)
    };
    Ok(detect_ok(input, *kind, segments, argv))
}

fn detect_ok(input: &str, kind: Kind, segments: Vec<DetectSegment>, argv: Vec<String>) -> Outcome {
    Outcome::Detect(DetectEnvelope {
        ok: true,
        op: "detect",
        kind: kind.as_str(),
        input: input.to_string(),
        segments,
        ffmpeg: argv,
    })
}

fn detect_argv(input: &str, kind: Kind) -> Vec<String> {
    match kind {
        Kind::Silence => vec![
            "ffmpeg".into(),
            "-i".into(),
            input.into(),
            "-af".into(),
            "silencedetect=noise=-30dB:d=0.5".into(),
            "-f".into(),
            "null".into(),
            "-".into(),
        ],
        Kind::Black | Kind::Scenes => vec!["ffmpeg".into(), "-i".into(), input.into()],
    }
}

fn run_detect(argv: &[String]) -> Result<String, Error> {
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| Error::ffmpeg("detect", e.to_string()))?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        Ok(stderr)
    } else {
        Err(Error::ffmpeg("detect", stderr))
    }
}

fn parse_segments(kind: Kind, log: &str) -> Vec<DetectSegment> {
    match kind {
        Kind::Silence => parse_silence(log),
        Kind::Black | Kind::Scenes => Vec::new(),
    }
}

fn parse_silence(log: &str) -> Vec<DetectSegment> {
    let mut start = None;
    let mut segments = Vec::new();
    for line in log.lines() {
        if let Some(value) = labeled_f64(line, "silence_start:") {
            start = Some(value);
        } else if let Some(end) = labeled_f64(line, "silence_end:") {
            if let Some(start) = start.take() {
                if start < end {
                    segments.push(DetectSegment {
                        start_s: start,
                        end_s: end,
                        kind: "silence",
                    });
                }
            }
        }
    }
    segments
}

fn labeled_f64(line: &str, label: &str) -> Option<f64> {
    let rest = line.split_once(label)?.1.trim();
    let token = rest.split_whitespace().next()?;
    token.parse().ok()
}
