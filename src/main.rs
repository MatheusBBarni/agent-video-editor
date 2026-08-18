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
    Run { plan: String },
    Info { input: String },
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
struct ConvertEnvelope {
    ok: bool,
    op: &'static str,
    output: String,
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

fn same_file(a: &str, b: &str) -> bool {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let pa = cwd.join(a);
    let pb = cwd.join(b);
    match (pa.canonicalize(), pb.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => pa == pb,
    }
}

fn run_ffmpeg(op: &'static str, argv: &[String]) {
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .unwrap_or_else(|e| fail(op, "ffmpeg_failed", e.to_string()));
    if !output.status.success() {
        fail(
            op,
            "ffmpeg_failed",
            String::from_utf8_lossy(&output.stderr),
        );
    }
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

fn atempo_filter(mut factor: f64) -> String {
    let mut parts = Vec::new();
    while factor > 2.0 {
        parts.push("atempo=2.0".to_string());
        factor /= 2.0;
    }
    while factor < 0.5 {
        parts.push("atempo=0.5".to_string());
        factor /= 0.5;
    }
    parts.push(format!("atempo={factor}"));
    parts.join(",")
}

fn tool_version(bin: &str) -> Option<String> {
    let output = std::process::Command::new(bin).arg("-version").output().ok()?;
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
            let mut planned = HashSet::new();
            let mut steps = Vec::new();
            for step in parsed.steps {
                let op = step["op"].as_str().unwrap_or("");
                if op != "trim" {
                    fail("run", "unknown_op", format!("unknown op: {op}"));
                }
                let input = step["input"].as_str().unwrap_or("").to_string();
                let output = match step["output"].as_str() {
                    Some(o) => o.to_string(),
                    None => fail("trim", "missing_output", "mutating commands require -o / --output"),
                };
                let from = step["from"].as_str().unwrap_or("").to_string();
                let to = step["to"].as_str().unwrap_or("").to_string();
                let input_ready = std::path::Path::new(&input).exists()
                    || (cli.dry_run && planned.contains(&input));
                if !input_ready {
                    fail("trim", "missing_input", format!("input not found: {input}"));
                }
                if same_file(&input, &output) {
                    fail(
                        "trim",
                        "in_place",
                        "refusing in-place edit: output resolves to the same file as input",
                    );
                }
                let accurate = step["accurate"].as_bool().unwrap_or(false);
                let ffmpeg = if accurate {
                    vec![
                        "ffmpeg".into(),
                        "-y".into(),
                        "-ss".into(),
                        from,
                        "-to".into(),
                        to,
                        "-i".into(),
                        input,
                        "-c:v".into(),
                        "libx264".into(),
                        "-pix_fmt".into(),
                        "yuv420p".into(),
                        "-crf".into(),
                        "23".into(),
                        "-preset".into(),
                        "medium".into(),
                        "-c:a".into(),
                        "aac".into(),
                        "-movflags".into(),
                        "+faststart".into(),
                        output.clone(),
                    ]
                } else {
                    vec![
                        "ffmpeg".into(),
                        "-y".into(),
                        "-ss".into(),
                        from,
                        "-to".into(),
                        to,
                        "-i".into(),
                        input,
                        "-c".into(),
                        "copy".into(),
                        output.clone(),
                    ]
                };
                planned.insert(output.clone());
                steps.push(Envelope {
                    ok: true,
                    op: "trim",
                    output,
                    ffmpeg,
                });
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
                fail("convert", "missing_output", "mutating commands require -o / --output");
            };
            if !std::path::Path::new(&input).exists() {
                fail("convert", "missing_input", format!("input not found: {input}"));
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
            if gif {
                let pass1 = vec![
                    "ffmpeg".into(),
                    "-y".into(),
                    "-i".into(),
                    input.clone(),
                    "-vf".into(),
                    "fps=15,scale=480:-1:flags=lanczos,palettegen".into(),
                    "palette.png".into(),
                ];
                let pass2 = vec![
                    "ffmpeg".into(),
                    "-y".into(),
                    "-i".into(),
                    input,
                    "-i".into(),
                    "palette.png".into(),
                    "-filter_complex".into(),
                    "fps=15,scale=480:-1:flags=lanczos[x];[x][1:v]paletteuse".into(),
                    output.clone(),
                ];
                let envelope = ConvertEnvelope {
                    ok: true,
                    op: "convert",
                    output,
                    ffmpeg: pass2.clone(),
                    passes: vec![pass1, pass2],
                };
                println!("{}", serde_json::to_string(&envelope).expect("json"));
            } else {
                let ffmpeg = vec!["ffmpeg".into(), "-y".into(), "-i".into(), input, output.clone()];
                let envelope = Envelope {
                    ok: true,
                    op: "convert",
                    output,
                    ffmpeg,
                };
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
                fail("compress", "missing_output", "mutating commands require -o / --output");
            };
            if !std::path::Path::new(&input).exists() {
                fail("compress", "missing_input", format!("input not found: {input}"));
            }
            if cli.copy_only {
                fail(
                    "compress",
                    "copy_only",
                    "compress requires re-encode; --copy-only refuses this operation",
                );
            }
            let ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-i".into(),
                input,
                "-crf".into(),
                crf.to_string(),
                "-preset".into(),
                preset,
                "-c:a".into(),
                "copy".into(),
                output.clone(),
            ];
            let envelope = Envelope {
                ok: true,
                op: "compress",
                output,
                ffmpeg,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Overlay {
            input,
            image,
            position,
            output,
        } => {
            let Some(output) = output else {
                fail("overlay", "missing_output", "mutating commands require -o / --output");
            };
            if !std::path::Path::new(&input).exists() {
                fail("overlay", "missing_input", format!("input not found: {input}"));
            }
            if !std::path::Path::new(&image).exists() {
                fail("overlay", "missing_image", format!("image not found: {image}"));
            }
            if cli.copy_only {
                fail(
                    "overlay",
                    "copy_only",
                    "overlay requires re-encode; --copy-only refuses this operation",
                );
            }
            let expr = match position.as_deref().unwrap_or("top-right") {
                "top-left" => "overlay=10:10",
                "top-right" => "overlay=W-w-10:10",
                "bottom-left" => "overlay=10:H-h-10",
                "bottom-right" => "overlay=W-w-10:H-h-10",
                "center" => "overlay=(W-w)/2:(H-h)/2",
                other => fail(
                    "overlay",
                    "unknown_position",
                    format!("unknown position: {other}"),
                ),
            };
            let ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-i".into(),
                input,
                "-i".into(),
                image,
                "-filter_complex".into(),
                expr.into(),
                "-c:a".into(),
                "copy".into(),
                output.clone(),
            ];
            let envelope = Envelope {
                ok: true,
                op: "overlay",
                output,
                ffmpeg,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::ReplaceAudio {
            input,
            mute,
            audio: _,
            mix: _,
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
            if !mute {
                fail(
                    "replace-audio",
                    "missing_audio",
                    "replace-audio requires --mute, --audio, or --mix",
                );
            }
            let ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-i".into(),
                input,
                "-c:v".into(),
                "copy".into(),
                "-an".into(),
                output.clone(),
            ];
            let envelope = Envelope {
                ok: true,
                op: "replace-audio",
                output,
                ffmpeg,
            };
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
            let ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-i".into(),
                input,
                "-vn".into(),
                "-acodec".into(),
                codec.into(),
                output.clone(),
            ];
            let envelope = Envelope {
                ok: true,
                op: "extract-audio",
                output,
                ffmpeg,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Speed {
            input,
            factor,
            output,
        } => {
            let Some(output) = output else {
                fail("speed", "missing_output", "mutating commands require -o / --output");
            };
            if !std::path::Path::new(&input).exists() {
                fail("speed", "missing_input", format!("input not found: {input}"));
            }
            if same_file(&input, &output) {
                fail(
                    "speed",
                    "in_place",
                    "refusing in-place edit: output resolves to the same file as input",
                );
            }
            if factor <= 0.0 {
                fail("speed", "invalid_factor", "speed factor must be greater than 0");
            }
            if cli.copy_only {
                fail(
                    "speed",
                    "copy_only",
                    "speed requires re-encode; --copy-only refuses this operation",
                );
            }
            let setpts = format!("setpts={}*PTS", 1.0 / factor);
            let atempo = atempo_filter(factor);
            let ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-i".into(),
                input,
                "-filter:v".into(),
                setpts,
                "-filter:a".into(),
                atempo,
                output.clone(),
            ];
            let envelope = Envelope {
                ok: true,
                op: "speed",
                output,
                ffmpeg,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Resize {
            input,
            preset,
            output,
        } => {
            let Some(output) = output else {
                fail("resize", "missing_output", "mutating commands require -o / --output");
            };
            if !std::path::Path::new(&input).exists() {
                fail("resize", "missing_input", format!("input not found: {input}"));
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
                fail("resize", "missing_preset", "resize requires --preset or --width and --height");
            };
            let (w, h) = match preset.as_str() {
                "tiktok" => (1080, 1920),
                "youtube" | "twitter" => (1920, 1080),
                "instagram" => (1080, 1350),
                "square" => (1080, 1080),
                other => fail("resize", "unknown_preset", format!("unknown preset: {other}")),
            };
            let vf = format!(
                "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
            );
            let ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-i".into(),
                input,
                "-vf".into(),
                vf,
                "-c:v".into(),
                "libx264".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-crf".into(),
                "23".into(),
                "-preset".into(),
                "medium".into(),
                "-c:a".into(),
                "copy".into(),
                output.clone(),
            ];
            let envelope = Envelope {
                ok: true,
                op: "resize",
                output,
                ffmpeg,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
        Command::Concat { inputs, output } => {
            let Some(output) = output else {
                fail("concat", "missing_output", "mutating commands require -o / --output");
            };
            if inputs.len() < 2 {
                fail("concat", "too_few_inputs", "concat requires at least two inputs");
            }
            for input in &inputs {
                if !std::path::Path::new(input).exists() {
                    fail("concat", "missing_input", format!("input not found: {input}"));
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
            let matched = shapes.len() == inputs.len()
                && shapes.windows(2).all(|w| w[0] == w[1]);
            let copy = shapes.is_empty() || matched;
            let mut ffmpeg = vec![
                "ffmpeg".into(),
                "-y".into(),
                "-f".into(),
                "concat".into(),
                "-safe".into(),
                "0".into(),
                "-i".into(),
                "concat-list.txt".into(),
            ];
            if copy {
                ffmpeg.extend(["-c".into(), "copy".into()]);
            } else {
                ffmpeg.extend([
                    "-c:v".into(),
                    "libx264".into(),
                    "-pix_fmt".into(),
                    "yuv420p".into(),
                    "-crf".into(),
                    "23".into(),
                    "-preset".into(),
                    "medium".into(),
                    "-c:a".into(),
                    "aac".into(),
                    "-movflags".into(),
                    "+faststart".into(),
                ]);
            }
            ffmpeg.push(output.clone());
            let envelope = Envelope {
                ok: true,
                op: "concat",
                output,
                ffmpeg,
            };
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
                .and_then(|streams| {
                    streams.iter().find(|s| s["codec_type"] == "video")
                });
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
                fail("trim", "missing_output", "mutating commands require -o / --output");
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
            let ffmpeg = if accurate {
                vec![
                    "ffmpeg".into(),
                    "-y".into(),
                    "-ss".into(),
                    from,
                    "-to".into(),
                    to,
                    "-i".into(),
                    input,
                    "-c:v".into(),
                    "libx264".into(),
                    "-pix_fmt".into(),
                    "yuv420p".into(),
                    "-crf".into(),
                    "23".into(),
                    "-preset".into(),
                    "medium".into(),
                    "-c:a".into(),
                    "aac".into(),
                    "-movflags".into(),
                    "+faststart".into(),
                    output.clone(),
                ]
            } else {
                vec![
                    "ffmpeg".into(),
                    "-y".into(),
                    "-ss".into(),
                    from,
                    "-to".into(),
                    to,
                    "-i".into(),
                    input,
                    "-c".into(),
                    "copy".into(),
                    output.clone(),
                ]
            };
            if !cli.dry_run {
                run_ffmpeg("trim", &ffmpeg);
            }
            let envelope = Envelope {
                ok: true,
                op: "trim",
                output,
                ffmpeg,
            };
            println!("{}", serde_json::to_string(&envelope).expect("json"));
        }
    }
}
