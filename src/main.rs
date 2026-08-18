use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "ave")]
struct Cli {
    #[arg(long, global = true)]
    dry_run: bool,
    #[arg(long, global = true)]
    no_overwrite: bool,
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
    },
    Doctor,
}

#[derive(Serialize)]
struct Envelope {
    ok: bool,
    op: &'static str,
    output: String,
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
            let ffmpeg = vec![
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
            ];
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
