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
        output: String,
    },
}

#[derive(Serialize)]
struct Envelope {
    ok: bool,
    op: &'static str,
    output: String,
    ffmpeg: Vec<String>,
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
