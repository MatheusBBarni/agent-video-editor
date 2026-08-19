use crate::error::{Error, FrameItem, FramesEnvelope};
use crate::exec::{Ctx, Outcome, probed_duration, run_ffmpeg};
use crate::op::Op;
use crate::recipes;

pub fn execute(op: &Op, ctx: &Ctx) -> Result<Outcome, Error> {
    let Op::Frames {
        input,
        at,
        every,
        sheet,
        output,
    } = op
    else {
        return Err(Error::new("frames", "internal", "not a frames op"));
    };
    let at = if !at.is_empty() {
        at.clone()
    } else if let Some(every) = every {
        let duration = probed_duration(&ctx.ffprobe, input, "frames")?;
        every_stamps(*every, duration)
    } else {
        return Err(Error::new(
            "frames",
            "missing_field",
            "frames requires --at or --every",
        ));
    };
    if !std::path::Path::new(input).exists() {
        return Err(Error::new(
            "frames",
            "missing_input",
            format!("input not found: {input}"),
        ));
    }
    if ctx.copy_only {
        return Err(Error::new(
            "frames",
            "copy_only",
            "frames requires still extract; --copy-only refuses this operation",
        ));
    }
    let items: Vec<FrameItem> = at
        .iter()
        .map(|stamp| FrameItem {
            at: stamp.clone(),
            path: format!("{output}/{}", still_name(stamp)),
        })
        .collect();
    if items.is_empty() {
        return Err(Error::new(
            "frames",
            "missing_field",
            "frames requires --at or --every",
        ));
    }
    let mut passes: Vec<Vec<String>> = items
        .iter()
        .map(|item| {
            recipes::with_bin(
                recipes::frame_argv(input, &item.at, &item.path),
                &ctx.ffmpeg,
            )
        })
        .collect();
    if let Some(sheet) = sheet {
        let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
        passes.push(recipes::with_bin(
            recipes::contact_sheet_argv(&paths, sheet),
            &ctx.ffmpeg,
        ));
    }
    if ctx.no_overwrite {
        for item in &items {
            if std::path::Path::new(&item.path).exists() {
                return Err(Error::new(
                    "frames",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {}", item.path),
                ));
            }
        }
        if let Some(sheet) = sheet {
            if std::path::Path::new(sheet).exists() {
                return Err(Error::new(
                    "frames",
                    "output_exists",
                    format!("output exists and --no-overwrite was set: {sheet}"),
                ));
            }
        }
    }
    if !ctx.dry_run {
        std::fs::create_dir_all(output).map_err(|e| {
            Error::new(
                "frames",
                "ffmpeg_failed",
                format!("could not create {output}: {e}"),
            )
        })?;
        for cmd in &passes {
            run_ffmpeg(cmd, ctx.verbose, None).map_err(|e| Error::ffmpeg("frames", e))?;
        }
    }
    Ok(Outcome::Frames(FramesEnvelope {
        ok: true,
        op: "frames",
        input: input.clone(),
        output: output.clone(),
        frames: items,
        sheet: sheet.clone(),
        ffmpeg: passes.last().cloned().unwrap_or_default(),
        passes: Some(passes),
    }))
}

fn still_name(at: &str) -> String {
    format!("t-{}.jpg", at.replace(':', "-"))
}

fn every_stamps(every: f64, duration: f64) -> Vec<String> {
    let mut t = 0.0;
    let mut stamps = Vec::new();
    while t <= duration + 1e-9 {
        stamps.push(format_stamp(t));
        t += every;
        if every <= 0.0 {
            break;
        }
    }
    stamps
}

fn format_stamp(t: f64) -> String {
    if (t - t.round()).abs() < 1e-9 {
        format!("{}", t.round() as i64)
    } else {
        format!("{t}")
    }
}
