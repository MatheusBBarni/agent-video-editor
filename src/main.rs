mod error;
mod exec;
mod op;
mod probe;
mod recipes;
mod skill;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use error::{Error, RunEnvelope, print_json};
use exec::{Ctx, Outcome, execute, run_plan};
use op::{Op, TrimEnd, replace_audio_choice, require_output};

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
        to: Option<String>,
        #[arg(long)]
        duration: Option<String>,
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
    /// Install the ave agent skill into one folder; symlink the rest
    #[command(name = "install-skill")]
    InstallSkill {
        /// Agent providers to install into (repeatable or comma-separated).
        /// One of: agents, claude, pi, cursor, all.
        /// First provider gets the files; the others get a symlink to it.
        #[arg(long, value_enum, value_delimiter = ',', num_args = 1..)]
        provider: Vec<skill::Provider>,
        /// Install into DIR/ave (repeatable). Skips --provider.
        /// First dir gets the files; extra dirs get a symlink to it.
        #[arg(long = "dir")]
        dirs: Vec<std::path::PathBuf>,
        /// Use home-dir paths (~/.claude/skills, …) instead of the project
        #[arg(long)]
        global: bool,
    },
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                let _ = err.print();
                std::process::exit(err.exit_code());
            }
            _ => error::fail(Error::new(usage_op(), "usage", err.to_string())),
        },
    };
    let ctx = Ctx {
        dry_run: cli.dry_run,
        no_overwrite: cli.no_overwrite,
        copy_only: cli.copy_only,
        ffmpeg: cli.ffmpeg.clone().unwrap_or_else(|| "ffmpeg".into()),
        ffprobe: cli.ffprobe.clone().unwrap_or_else(|| "ffprobe".into()),
    };

    match cli.command {
        Command::Run { plan } => run_cmd(&plan, &ctx),
        Command::InstallSkill {
            provider,
            dirs,
            global,
        } => {
            skill::install(&dirs, &provider, global, cli.dry_run, cli.no_overwrite);
        }
        command => match to_op(command).and_then(|op| execute(&op, &ctx)) {
            Ok(Outcome::Edit(env)) => print_json(&env),
            Ok(Outcome::Info(env)) => print_json(&env),
            Ok(Outcome::Doctor(env)) => {
                let ok = env.ok;
                print_json(&env);
                if !ok {
                    std::process::exit(1);
                }
            }
            Err(err) => error::fail(err),
        },
    }
}

fn usage_op() -> &'static str {
    const OPS: &[&str] = &[
        "trim",
        "doctor",
        "run",
        "info",
        "concat",
        "resize",
        "convert",
        "compress",
        "overlay",
        "replace-audio",
        "extract-audio",
        "speed",
        "install-skill",
    ];
    std::env::args()
        .skip(1)
        .find(|arg| OPS.contains(&arg.as_str()))
        .map(|arg| match arg.as_str() {
            "trim" => "trim",
            "doctor" => "doctor",
            "run" => "run",
            "info" => "info",
            "concat" => "concat",
            "resize" => "resize",
            "convert" => "convert",
            "compress" => "compress",
            "overlay" => "overlay",
            "replace-audio" => "replace-audio",
            "extract-audio" => "extract-audio",
            "speed" => "speed",
            "install-skill" => "install-skill",
            _ => "ave",
        })
        .unwrap_or("ave")
}

fn run_cmd(plan: &str, ctx: &Ctx) {
    let text = if plan == "-" {
        use std::io::Read;
        let mut buf = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
            error::fail(error::Error::new("run", "bad_plan", e.to_string()));
        }
        buf
    } else {
        match std::fs::read_to_string(plan) {
            Ok(t) => t,
            Err(e) => error::fail(error::Error::new("run", "bad_plan", e.to_string())),
        }
    };
    let parsed: PlanFile = match serde_json::from_str(&text) {
        Ok(p) => p,
        Err(e) => error::fail(error::Error::new("run", "bad_plan", e.to_string())),
    };

    let mut ops = Vec::new();
    for step in &parsed.steps {
        match Op::from_json(step) {
            Ok(op) => ops.push(op),
            Err(err) => error::fail(err),
        }
    }

    match run_plan(&ops, ctx) {
        Ok(steps) => print_json(&RunEnvelope {
            ok: true,
            op: "run",
            steps,
            failed_step: None,
            error: None,
            message: None,
            written: None,
        }),
        Err((failed_step, err, steps, written)) => {
            print_json(&RunEnvelope {
                ok: false,
                op: "run",
                steps,
                failed_step: Some(failed_step),
                error: Some(err.code),
                message: Some(err.message),
                written: Some(written),
            });
            std::process::exit(1);
        }
    }
}

#[derive(serde::Deserialize)]
struct PlanFile {
    steps: Vec<serde_json::Value>,
}

fn to_op(command: Command) -> Result<Op, error::Error> {
    match command {
        Command::Run { .. } | Command::InstallSkill { .. } => {
            unreachable!("handled separately")
        }
        Command::Doctor => Ok(Op::Doctor),
        Command::Info { input } => Ok(Op::Info { input }),
        Command::Trim {
            input,
            from,
            to,
            duration,
            output,
            accurate,
        } => {
            let end = TrimEnd::exclusive(to, duration, "trim")?;
            end.validate_against(&from, "trim")?;
            Ok(Op::Trim {
                input,
                from,
                end,
                output: require_output("trim", output)?,
                accurate,
            })
        }
        Command::Concat { inputs, output } => {
            if inputs.len() < 2 {
                return Err(error::Error::new(
                    "concat",
                    "too_few_inputs",
                    "concat requires at least two inputs",
                ));
            }
            Ok(Op::Concat {
                inputs,
                output: require_output("concat", output)?,
            })
        }
        Command::Resize {
            input,
            preset,
            output,
        } => Ok(Op::Resize {
            input,
            output: require_output("resize", output)?,
            preset,
            width: None,
            height: None,
        }),
        Command::Convert { input, output } => Ok(Op::Convert {
            input,
            output: require_output("convert", output)?,
        }),
        Command::Compress {
            input,
            crf,
            preset,
            output,
        } => Ok(Op::Compress {
            input,
            output: require_output("compress", output)?,
            crf,
            preset,
        }),
        Command::Overlay {
            input,
            image,
            position,
            output,
        } => Ok(Op::Overlay {
            input,
            image,
            output: require_output("overlay", output)?,
            position,
        }),
        Command::ReplaceAudio {
            input,
            mute,
            audio,
            mix,
            output,
        } => {
            let (mute, audio, mix) = replace_audio_choice(mute, audio, mix, "replace-audio")?;
            Ok(Op::ReplaceAudio {
                input,
                output: require_output("replace-audio", output)?,
                mute,
                audio,
                mix,
            })
        }
        Command::ExtractAudio {
            input,
            output,
            format,
        } => Ok(Op::ExtractAudio {
            input,
            output: require_output("extract-audio", output)?,
            format,
        }),
        Command::Speed {
            input,
            factor,
            output,
        } => Ok(Op::Speed {
            input,
            output: require_output("speed", output)?,
            factor,
        }),
    }
}
