use crate::error::{DetectEnvelope, DetectSegment, Error};
use crate::exec::{Ctx, Outcome, probed_duration, run_ffmpeg};
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

pub fn execute(input: &str, kind: Kind, ctx: &Ctx) -> Result<Outcome, Error> {
    if !std::path::Path::new(input).exists() {
        return Err(Error::new(
            "detect",
            "missing_input",
            format!("input not found: {input}"),
        ));
    }
    if kind == Kind::Silence && probe::probed_has_audio(&ctx.ffprobe, input) == Some(false) {
        return Err(Error::new(
            "detect",
            "no_audio",
            format!("input has no audio stream: {input}"),
        ));
    }
    let argv = recipes::with_bin(detect_argv(input, kind), &ctx.ffmpeg);
    let segments = if ctx.dry_run {
        Vec::new()
    } else {
        let duration = match kind {
            Kind::Scenes => Some(probed_duration(&ctx.ffprobe, input, "detect")?),
            Kind::Silence | Kind::Black => None,
        };
        let log = run_ffmpeg(&argv).map_err(|e| Error::ffmpeg("detect", e))?;
        parse_segments(kind, &log, duration)
    };
    Ok(Outcome::Detect(DetectEnvelope {
        ok: true,
        op: "detect",
        kind: kind.as_str(),
        input: input.to_string(),
        segments,
        ffmpeg: argv,
    }))
}

fn detect_argv(input: &str, kind: Kind) -> Vec<String> {
    match kind {
        Kind::Silence => recipes::detect_silence_argv(input),
        Kind::Black => recipes::detect_black_argv(input),
        Kind::Scenes => recipes::detect_scenes_argv(input),
    }
}

fn parse_segments(kind: Kind, log: &str, duration_s: Option<f64>) -> Vec<DetectSegment> {
    match kind {
        Kind::Silence => parse_paired(log, "silence_start:", "silence_end:", "silence"),
        Kind::Black => parse_paired(log, "black_start:", "black_end:", "black"),
        Kind::Scenes => parse_scenes(log, duration_s),
    }
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
        }
        if let Some(end) = labeled_f64(line, end_label) {
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
