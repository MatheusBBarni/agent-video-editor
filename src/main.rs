use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "ave")]
struct Cli {
    #[arg(long, global = true)]
    dry_run: bool,
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
}

#[derive(Serialize)]
struct Envelope {
    ok: bool,
    op: &'static str,
    output: String,
    ffmpeg: Vec<String>,
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

fn main() {
    let cli = Cli::parse();
    match cli.command {
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
