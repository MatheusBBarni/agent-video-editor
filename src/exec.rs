use crate::error::{DoctorEnvelope, Envelope, Error, InfoEnvelope};
use crate::op::{KeepRange, Op, TrimEnd, parse_timestamp};
use crate::probe::{self, media_meta};
use crate::recipes;

pub struct Ctx {
    pub dry_run: bool,
    pub no_overwrite: bool,
    pub copy_only: bool,
    pub ffmpeg: String,
    pub ffprobe: String,
}

pub enum Outcome {
    Edit(Envelope),
    Info(InfoEnvelope),
    Doctor(DoctorEnvelope),
}

struct Job {
    argv: Vec<String>,
    passes: Option<Vec<Vec<String>>>,
    cleanup: Vec<String>,
    reencode: bool,
}

pub fn execute(op: &Op, ctx: &Ctx) -> Result<Outcome, Error> {
    execute_assuming(op, ctx, &std::collections::HashSet::new())
}

fn execute_assuming(
    op: &Op,
    ctx: &Ctx,
    assumed: &std::collections::HashSet<String>,
) -> Result<Outcome, Error> {
    match op {
        Op::Doctor => return doctor(ctx),
        Op::Info { input } => return info(input, ctx),
        _ => {}
    }

    let name = op.name();
    let output = op
        .output()
        .ok_or_else(|| {
            Error::new(
                name,
                "missing_output",
                "mutating commands require -o / --output",
            )
        })?
        .to_string();

    for input in op.inputs() {
        let ready =
            std::path::Path::new(input).exists() || (ctx.dry_run && assumed.contains(input));
        if !ready {
            return Err(Error::new(
                name,
                "missing_input",
                format!("input not found: {input}"),
            ));
        }
        if same_file(input, &output) {
            return Err(Error::new(
                name,
                "in_place",
                "refusing in-place edit: output resolves to the same file as input",
            ));
        }
    }
    if ctx.no_overwrite && std::path::Path::new(&output).exists() {
        return Err(Error::new(
            name,
            "output_exists",
            format!("output exists and --no-overwrite was set: {output}"),
        ));
    }

    let job = build_job(op, ctx)?;
    if job.reencode && ctx.copy_only {
        return Err(Error::new(
            name,
            "copy_only",
            format!("{name} requires re-encode; --copy-only refuses this operation"),
        ));
    }

    if !ctx.dry_run {
        let cmds = job
            .passes
            .as_deref()
            .unwrap_or(std::slice::from_ref(&job.argv));
        let result = cmds
            .iter()
            .try_for_each(|cmd| run_ffmpeg(cmd).map_err(|e| Error::ffmpeg(name, e)));
        for path in &job.cleanup {
            let _ = std::fs::remove_file(path);
        }
        result?;
    }

    let (duration_s, width, height, size_bytes) = if ctx.dry_run {
        (0.0, 0, 0, 0)
    } else {
        media_meta(&ctx.ffprobe, &output)
    };

    Ok(Outcome::Edit(Envelope {
        ok: true,
        op: name,
        output,
        duration_s,
        width,
        height,
        size_bytes,
        ffmpeg: job.argv,
        passes: job.passes,
    }))
}

type RunFail = (usize, Error, Vec<Envelope>, Vec<String>);

pub fn run_plan(ops: &[Op], ctx: &Ctx) -> Result<Vec<Envelope>, RunFail> {
    let mut planned = std::collections::HashSet::new();
    let mut steps = Vec::new();
    let mut written = Vec::new();

    for (idx, op) in ops.iter().enumerate() {
        for input in op.inputs() {
            let ready =
                std::path::Path::new(input).exists() || (ctx.dry_run && planned.contains(input));
            if !ready {
                return Err((
                    idx,
                    Error::new(
                        op.name(),
                        "missing_input",
                        format!("input not found: {input}"),
                    ),
                    steps,
                    written,
                ));
            }
        }
        match execute_assuming(op, ctx, &planned) {
            Ok(Outcome::Edit(env)) => {
                if !ctx.dry_run {
                    written.push(env.output.clone());
                }
                planned.insert(env.output.clone());
                steps.push(env);
            }
            Ok(_) => {}
            Err(err) => return Err((idx, err, steps, written)),
        }
    }
    Ok(steps)
}

fn primary_audio(op: &Op, ctx: &Ctx) -> Option<bool> {
    probe::probed_has_audio(&ctx.ffprobe, op.inputs().first()?)
}

fn build_job(op: &Op, ctx: &Ctx) -> Result<Job, Error> {
    let bin = ctx.ffmpeg.as_str();
    let audio = primary_audio(op, ctx);
    match op {
        Op::Trim {
            input,
            from,
            end,
            output,
            accurate,
        } => Ok(trim_job(
            input,
            from,
            end,
            output,
            *accurate,
            audio.unwrap_or(false),
            bin,
        )),
        Op::Concat { inputs, output } => concat_job(inputs, output, ctx, bin),
        Op::CutOut {
            input,
            from,
            to,
            output,
            accurate,
        } => cut_out_job(input, from, to, output, *accurate, ctx, bin),
        Op::Keep {
            input,
            ranges,
            output,
            accurate,
        } => keep_job(input, ranges, output, *accurate, ctx, bin),
        Op::Resize {
            input,
            output,
            fit,
            ..
        } => {
            let (w, h) = op.resize_size()?;
            let vf = match fit.as_deref() {
                Some("crop") => recipes::scale_crop(w, h),
                Some("stretch") => recipes::scale_stretch(w, h),
                _ => recipes::scale_pad(w, h),
            };
            Ok(Job {
                argv: recipes::with_bin(
                    recipes::resize_argv(input, output, &vf, audio.unwrap_or(false)),
                    bin,
                ),
                passes: None,
                cleanup: vec![],
                reencode: true,
            })
        }
        Op::Speed {
            input,
            output,
            factor,
        } => {
            if *factor <= 0.0 {
                return Err(Error::new(
                    "speed",
                    "invalid_factor",
                    "speed factor must be greater than 0",
                ));
            }
            Ok(Job {
                argv: recipes::with_bin(
                    recipes::speed_argv(input, output, *factor, audio.unwrap_or(false)),
                    bin,
                ),
                passes: None,
                cleanup: vec![],
                reencode: true,
            })
        }
        Op::ExtractAudio {
            input,
            output,
            format,
        } => {
            match audio {
                None if std::path::Path::new(input).exists() => {
                    return Err(Error::new(
                        "extract-audio",
                        "ffprobe_failed",
                        format!("could not probe input: {input}"),
                    ));
                }
                Some(false) => {
                    return Err(Error::new(
                        "extract-audio",
                        "no_audio",
                        format!("input has no audio stream: {input}"),
                    ));
                }
                None | Some(true) => {}
            }
            let ext = format.as_deref().unwrap_or_else(|| {
                std::path::Path::new(output)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("mp3")
            });
            let codec = recipes::audio_codec(ext).ok_or_else(|| {
                Error::new(
                    "extract-audio",
                    "unknown_format",
                    format!("unknown audio format: {ext}"),
                )
            })?;
            Ok(Job {
                argv: recipes::with_bin(recipes::extract_audio_argv(input, output, codec), bin),
                passes: None,
                cleanup: vec![],
                reencode: false,
            })
        }
        Op::ReplaceAudio {
            input,
            output,
            mute,
            audio,
            mix,
        } => {
            let argv = if *mute {
                recipes::mute_argv(input, output)
            } else if let Some(audio) = audio {
                recipes::replace_audio_argv(input, audio, output)
            } else if let Some(mix) = mix {
                recipes::mix_audio_argv(input, mix, output)
            } else {
                return Err(Error::new(
                    "replace-audio",
                    "missing_audio",
                    "replace-audio requires --mute, --audio, or --mix",
                ));
            };
            Ok(Job {
                argv: recipes::with_bin(argv, bin),
                passes: None,
                cleanup: vec![],
                reencode: false,
            })
        }
        Op::Overlay {
            input,
            image,
            output,
            position,
        } => {
            let expr = recipes::overlay_expr(position.as_deref().unwrap_or("top-right"))
                .ok_or_else(|| {
                    Error::new(
                        "overlay",
                        "unknown_position",
                        format!("unknown position: {}", position.clone().unwrap_or_default()),
                    )
                })?;
            Ok(Job {
                argv: recipes::with_bin(
                    recipes::overlay_argv(input, image, output, expr, audio.unwrap_or(false)),
                    bin,
                ),
                passes: None,
                cleanup: vec![],
                reencode: true,
            })
        }
        Op::Compress {
            input,
            output,
            crf,
            preset,
        } => Ok(Job {
            argv: recipes::with_bin(
                recipes::compress_argv(input, output, *crf, preset, audio.unwrap_or(false)),
                bin,
            ),
            passes: None,
            cleanup: vec![],
            reencode: true,
        }),
        Op::Convert { input, output } => {
            let gif = std::path::Path::new(output)
                .extension()
                .and_then(|e| e.to_str())
                == Some("gif");
            if gif {
                let palette = unique_temp_file("palette", "png");
                let (pass1, pass2) = recipes::gif_passes(input, output, &palette);
                let pass1 = recipes::with_bin(pass1, bin);
                let pass2 = recipes::with_bin(pass2, bin);
                Ok(Job {
                    argv: pass2.clone(),
                    passes: Some(vec![pass1, pass2]),
                    cleanup: vec![palette],
                    reencode: true,
                })
            } else {
                Ok(Job {
                    argv: recipes::with_bin(recipes::convert_argv(input, output), bin),
                    passes: None,
                    cleanup: vec![],
                    reencode: false,
                })
            }
        }
        Op::Frame {
            input,
            at,
            output,
        } => Ok(Job {
            argv: recipes::with_bin(recipes::frame_argv(input, at, output), bin),
            passes: None,
            cleanup: vec![],
            reencode: true,
        }),
        Op::Info { .. } | Op::Doctor => Err(Error::new(op.name(), "internal", "not a mutating op")),
    }
}

fn trim_job(
    input: &str,
    from: &str,
    end: &TrimEnd,
    output: &str,
    accurate: bool,
    has_audio: bool,
    bin: &str,
) -> Job {
    let (end_flag, end_val) = end.ffmpeg_flag();
    Job {
        argv: recipes::with_bin(
            recipes::trim_argv(from, end_flag, end_val, input, output, accurate, has_audio),
            bin,
        ),
        passes: None,
        cleanup: vec![],
        reencode: accurate,
    }
}

enum ConcatPlan {
    Remux {
        video_bsf: &'static str,
        audio_bsf: Option<&'static str>,
    },
    Reencode,
}

fn concat_job(inputs: &[String], output: &str, ctx: &Ctx, bin: &str) -> Result<Job, Error> {
    concat_job_with_plan(inputs, output, ctx, bin, concat_plan(inputs, &ctx.ffprobe))
}

fn concat_job_with_plan(
    inputs: &[String],
    output: &str,
    ctx: &Ctx,
    bin: &str,
    plan: ConcatPlan,
) -> Result<Job, Error> {
    match plan {
        ConcatPlan::Remux {
            video_bsf,
            audio_bsf,
        } => {
            let ts_paths: Vec<String> = (0..inputs.len())
                .map(|i| unique_temp_file(&format!("concat-{i}"), "ts"))
                .collect();
            let mut passes: Vec<Vec<String>> = inputs
                .iter()
                .zip(&ts_paths)
                .map(|(input, ts)| {
                    recipes::with_bin(
                        recipes::concat_copy_to_mpegts_argv(input, ts, video_bsf),
                        bin,
                    )
                })
                .collect();
            let list_path = unique_temp_file("concat", "txt");
            if !ctx.dry_run {
                write_concat_list_at(&list_path, &ts_paths)?;
            }
            let concat = recipes::with_bin(
                recipes::concat_from_mpegts_argv(&list_path, output, audio_bsf),
                bin,
            );
            passes.push(concat.clone());
            let mut cleanup = ts_paths;
            cleanup.push(list_path);
            Ok(Job {
                argv: concat,
                passes: Some(passes),
                cleanup,
                reencode: false,
            })
        }
        ConcatPlan::Reencode => {
            let list_path = if ctx.dry_run {
                "concat-list.txt".to_string()
            } else {
                write_concat_list(inputs)?
            };
            Ok(Job {
                argv: recipes::with_bin(recipes::concat_argv(&list_path, output), bin),
                passes: None,
                cleanup: if ctx.dry_run { vec![] } else { vec![list_path] },
                reencode: true,
            })
        }
    }
}

fn concat_plan(inputs: &[String], ffprobe_bin: &str) -> ConcatPlan {
    let infos: Vec<_> = inputs
        .iter()
        .filter_map(|input| {
            let probe = probe::probe_json(ffprobe_bin, input)?;
            let info = probe::media_info_from_probe(&probe);
            info.has_video.then_some(info)
        })
        .collect();
    if infos.len() != inputs.len() {
        return ConcatPlan::Reencode;
    }
    if infos.windows(2).any(|w| !w[0].same_concat_shape(&w[1])) {
        return ConcatPlan::Reencode;
    }
    let Some(video_bsf) = recipes::mpegts_video_bsf(&infos[0].video_codec) else {
        return ConcatPlan::Reencode;
    };
    let audio_bsf =
        (infos[0].has_audio && infos[0].audio_codec == "aac").then_some("aac_adtstoasc");
    ConcatPlan::Remux {
        video_bsf,
        audio_bsf,
    }
}

fn keep_job(
    input: &str,
    ranges: &[KeepRange],
    output: &str,
    accurate: bool,
    ctx: &Ctx,
    bin: &str,
) -> Result<Job, Error> {
    let end_s = probed_duration(&ctx.ffprobe, input, "keep")?;
    let pieces = resolve_keep_ranges(ranges, end_s, "keep")?;
    keep_pieces_job(input, &pieces, output, accurate, ctx, bin)
}

struct KeepPiece {
    from: String,
    to: String,
}

fn resolve_keep_ranges(
    ranges: &[KeepRange],
    end_s: f64,
    op: &'static str,
) -> Result<Vec<KeepPiece>, Error> {
    let mut prev_to = 0.0;
    let mut seen = false;
    let mut pieces = Vec::new();
    for range in ranges {
        let from_s = parse_timestamp(&range.from).ok_or_else(|| {
            Error::new(
                op,
                "bad_timestamp",
                format!("invalid timestamp: {}", range.from),
            )
        })?;
        let to_s = if range.to == "end" {
            end_s
        } else {
            parse_timestamp(&range.to).ok_or_else(|| {
                Error::new(
                    op,
                    "bad_timestamp",
                    format!("invalid timestamp: {}", range.to),
                )
            })?
        };
        if from_s >= to_s {
            return Err(Error::new(op, "bad_range", "from must be less than to"));
        }
        if to_s > end_s {
            return Err(Error::new(
                op,
                "bad_range",
                "to is past the end of the input",
            ));
        }
        if seen && from_s < prev_to {
            return Err(Error::new(op, "bad_range", "keep ranges must not overlap"));
        }
        seen = true;
        prev_to = to_s;
        let to = if range.to == "end" {
            end_s.to_string()
        } else {
            range.to.clone()
        };
        pieces.push(KeepPiece {
            from: range.from.clone(),
            to,
        });
    }
    if pieces.is_empty() {
        return Err(Error::new(
            op,
            "bad_range",
            "keep requires at least one range",
        ));
    }
    Ok(pieces)
}

fn keep_pieces_job(
    input: &str,
    pieces: &[KeepPiece],
    output: &str,
    accurate: bool,
    ctx: &Ctx,
    bin: &str,
) -> Result<Job, Error> {
    let has_audio = probe::probed_has_audio(&ctx.ffprobe, input).unwrap_or(false);
    match pieces {
        [] => Err(Error::new(
            "keep",
            "bad_range",
            "keep requires at least one range",
        )),
        [one] => Ok(trim_job(
            input,
            &one.from,
            &TrimEnd::To(one.to.clone()),
            output,
            accurate,
            has_audio,
            bin,
        )),
        many => {
            let temps: Vec<String> = (0..many.len())
                .map(|i| unique_temp_file(&format!("keep-{i}"), "mp4"))
                .collect();
            let mut passes: Vec<Vec<String>> = many
                .iter()
                .zip(&temps)
                .map(|(piece, temp)| {
                    trim_job(
                        input,
                        &piece.from,
                        &TrimEnd::To(piece.to.clone()),
                        temp,
                        accurate,
                        has_audio,
                        bin,
                    )
                    .argv
                })
                .collect();
            let plan = if accurate {
                ConcatPlan::Remux {
                    video_bsf: "h264_mp4toannexb",
                    audio_bsf: has_audio.then_some("aac_adtstoasc"),
                }
            } else {
                concat_plan(&[input.to_string()], &ctx.ffprobe)
            };
            let concat = concat_job_with_plan(&temps, output, ctx, bin, plan)?;
            match &concat.passes {
                Some(extra) => passes.extend(extra.iter().cloned()),
                None => passes.push(concat.argv.clone()),
            }
            let mut cleanup = concat.cleanup;
            cleanup.extend(temps);
            Ok(Job {
                argv: concat.argv,
                passes: Some(passes),
                cleanup,
                reencode: accurate || concat.reencode,
            })
        }
    }
}

fn cut_out_job(
    input: &str,
    from: &str,
    to: &str,
    output: &str,
    accurate: bool,
    ctx: &Ctx,
    bin: &str,
) -> Result<Job, Error> {
    let end_s = probed_duration(&ctx.ffprobe, input, "cut-out")?;
    let from_s = parse_timestamp(from).ok_or_else(|| {
        Error::new(
            "cut-out",
            "bad_timestamp",
            format!("invalid timestamp: {from}"),
        )
    })?;
    let to_s = parse_timestamp(to).ok_or_else(|| {
        Error::new(
            "cut-out",
            "bad_timestamp",
            format!("invalid timestamp: {to}"),
        )
    })?;
    if from_s >= to_s {
        return Err(Error::new(
            "cut-out",
            "bad_range",
            "from must be less than to",
        ));
    }
    if to_s > end_s {
        return Err(Error::new(
            "cut-out",
            "bad_range",
            "to is past the end of the input",
        ));
    }
    if from_s <= 0.0 && to_s >= end_s {
        return Err(Error::new(
            "cut-out",
            "bad_range",
            "cut-out would leave nothing",
        ));
    }

    let mut pieces = Vec::new();
    if from_s > 0.0 {
        pieces.push(KeepPiece {
            from: "0".into(),
            to: from.to_string(),
        });
    }
    if to_s < end_s {
        pieces.push(KeepPiece {
            from: to.to_string(),
            to: end_s.to_string(),
        });
    }
    keep_pieces_job(input, &pieces, output, accurate, ctx, bin)
}

fn probed_duration(ffprobe_bin: &str, input: &str, op: &'static str) -> Result<f64, Error> {
    let probe = probe::probe_json(ffprobe_bin, input).ok_or_else(|| {
        Error::new(
            op,
            "ffprobe_failed",
            format!("could not probe input: {input}"),
        )
    })?;
    let duration = probe::media_info_from_probe(&probe).duration_s;
    if duration <= 0.0 {
        return Err(Error::new(
            op,
            "ffprobe_failed",
            format!("could not probe duration: {input}"),
        ));
    }
    Ok(duration)
}

fn unique_temp_file(kind: &str, ext: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "ave-{kind}-{}-{}.{ext}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
        .to_string_lossy()
        .into_owned()
}

fn write_concat_list(inputs: &[String]) -> Result<String, Error> {
    let path = unique_temp_file("concat", "txt");
    write_concat_list_at(&path, inputs)?;
    Ok(path)
}

fn write_concat_list_at(path: &str, inputs: &[String]) -> Result<(), Error> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut body = String::new();
    for input in inputs {
        let escaped = cwd.join(input).to_string_lossy().replace('\'', r"'\''");
        body.push_str(&format!("file '{escaped}'\n"));
    }
    std::fs::write(path, body).map_err(|e| Error::new("concat", "concat_list", e.to_string()))?;
    Ok(())
}

fn same_file(a: &str, b: &str) -> bool {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    resolve_path(&cwd, a) == resolve_path(&cwd, b)
}

fn resolve_path(cwd: &std::path::Path, path: &str) -> std::path::PathBuf {
    let joined = cwd.join(path);
    if let Ok(canon) = joined.canonicalize() {
        return canon;
    }
    let normalized = normalize_lexically(&joined);
    if let Ok(canon) = normalized.canonicalize() {
        return canon;
    }
    match (normalized.parent(), normalized.file_name()) {
        (Some(parent), Some(name)) => parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf())
            .join(name),
        _ => normalized,
    }
}

fn normalize_lexically(path: &std::path::Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn run_ffmpeg(argv: &[String]) -> Result<(), String> {
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn info(input: &str, ctx: &Ctx) -> Result<Outcome, Error> {
    if !std::path::Path::new(input).exists() {
        return Err(Error::new(
            "info",
            "missing_input",
            format!("input not found: {input}"),
        ));
    }
    let argv = probe::ffprobe_argv(&ctx.ffprobe, input);
    let probe = probe::probe_or_err(&ctx.ffprobe, input)?;
    let meta = probe::media_info_from_probe(&probe);
    Ok(Outcome::Info(InfoEnvelope {
        ok: true,
        op: "info",
        duration_s: meta.duration_s,
        width: meta.width,
        height: meta.height,
        size_bytes: meta.size_bytes,
        video_codec: meta.video_codec,
        audio_codec: meta.audio_codec,
        fps: meta.fps,
        has_video: meta.has_video,
        has_audio: meta.has_audio,
        rotate_deg: meta.rotate_deg,
        display_width: meta.display_width,
        display_height: meta.display_height,
        ffmpeg: argv,
    }))
}

fn doctor(ctx: &Ctx) -> Result<Outcome, Error> {
    let ffmpeg_version = probe::tool_version(&ctx.ffmpeg);
    let ffprobe_version = probe::tool_version(&ctx.ffprobe);
    Ok(Outcome::Doctor(DoctorEnvelope {
        ok: ffmpeg_version.is_some() && ffprobe_version.is_some(),
        op: "doctor",
        ffmpeg_found: ffmpeg_version.is_some(),
        ffprobe_found: ffprobe_version.is_some(),
        ffmpeg_version: ffmpeg_version.unwrap_or_default(),
        ffprobe_version: ffprobe_version.unwrap_or_default(),
    }))
}
