use crate::error::{DetectEnvelope, DetectSegment, Error};
use crate::exec::{Ctx, Outcome};
use crate::op::Op;
use crate::probe;
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
    if *kind == Kind::Silence && probe::probed_has_audio(&ctx.ffprobe, input) == Some(false) {
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
        parse_segments(
            *kind,
            &run_detect(&argv)?,
            probed_duration(&ctx.ffprobe, input),
        )
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
    let mut argv = vec!["ffmpeg".into(), "-i".into(), input.into()];
    match kind {
        Kind::Silence => argv.extend(["-af".into(), "silencedetect=noise=-30dB:d=0.5".into()]),
        Kind::Black => argv.extend([
            "-vf".into(),
            "blackdetect=d=0.5:pix_th=0.10".into(),
            "-an".into(),
        ]),
        Kind::Scenes => argv.extend(["-vf".into(), "scdet".into(), "-an".into()]),
    }
    argv.extend(["-f".into(), "null".into(), "-".into()]);
    argv
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

fn probed_duration(ffprobe_bin: &str, input: &str) -> Option<f64> {
    let duration = probe::media_info_from_probe(&probe::probe_json(ffprobe_bin, input)?).duration_s;
    (duration > 0.0).then_some(duration)
}

fn parse_segments(kind: Kind, log: &str, duration_s: Option<f64>) -> Vec<DetectSegment> {
    match kind {
        Kind::Silence => parse_paired(log, "silence_start:", "silence_end:", "silence"),
        Kind::Black => parse_black(log),
        Kind::Scenes => parse_scenes(log, duration_s),
    }
}

fn parse_black(log: &str) -> Vec<DetectSegment> {
    let mut segments = Vec::new();
    for line in log.lines() {
        let Some(start) = labeled_f64(line, "black_start:") else {
            continue;
        };
        let Some(end) = labeled_f64(line, "black_end:") else {
            continue;
        };
        if start < end {
            segments.push(DetectSegment {
                start_s: start,
                end_s: end,
                kind: "black",
            });
        }
    }
    segments
}

fn parse_paired(
    log: &str,
    start_label: &str,
    end_label: &str,
    kind: &'static str,
) -> Vec<DetectSegment> {
    let mut start = None;
    let mut segments = Vec::new();
    for line in log.lines() {
        if let Some(value) = labeled_f64(line, start_label) {
            start = Some(value);
        } else if let Some(end) = labeled_f64(line, end_label) {
            if let Some(start) = start.take() {
                if start < end {
                    segments.push(DetectSegment {
                        start_s: start,
                        end_s: end,
                        kind,
                    });
                }
            }
        }
    }
    segments
}

fn parse_scenes(log: &str, duration_s: Option<f64>) -> Vec<DetectSegment> {
    let mut times: Vec<f64> = log
        .lines()
        .filter_map(|line| labeled_f64(line, "lavfi.scd.time:"))
        .filter(|t| t.is_finite() && *t > 0.0)
        .collect();
    if times.is_empty() {
        return Vec::new();
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup();
    let mut bounds = vec![0.0];
    bounds.extend(times);
    if let Some(duration) = duration_s {
        if duration > *bounds.last().unwrap_or(&0.0) {
            bounds.push(duration);
        }
    }
    bounds
        .windows(2)
        .filter(|pair| pair[0] < pair[1])
        .map(|pair| DetectSegment {
            start_s: pair[0],
            end_s: pair[1],
            kind: "scenes",
        })
        .collect()
}

fn labeled_f64(line: &str, label: &str) -> Option<f64> {
    let rest = line.split_once(label)?.1.trim();
    let token = rest
        .split(|c: char| c.is_whitespace() || c == ',')
        .find(|s| !s.is_empty())?;
    token.parse().ok()
}
