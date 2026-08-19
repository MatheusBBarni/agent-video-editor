use crate::error::Error;
use crate::exec::{Ctx, Outcome};
use crate::op::Op;

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

pub fn execute(op: &Op, _ctx: &Ctx) -> Result<Outcome, Error> {
    let Op::Detect { input, .. } = op else {
        return Err(Error::new("detect", "internal", "not a detect op"));
    };
    if !std::path::Path::new(input).exists() {
        return Err(Error::new(
            "detect",
            "missing_input",
            format!("input not found: {input}"),
        ));
    }
    Err(Error::new("detect", "internal", "detect is not implemented"))
}
