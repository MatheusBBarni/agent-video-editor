use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub fn run_ffmpeg(
    argv: &[String],
    verbose: bool,
    duration_s: Option<f64>,
) -> Result<String, String> {
    if duration_s.is_none() {
        return run_captured(argv, verbose);
    }
    run_with_progress(argv, verbose, duration_s.unwrap_or(0.0))
}

fn run_captured(argv: &[String], verbose: bool) -> Result<String, String> {
    let output = Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| e.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if verbose && !stderr.is_empty() {
        eprint!("{stderr}");
    }
    if output.status.success() {
        Ok(stderr)
    } else {
        Err(stderr)
    }
}

fn run_with_progress(argv: &[String], verbose: bool, duration_s: f64) -> Result<String, String> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .args(["-nostats", "-progress", "pipe:2"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stderr = String::new();
    let mut time_s = 0.0;
    if let Some(pipe) = child.stderr.take() {
        for line in BufReader::new(pipe).lines() {
            let line = line.map_err(|e| e.to_string())?;
            stderr.push_str(&line);
            stderr.push('\n');
            if verbose && !is_progress_kv(&line) {
                eprintln!("{line}");
            }
            if let Some(t) = parse_progress_time(&line) {
                time_s = t;
                emit_progress(time_s, duration_s);
            } else if line == "progress=end" {
                emit_progress(
                    if duration_s > 0.0 { duration_s } else { time_s },
                    duration_s,
                );
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(stderr)
    } else {
        Err(stderr)
    }
}

fn is_progress_kv(line: &str) -> bool {
    matches!(
        line.split_once('=').map(|(k, _)| k),
        Some(
            "frame"
                | "fps"
                | "stream_0_0_q"
                | "bitrate"
                | "total_size"
                | "out_time_us"
                | "out_time_ms"
                | "out_time"
                | "dup_frames"
                | "drop_frames"
                | "speed"
                | "progress"
        )
    )
}

fn parse_progress_time(line: &str) -> Option<f64> {
    let (key, val) = line.split_once('=')?;
    if val == "N/A" {
        return None;
    }
    match key {
        "out_time_us" => val.parse::<f64>().ok().map(|us| us / 1_000_000.0),
        "out_time" => parse_hms(val),
        _ => None,
    }
}

fn parse_hms(raw: &str) -> Option<f64> {
    let parts: Vec<&str> = raw.trim().split(':').collect();
    let [h, m, s] = parts.as_slice() else {
        return None;
    };
    Some(h.parse::<f64>().ok()? * 3600.0 + m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?)
}

fn emit_progress(time_s: f64, duration_s: f64) {
    let progress = if duration_s > 0.0 {
        (time_s / duration_s).clamp(0.0, 1.0)
    } else {
        0.0
    };
    eprintln!(
        "{}",
        serde_json::json!({ "progress": progress, "time_s": time_s })
    );
}
