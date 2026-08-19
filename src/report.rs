use crate::error::{
    DetectEnvelope, DoctorEnvelope, Envelope, FramesEnvelope, InfoEnvelope, RunEnvelope, print_json,
};
use crate::exec::Outcome;
use serde::Serialize;

pub fn print_outcome(human: bool, outcome: Outcome) {
    match outcome {
        Outcome::Edit(env) => emit(human, &env, human_edit(&env)),
        Outcome::Info(env) => emit(human, &env, human_info(&env)),
        Outcome::Frames(env) => emit(human, &env, human_frames(&env)),
        Outcome::Detect(env) => emit(human, &env, human_detect(&env)),
        Outcome::Doctor(env) => {
            let ok = env.ok;
            emit(human, &env, human_doctor(&env));
            if !ok {
                std::process::exit(1);
            }
        }
    }
}

pub fn print_run(human: bool, env: &RunEnvelope) {
    emit(human, env, human_run(env));
}

fn emit(human: bool, json: &impl Serialize, text: String) {
    if human {
        print!("{text}");
        if !text.ends_with('\n') {
            println!();
        }
    } else {
        print_json(json);
    }
}

fn human_info(env: &InfoEnvelope) -> String {
    format!(
        "ok {}\nduration: {}s\n{}x{}\nvideo: {}\naudio: {}\n",
        env.op, env.duration_s, env.width, env.height, env.video_codec, env.audio_codec
    )
}

fn human_edit(env: &Envelope) -> String {
    format!(
        "ok {}\noutput: {}\nduration: {}s\n{}x{}\n",
        env.op, env.output, env.duration_s, env.width, env.height
    )
}

fn human_frames(env: &FramesEnvelope) -> String {
    format!(
        "ok {}\noutput: {}\nframes: {}\n",
        env.op,
        env.output,
        env.frames.len()
    )
}

fn human_detect(env: &DetectEnvelope) -> String {
    format!(
        "ok {}\nkind: {}\nsegments: {}\n",
        env.op,
        env.kind,
        env.segments.len()
    )
}

fn human_doctor(env: &DoctorEnvelope) -> String {
    format!(
        "ok {}\nffmpeg: {}\nffprobe: {}\n",
        env.op, env.ffmpeg_version, env.ffprobe_version
    )
}

fn human_run(env: &RunEnvelope) -> String {
    let mut out = format!("ok {}\nsteps: {}\n", env.ok, env.steps.len());
    if let Some(error) = env.error {
        out.push_str(&format!("error: {error}\n"));
    }
    if let Some(message) = &env.message {
        out.push_str(&format!("message: {message}\n"));
    }
    out
}
