use crate::error::Error;

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

pub fn overlay_place(
    position: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
    op: &'static str,
) -> Result<(Option<String>, Option<i32>, Option<i32>), Error> {
    let position = position.filter(|s| !s.is_empty());
    match (&position, x, y) {
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
        _ => Ok((position, x, y)),
    }
}
