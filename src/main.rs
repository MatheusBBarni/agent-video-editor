mod recipes;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Parser)]
#[command(name = "ave")]
struct Cli {
    #[arg(long, global = true)]
    dry_run: bool,
    #[arg(long, global = true)]
    no_overwrite: bool,
    #[arg(long, global = true)]
    copy_only: bool,
    #[arg(long, global = true)]
    ffmpeg: Option<String>,
    #[arg(long, global = true)]
    ffprobe: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Trim {
        input: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
        #[arg(long)]
        accurate: bool,
    },
    Doctor,
    Run {
        plan: String,
    },
    Info {
        input: String,
    },
    Concat {
        inputs: Vec<String>,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
    Resize {
        input: String,
        #[arg(long)]
        preset: Option<String>,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
    Convert {
        input: String,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
    Compress {
        input: String,
        #[arg(long, default_value_t = 23)]
        crf: u8,
        #[arg(long, default_value = "medium")]
        preset: String,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
    Overlay {
        input: String,
        #[arg(long)]
        image: String,
        #[arg(long)]
        position: Option<String>,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
    #[command(name = "replace-audio")]
    ReplaceAudio {
        input: String,
        #[arg(long)]
        mute: bool,
        #[arg(long)]
        audio: Option<String>,
        #[arg(long)]
        mix: Option<String>,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
    #[command(name = "extract-audio")]
    ExtractAudio {
        input: String,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
        #[arg(long)]
        format: Option<String>,
    },
    Speed {
        input: String,
        #[arg(long)]
        factor: f64,
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
}

#[derive(Serialize)]
struct Envelope {
    ok: bool,
    op: &'static str,
    output: String,
    duration_s: f64,
    width: u32,
    height: u32,
    size_bytes: u64,
    ffmpeg: Vec<String>,
}

#[derive(Deserialize)]
struct Plan {
    steps: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct RunEnvelope {
    ok: bool,
    op: &'static str,
    steps: Vec<Envelope>,
}

#[derive(Serialize)]
struct RunFailEnvelope {
    ok: bool,
    op: &'static str,
    failed_step: usize,
    error: &'static str,
    message: String,
    steps: Vec<Envelope>,
    written: Vec<String>,
}

#[derive(Serialize)]
struct ConvertEnvelope {
    ok: bool,
    op: &'static str,
    output: String,
    duration_s: f64,
    width: u32,
    height: u32,
    size_bytes: u64,
    ffmpeg: Vec<String>,
    passes: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct InfoEnvelope {
    ok: bool,
    op: &'static str,
    duration_s: f64,
    width: u32,
    height: u32,
    size_bytes: u64,
    ffmpeg: Vec<String>,
}

#[derive(Serialize)]
struct DoctorEnvelope {
    ok: bool,
    op: &'static str,
    ffmpeg_found: bool,
    ffprobe_found: bool,
    ffmpeg_version: String,
    ffprobe_version: String,
}

#[derive(Serialize)]
struct FailEnvelope {
    ok: bool,
    op: &'static str,
    error: &'static str,
    message: String,
}

fn write_concat_list(inputs: &[String]) -> String {
    let path = std::env::temp_dir().join(format!(
        "ave-concat-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut body = String::new();
    for input in inputs {
        let abs = cwd.join(input);
        let escaped = abs.to_string_lossy().replace('\'', r"'\''");
        body.push_str(&format!("file '{escaped}'\n"));
    }
    std::fs::write(&path, body).unwrap_or_else(|e| fail("concat", "concat_list", e.to_string()));
    path.to_string_lossy().into_owned()
}

fn same_file(a: &str, b: &str) -> bool {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let pa = cwd.join(a);
    let pb = cwd.join(b);
    match (pa.canonicalize(), pb.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => pa == pb,
    }
}

fn media_meta(ffprobe_bin: &str, path: &str) -> (f64, u32, u32, u64) {
    let Some(probe) = probe_json(ffprobe_bin, path) else {
        return (0.0, 0, 0, 0);
    };
    let duration_s = probe["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| probe["format"]["duration"].as_f64())
        .unwrap_or(0.0);
    let size_bytes = probe["format"]["size"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| probe["format"]["size"].as_u64())
        .unwrap_or(0);
    let video = probe["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"));
    let width = video.and_then(|s| s["width"].as_u64()).unwrap_or(0) as u32;
    let height = video.and_then(|s| s["height"].as_u64()).unwrap_or(0) as u32;
    (duration_s, width, height, size_bytes)
}

fn ok_envelope(
    op: &'static str,
    output: String,
    ffmpeg: Vec<String>,
    ffprobe_bin: &str,
    dry_run: bool,
) -> Envelope {
    let (duration_s, width, height, size_bytes) = if dry_run {
        (0.0, 0, 0, 0)
    } else {
        media_meta(ffprobe_bin, &output)
    };
    Envelope {
        ok: true,
        op,
        output,
        duration_s,
        width,
        height,
        size_bytes,
        ffmpeg,
    }
}

fn prepare_argv(op: &'static str, argv: Vec<String>, bin: &str, dry_run: bool) -> Vec<String> {
    let argv = recipes::with_bin(argv, bin);
    if !dry_run {
        run_ffmpeg(op, &argv);
    }
    argv
}

fn run_ffmpeg(op: &'static str, argv: &[String]) {
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .unwrap_or_else(|e| fail(op, "ffmpeg_failed", e.to_string()));
    if !output.status.success() {
        fail(op, "ffmpeg_failed", String::from_utf8_lossy(&output.stderr));
    }
}

fn fail_run(
    failed_step: usize,
    error: &'static str,
    message: impl Into<String>,
    steps: Vec<Envelope>,
    written: Vec<String>,
) -> ! {
    let envelope = RunFailEnvelope {
        ok: false,
        op: "run",
        failed_step,
        error,
        message: message.into(),
        steps,
        written,
    };
    println!("{}", serde_json::to_string(&envelope).expect("json"));
    std::process::exit(1);
}

fn fail(op: &'static str, error: &'static str, message: impl Into<String>) -> ! {
    let envelope = FailEnvelope {
        ok: false,
        op,
        error,
        message: message.into(),
    };
    println!("{}", serde_json::to_string(&envelope).expect("json"));
    std::process::exit(1);
}

#[derive(PartialEq, Eq)]
struct VideoShape {
    codec: String,
    width: u32,
    height: u32,
    fps: String,
}

fn probe_json(ffprobe_bin: &str, input: &str) -> Option<serde_json::Value> {
    let output = std::process::Command::new(ffprobe_bin)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            input,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn probe_video(ffprobe_bin: &str, input: &str) -> Option<VideoShape> {
    let probe = probe_json(ffprobe_bin, input)?;
    let video = probe["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"))?;
    Some(VideoShape {
        codec: video["codec_name"].as_str().unwrap_or("").to_string(),
        width: video["width"].as_u64().unwrap_or(0) as u32,
        height: video["height"].as_u64().unwrap_or(0) as u32,
        fps: video["avg_frame_rate"].as_str().unwrap_or("").to_string(),
    })
}

fn require_step_str<'a>(
    step: &'a serde_json::Value,
    key: &str,
) -> Result<&'a str, (&'static str, String)> {
    step[key].as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
        (
            "missing_field",
            format!("step missing required field: {key}"),
        )
    })
}

fn validate_plan_step(step: &serde_json::Value) -> Result<(), (&'static str, String)> {
    let op = step["op"].as_str().unwrap_or("");
    match op {
        "trim" => {
            require_step_str(step, "input")?;
            require_step_str(step, "from")?;
            if step["to"].as_str().is_none() && step["duration"].as_str().is_none() {
                return Err(("missing_field", "trim requires to or duration".into()));
            }
            require_step_str(step, "output")?;
            Ok(())
        }
        "concat" => {
            let inputs = step["inputs"]
                .as_array()
                .ok_or(("missing_field", "concat requires inputs".into()))?;
            if inputs.len() < 2 {
                return Err((
                    "too_few_inputs",
                    "concat requires at least two inputs".into(),
                ));
            }
            require_step_str(step, "output")?;
            Ok(())
        }
        "resize" => {
            require_step_str(step, "input")?;
            require_step_str(step, "output")?;
            if step["preset"].as_str().is_none()
                && (step["width"].as_u64().is_none() || step["height"].as_u64().is_none())
            {
                return Err((
                    "missing_field",
                    "resize requires preset or width and height".into(),
                ));
            }
            Ok(())
        }
        "speed" => {
            require_step_str(step, "input")?;
            require_step_str(step, "output")?;
            if step["factor"].as_f64().is_none() {
                return Err(("missing_field", "speed requires factor".into()));
            }
            Ok(())
        }
        "extract-audio" | "compress" | "convert" => {
            require_step_str(step, "input")?;
            require_step_str(step, "output")?;
            Ok(())
        }
        "replace-audio" => {
            require_step_str(step, "input")?;
            require_step_str(step, "output")?;
            if !step["mute"].as_bool().unwrap_or(false)
                && step["audio"].as_str().is_none()
                && step["mix"].as_str().is_none()
            {
                return Err((
                    "missing_audio",
                    "replace-audio requires mute, audio, or mix".into(),
                ));
            }
            Ok(())
        }
        "overlay" => {
            require_step_str(step, "input")?;
            require_step_str(step, "image")?;
            require_step_str(step, "output")?;
            Ok(())
        }
        "info" | "doctor" => Ok(()),
        "" => Err(("unknown_op", "step missing op".into())),
        other => Err(("unknown_op", format!("unknown op: {other}"))),
    }
}

type PlanFfmpeg =
    Result<(&'static str, String, Vec<String>, Option<String>), (&'static str, String)>;

fn plan_ffmpeg(
    step: &serde_json::Value,
    ffprobe_bin: &str,
    copy_only: bool,
    dry_run: bool,
) -> PlanFfmpeg {
    let op = step["op"].as_str().unwrap_or("");
    match op {
        "trim" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            let from = require_step_str(step, "from")?.to_string();
            let to = step["to"].as_str().unwrap_or("").to_string();
            let accurate = step["accurate"].as_bool().unwrap_or(false);
            if accurate && copy_only {
                return Err(("copy_only", "accurate trim requires re-encode".into()));
            }
            Ok((
                "trim",
                output.clone(),
                recipes::trim_argv(&from, &to, &input, &output, accurate),
                None,
            ))
        }
        "concat" => {
            let output = require_step_str(step, "output")?.to_string();
            let inputs: Vec<String> = step["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            let shapes: Vec<_> = inputs
                .iter()
                .filter_map(|input| probe_video(ffprobe_bin, input))
                .collect();
            let matched = shapes.len() == inputs.len() && shapes.windows(2).all(|w| w[0] == w[1]);
            let copy = shapes.is_empty() || matched;
            if !copy && copy_only {
                return Err(("copy_only", "mismatched concat requires re-encode".into()));
            }
            let list_path = if dry_run {
                "concat-list.txt".to_string()
            } else {
                write_concat_list(&inputs)
            };
            Ok((
                "concat",
                output.clone(),
                recipes::concat_argv(&list_path, &output, copy),
                (!dry_run).then_some(list_path),
            ))
        }
        "resize" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            if copy_only {
                return Err(("copy_only", "resize requires re-encode".into()));
            }
            let (w, h) = if let Some(preset) = step["preset"].as_str() {
                recipes::preset_size(preset)
                    .ok_or(("unknown_preset", format!("unknown preset: {preset}")))?
            } else {
                (
                    step["width"].as_u64().unwrap_or(0) as u32,
                    step["height"].as_u64().unwrap_or(0) as u32,
                )
            };
            Ok((
                "resize",
                output.clone(),
                recipes::resize_argv(&input, &output, &recipes::scale_pad(w, h)),
                None,
            ))
        }
        "speed" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            let factor = step["factor"].as_f64().unwrap_or(0.0);
            if factor <= 0.0 {
                return Err((
                    "invalid_factor",
                    "speed factor must be greater than 0".into(),
                ));
            }
            if copy_only {
                return Err(("copy_only", "speed requires re-encode".into()));
            }
            Ok((
                "speed",
                output.clone(),
                recipes::speed_argv(&input, &output, factor),
                None,
            ))
        }
        "extract-audio" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            let ext = step["format"].as_str().unwrap_or_else(|| {
                std::path::Path::new(&output)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("mp3")
            });
            let codec = match ext {
                "mp3" => "libmp3lame",
                "wav" => "pcm_s16le",
                "aac" => "aac",
                "flac" => "flac",
                "copy" => "copy",
                other => return Err(("unknown_format", format!("unknown audio format: {other}"))),
            };
            Ok((
                "extract-audio",
                output.clone(),
                recipes::extract_audio_argv(&input, &output, codec),
                None,
            ))
        }
        "replace-audio" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            let ffmpeg = if step["mute"].as_bool().unwrap_or(false) {
                recipes::mute_argv(&input, &output)
            } else if let Some(audio) = step["audio"].as_str() {
                recipes::replace_audio_argv(&input, audio, &output)
            } else if let Some(mix) = step["mix"].as_str() {
                recipes::mix_audio_argv(&input, mix, &output)
            } else {
                return Err((
                    "missing_audio",
                    "replace-audio requires mute, audio, or mix".into(),
                ));
            };
            Ok(("replace-audio", output, ffmpeg, None))
        }
        "overlay" => {
            let input = require_step_str(step, "input")?.to_string();
            let image = require_step_str(step, "image")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            if copy_only {
                return Err(("copy_only", "overlay requires re-encode".into()));
            }
            let expr = recipes::overlay_expr(step["position"].as_str().unwrap_or("top-right"))
                .ok_or(("unknown_position", "unknown overlay position".into()))?;
            Ok((
                "overlay",
                output.clone(),
                recipes::overlay_argv(&input, &image, &output, expr),
                None,
            ))
        }
        "compress" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            if copy_only {
                return Err(("copy_only", "compress requires re-encode".into()));
            }
            let crf = step["crf"].as_u64().unwrap_or(23) as u8;
            let preset = step["preset"].as_str().unwrap_or("medium");
            Ok((
                "compress",
                output.clone(),
                recipes::compress_argv(&input, &output, crf, preset),
                None,
            ))
        }
        "convert" => {
            let input = require_step_str(step, "input")?.to_string();
            let output = require_step_str(step, "output")?.to_string();
            let gif = std::path::Path::new(&output)
                .extension()
                .and_then(|e| e.to_str())
                == Some("gif");
            if gif && copy_only {
                return Err(("copy_only", "gif convert requires re-encode".into()));
            }
            let ffmpeg = if gif {
                recipes::gif_passes(&input, &output).1
            } else {
                recipes::convert_argv(&input, &output)
            };
            Ok(("convert", output, ffmpeg, None))
        }
        other => Err(("unknown_op", format!("unknown op: {other}"))),
    }
}

fn step_inputs(step: &serde_json::Value) -> Vec<String> {
    let mut inputs = Vec::new();
    if let Some(input) = step["input"].as_str() {
        inputs.push(input.to_string());
    }
    if let Some(arr) = step["inputs"].as_array() {
        inputs.extend(arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())));
    }
    if let Some(image) = step["image"].as_str() {
        inputs.push(image.to_string());
    }
    if let Some(audio) = step["audio"].as_str() {
        inputs.push(audio.to_string());
    }
    if let Some(mix) = step["mix"].as_str() {
        inputs.push(mix.to_string());
    }
    inputs
}

fn tool_version(bin: &str) -> Option<String> {
    let output = std::process::Command::new(bin)
        .arg("-version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next().unwrap_or("");
    first
        .split_whitespace()
        .nth(2)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    let cli = Cli::parse();
    let ffmpeg_bin = cli.ffmpeg.as_deref().unwrap_or("ffmpeg");
    let ffprobe_bin = cli.ffprobe.as_deref().unwrap_or("ffprobe");
    match cli.command {
        Command::Run { plan } => {
            let text = if plan == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .unwrap_or_else(|e| fail("run", "bad_plan", e.to_string()));
                buf
            } else {
                std::fs::read_to_string(&plan)
                    .unwrap_or_else(|e| fail("run", "bad_plan", e.to_string()))
            };
            let parsed: Plan = serde_json::from_str(&text)
                .unwrap_or_else(|e| fail("run", "bad_plan", e.to_string()));
            for step in &parsed.steps {
                if let Err((error, message)) = validate_plan_step(step) {
                    fail("run", error, message);
                }
            }
            let mut planned = HashSet::new();
            let mut steps = Vec::new();
            let mut written = Vec::new();
            for (idx, step) in parsed.steps.into_iter().enumerate() {
                let output = step["output"].as_str().unwrap_or("").to_string();
                for input in step_inputs(&step) {
                    let input_ready = std::path::Path::new(&input).exists()
                        || (cli.dry_run && planned.contains(&input));
                    if !input_ready {
                        fail_run(
                            idx,
                            "missing_input",
                            format!("input not found: {input}"),
                            steps,
                            written,
                        );
                    }
                    if !output.is_empty() && same_file(&input, &output) {
                        fail_run(
                            idx,
                            "in_place",
                            "refusing in-place edit: output resolves to the same file as input",
                            steps,
                            written,
                        );
                    }
                }
                let (op, output, ffmpeg, cleanup) =
                    match plan_ffmpeg(&step, ffprobe_bin, cli.copy_only, cli.dry_run) {
                        Ok(v) => v,
                        Err((error, message)) => fail_run(idx, error, message, steps, written),
                    };
                let ffmpeg = recipes::with_bin(ffmpeg, ffmpeg_bin);
                if !cli.dry_run {
                    let result = std::process::Command::new(&ffmpeg[0])
                        .args(&ffmpeg[1..])
                        .output();
                    if let Some(path) = cleanup {
                        let _ = std::fs::remove_file(path);
                    }
                    match result {
                        Ok(out) if out.status.success() => {}
                        Ok(out) => fail_run(
                            idx,
                            "ffmpeg_failed",
                            String::from_utf8_lossy(&out.stderr),
                            steps,
                            written,
                        ),
                        Err(e) => fail_run(idx, "ffmpeg_failed", e.to_string(), steps, written),
                    }
                    written.push(output.clone());
                }
                planned.insert(output.clone());
                steps.push(ok_envelope(op, output, ffmpeg, ffprobe_bin, cli.dry_run));
            }
            let envelope = RunEnvelope {
                ok: true,
                op: "run",
                steps,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Convert { input, output } => {
            let Some(output) = output else {
                fail(
                    "convert",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "convert",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            let gif = std::path::Path::new(&output)
                .extension()
                .and_then(|e| e.to_str())
                == Some("gif");
            if gif && cli.copy_only {
                fail(
                    "convert",
                    "copy_only",
                    "gif convert requires re-encode; --copy-only refuses this operation",
                );
            }
            if same_file(&input, &output) {
                fail(
                    "convert",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if cli.no_overwrite && std::path::Path::new(&output).exists() {
                fail(
                    "convert",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {output}"),
                );
            }
            if gif {
                let (pass1, pass2) = recipes::gif_passes(&input, &output);
                let pass1 = recipes::with_bin(pass1, ffmpeg_bin);
                let pass2 = recipes::with_bin(pass2, ffmpeg_bin);
                if !cli.dry_run {
                    run_ffmpeg("convert", &pass1);
                    let result = std::process::Command::new(&pass2[0])
                        .args(&pass2[1..])
                        .output();
                    let _ = std::fs::remove_file("palette.png");
                    match result {
                        Ok(out) if out.status.success() => {}
                        Ok(out) => fail(
                            "convert",
                            "ffmpeg_failed",
                            String::from_utf8_lossy(&out.stderr),
                        ),
                        Err(e) => fail("convert", "ffmpeg_failed", e.to_string()),
                    }
                }
                let (duration_s, width, height, size_bytes) = if cli.dry_run {
                    (0.0, 0, 0, 0)
                } else {
                    media_meta(ffprobe_bin, &output)
                };
                let envelope = ConvertEnvelope {
                    ok: true,
                    op: "convert",
                    output,
                    duration_s,
                    width,
                    height,
                    size_bytes,
                    ffmpeg: pass2.clone(),
                    passes: vec![pass1, pass2],
                };
                println!("{}", serde_json::to_string(&envelope).expect("json"));
            } else {
                let ffmpeg = prepare_argv(
                    "convert",
                    recipes::convert_argv(&input, &output),
                    ffmpeg_bin,
                    cli.dry_run,
                );
                let envelope = ok_envelope("convert", output, ffmpeg, ffprobe_bin, cli.dry_run);
                println!("{}", serde_json::to_string(&envelope).expect("json"));
            }
        }
        Command::Compress {
            input,
            crf,
            preset,
            output,
        } => {
            let Some(output) = output else {
                fail(
                    "compress",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "compress",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            if cli.copy_only {
                fail(
                    "compress",
                    "copy_only",
                    "compress requires re-encode; --copy-only refuses this operation",
                );
            }
            if same_file(&input, &output) {
                fail(
                    "compress",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if cli.no_overwrite && std::path::Path::new(&output).exists() {
                fail(
                    "compress",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {output}"),
                );
            }
            let ffmpeg = prepare_argv(
                "compress",
                recipes::compress_argv(&input, &output, crf, &preset),
                ffmpeg_bin,
                cli.dry_run,
            );
            let envelope = ok_envelope("compress", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Overlay {
            input,
            image,
            position,
            output,
        } => {
            let Some(output) = output else {
                fail(
                    "overlay",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "overlay",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            if !std::path::Path::new(&image).exists() {
                fail(
                    "overlay",
                    "missing_image",
                    format!("image not found: {image}"),
                );
            }
            if cli.copy_only {
                fail(
                    "overlay",
                    "copy_only",
                    "overlay requires re-encode; --copy-only refuses this operation",
                );
            }
            let expr = recipes::overlay_expr(position.as_deref().unwrap_or("top-right"))
                .unwrap_or_else(|| {
                    fail(
                        "overlay",
                        "unknown_position",
                        format!("unknown position: {}", position.unwrap_or_default()),
                    )
                });
            if same_file(&input, &output) {
                fail(
                    "overlay",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if cli.no_overwrite && std::path::Path::new(&output).exists() {
                fail(
                    "overlay",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {output}"),
                );
            }
            let ffmpeg = prepare_argv(
                "overlay",
                recipes::overlay_argv(&input, &image, &output, expr),
                ffmpeg_bin,
                cli.dry_run,
            );
            let envelope = ok_envelope("overlay", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::ReplaceAudio {
            input,
            mute,
            audio,
            mix,
            output,
        } => {
            let Some(output) = output else {
                fail(
                    "replace-audio",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "replace-audio",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            if same_file(&input, &output) {
                fail(
                    "replace-audio",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            let ffmpeg = if mute {
                recipes::mute_argv(&input, &output)
            } else if let Some(audio) = audio {
                if !std::path::Path::new(&audio).exists() {
                    fail(
                        "replace-audio",
                        "missing_audio",
                        format!("audio not found: {audio}"),
                    );
                }
                recipes::replace_audio_argv(&input, &audio, &output)
            } else if let Some(mix) = mix {
                if !std::path::Path::new(&mix).exists() {
                    fail(
                        "replace-audio",
                        "missing_audio",
                        format!("audio not found: {mix}"),
                    );
                }
                recipes::mix_audio_argv(&input, &mix, &output)
            } else {
                fail(
                    "replace-audio",
                    "missing_audio",
                    "replace-audio requires --mute, --audio, or --mix",
                );
            };
            let ffmpeg = prepare_argv("replace-audio", ffmpeg, ffmpeg_bin, cli.dry_run);
            let envelope = ok_envelope("replace-audio", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::ExtractAudio {
            input,
            output,
            format,
        } => {
            let Some(output) = output else {
                fail(
                    "extract-audio",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "extract-audio",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            let ext = format
                .as_deref()
                .or_else(|| std::path::Path::new(&output).extension()?.to_str())
                .unwrap_or("mp3");
            let codec = match ext {
                "mp3" => "libmp3lame",
                "wav" => "pcm_s16le",
                "aac" => "aac",
                "flac" => "flac",
                "copy" => "copy",
                other => fail(
                    "extract-audio",
                    "unknown_format",
                    format!("unknown audio format: {other}"),
                ),
            };
            if same_file(&input, &output) {
                fail(
                    "extract-audio",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            let ffmpeg = prepare_argv(
                "extract-audio",
                recipes::extract_audio_argv(&input, &output, codec),
                ffmpeg_bin,
                cli.dry_run,
            );
            let envelope = ok_envelope("extract-audio", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Speed {
            input,
            factor,
            output,
        } => {
            let Some(output) = output else {
                fail(
                    "speed",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "speed",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            if same_file(&input, &output) {
                fail(
                    "speed",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if factor <= 0.0 {
                fail(
                    "speed",
                    "invalid_factor",
                    "speed factor must be greater than 0",
                );
            }
            if cli.copy_only {
                fail(
                    "speed",
                    "copy_only",
                    "speed requires re-encode; --copy-only refuses this operation",
                );
            }
            let ffmpeg = prepare_argv(
                "speed",
                recipes::speed_argv(&input, &output, factor),
                ffmpeg_bin,
                cli.dry_run,
            );
            let envelope = ok_envelope("speed", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Resize {
            input,
            preset,
            output,
        } => {
            let Some(output) = output else {
                fail(
                    "resize",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail(
                    "resize",
                    "missing_input",
                    format!("input not found: {input}"),
                );
            }
            if same_file(&input, &output) {
                fail(
                    "resize",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if cli.copy_only {
                fail(
                    "resize",
                    "copy_only",
                    "resize requires re-encode; --copy-only refuses this operation",
                );
            }
            let Some(preset) = preset else {
                fail(
                    "resize",
                    "missing_preset",
                    "resize requires --preset or --width and --height",
                );
            };
            let (w, h) = recipes::preset_size(&preset).unwrap_or_else(|| {
                fail(
                    "resize",
                    "unknown_preset",
                    format!("unknown preset: {preset}"),
                )
            });
            let ffmpeg = prepare_argv(
                "resize",
                recipes::resize_argv(&input, &output, &recipes::scale_pad(w, h)),
                ffmpeg_bin,
                cli.dry_run,
            );
            let envelope = ok_envelope("resize", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Concat { inputs, output } => {
            let Some(output) = output else {
                fail(
                    "concat",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if inputs.len() < 2 {
                fail(
                    "concat",
                    "too_few_inputs",
                    "concat requires at least two inputs",
                );
            }
            for input in &inputs {
                if !std::path::Path::new(input).exists() {
                    fail(
                        "concat",
                        "missing_input",
                        format!("input not found: {input}"),
                    );
                }
                if same_file(input, &output) {
                    fail(
                        "concat",
                        "in_place",
                        "refusing in-place edit: output resolves to the same file as input",
                    );
                }
            }
            if cli.no_overwrite && std::path::Path::new(&output).exists() {
                fail(
                    "concat",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {output}"),
                );
            }
            let shapes: Vec<_> = inputs
                .iter()
                .filter_map(|input| probe_video(ffprobe_bin, input))
                .collect();
            let matched = shapes.len() == inputs.len() && shapes.windows(2).all(|w| w[0] == w[1]);
            let copy = shapes.is_empty() || matched;
            if !copy && cli.copy_only {
                fail(
                    "concat",
                    "copy_only",
                    "mismatched concat requires re-encode; --copy-only refuses this operation",
                );
            }
            let list_path = if cli.dry_run {
                "concat-list.txt".to_string()
            } else {
                write_concat_list(&inputs)
            };
            let ffmpeg = prepare_argv(
                "concat",
                recipes::concat_argv(&list_path, &output, copy),
                ffmpeg_bin,
                cli.dry_run,
            );
            if !cli.dry_run {
                let _ = std::fs::remove_file(&list_path);
            }
            let envelope = ok_envelope("concat", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Info { input } => {
            if !std::path::Path::new(&input).exists() {
                fail("info", "missing_input", format!("input not found: {input}"));
            }
            let argv = vec![
                ffprobe_bin.to_string(),
                "-v".into(),
                "quiet".into(),
                "-print_format".into(),
                "json".into(),
                "-show_format".into(),
                "-show_streams".into(),
                input.clone(),
            ];
            let output = std::process::Command::new(ffprobe_bin)
                .args(&argv[1..])
                .output()
                .unwrap_or_else(|e| fail("info", "ffprobe_failed", e.to_string()));
            if !output.status.success() {
                fail(
                    "info",
                    "ffprobe_failed",
                    String::from_utf8_lossy(&output.stderr),
                );
            }
            let probe: serde_json::Value = serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|e| fail("info", "ffprobe_failed", e.to_string()));
            let duration_s = probe["format"]["duration"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| probe["format"]["duration"].as_f64())
                .unwrap_or(0.0);
            let size_bytes = probe["format"]["size"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| probe["format"]["size"].as_u64())
                .unwrap_or(0);
            let video = probe["streams"]
                .as_array()
                .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"));
            let width = video.and_then(|s| s["width"].as_u64()).unwrap_or(0) as u32;
            let height = video.and_then(|s| s["height"].as_u64()).unwrap_or(0) as u32;
            let envelope = InfoEnvelope {
                ok: true,
                op: "info",
                duration_s,
                width,
                height,
                size_bytes,
                ffmpeg: argv,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Doctor => {
            let ffmpeg_version = tool_version(ffmpeg_bin);
            let ffprobe_version = tool_version(ffprobe_bin);
            let ffmpeg_found = ffmpeg_version.is_some();
            let ffprobe_found = ffprobe_version.is_some();
            let ok = ffmpeg_found && ffprobe_found;
            let envelope = DoctorEnvelope {
                ok,
                op: "doctor",
                ffmpeg_found,
                ffprobe_found,
                ffmpeg_version: ffmpeg_version.unwrap_or_default(),
                ffprobe_version: ffprobe_version.unwrap_or_default(),
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
            if !ok {
                std::process::exit(1);
            }
        }
        Command::Trim {
            input,
            from,
            to,
            output,
            accurate,
        } => {
            let Some(output) = output else {
                fail(
                    "trim",
                    "missing_output",
                    "mutating commands require -o / --output",
                );
            };
            if !std::path::Path::new(&input).exists() {
                fail("trim", "missing_input", format!("input not found: {input}"));
            }
            if same_file(&input, &output) {
                fail(
                    "trim",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if cli.no_overwrite && std::path::Path::new(&output).exists() {
                fail(
                    "trim",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {output}"),
                );
            }
            if accurate && cli.copy_only {
                fail(
                    "trim",
                    "copy_only",
                    "accurate trim requires re-encode; --copy-only refuses this operation",
                );
            }
            let ffmpeg = prepare_argv(
                "trim",
                recipes::trim_argv(&from, &to, &input, &output, accurate),
                ffmpeg_bin,
                cli.dry_run,
            );
            let envelope = ok_envelope("trim", output, ffmpeg, ffprobe_bin, cli.dry_run);
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
    }
}
