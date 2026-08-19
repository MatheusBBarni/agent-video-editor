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
