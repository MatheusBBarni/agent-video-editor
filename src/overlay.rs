use crate::error::Error;
use crate::recipes;

pub fn parse_opacity(raw: Option<f64>, op: &'static str) -> Result<Option<f64>, Error> {
    let Some(value) = raw else {
        return Ok(None);
    };
    if value.is_finite() && value > 0.0 && value <= 1.0 {
        return Ok(Some(value));
    }
    Err(Error::new(
        op,
        "bad_range",
        "opacity must be greater than 0 and at most 1",
    ))
}

#[derive(Debug, Clone)]
pub enum OverlayAt {
    Named(String),
    Xy { x: i32, y: i32 },
}

impl OverlayAt {
    pub fn parse(
        position: Option<String>,
        x: Option<i32>,
        y: Option<i32>,
        op: &'static str,
    ) -> Result<Self, Error> {
        let position = position.filter(|s| !s.is_empty());
        match (position, x, y) {
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(Error::new(
                op,
                "conflicting_fields",
                "overlay accepts only one of position or x and y",
            )),
            (None, Some(_), None) | (None, None, Some(_)) => Err(Error::new(
                op,
                "missing_field",
                "overlay pixel placement requires both x and y",
            )),
            (None, Some(x), Some(y)) => Ok(Self::Xy { x, y }),
            (named, None, None) => Ok(Self::Named(named.unwrap_or_else(|| "top-right".into()))),
        }
    }

    pub fn filter(
        &self,
        opacity: Option<f64>,
        span: Option<&(String, String)>,
    ) -> Result<String, Error> {
        let base = match self {
            Self::Xy { x, y } => recipes::overlay_xy(*x, *y),
            Self::Named(name) => recipes::overlay_expr(name)
                .ok_or_else(|| {
                    Error::new(
                        "overlay",
                        "unknown_position",
                        format!("unknown position: {name}"),
                    )
                })?
                .to_string(),
        };
        Ok(recipes::overlay_filter(&base, opacity, span))
    }
}
