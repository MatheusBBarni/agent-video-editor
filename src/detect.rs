use crate::error::Error;

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
