use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hw {
    #[default]
    None,
    VideoToolbox,
    Nvenc,
}

impl Hw {
    pub fn parse(raw: Option<&str>) -> Result<Self, Error> {
        match raw.unwrap_or("none") {
            "none" => Ok(Self::None),
            "videotoolbox" => Ok(Self::VideoToolbox),
            "nvenc" => Ok(Self::Nvenc),
            other => Err(Error::new(
                "ave",
                "unknown_hw",
                format!("unknown hw: {other}"),
            )),
        }
    }

    pub fn codec(self) -> &'static str {
        match self {
            Self::None => "libx264",
            Self::VideoToolbox => "h264_videotoolbox",
            Self::Nvenc => "h264_nvenc",
        }
    }
}
