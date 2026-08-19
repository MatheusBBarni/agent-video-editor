use crate::error::{DetectEnvelope, Error};
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
    let argv = recipes::with_bin(detect_argv(input, *kind), &ctx.ffmpeg);
    if ctx.dry_run {
        return Ok(detect_ok(input, *kind, Vec::new(), argv));
    }
    Err(Error::new("detect", "internal", "detect is not implemented"))
}

fn detect_ok(
    input: &str,
    kind: Kind,
    segments: Vec<crate::error::DetectSegment>,
    argv: Vec<String>,
) -> Outcome {
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
