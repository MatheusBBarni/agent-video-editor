pub fn with_bin(mut argv: Vec<String>, bin: &str) -> Vec<String> {
    if let Some(first) = argv.first_mut() {
        *first = bin.to_string();
    }
    argv
}

pub fn trim_argv(from: &str, to: &str, input: &str, output: &str, accurate: bool) -> Vec<String> {
    if accurate {
        vec![
            "ffmpeg".into(),
            "-y".into(),
            "-accurate_seek".into(),
            "-ss".into(),
            from.into(),
            "-to".into(),
            to.into(),
            "-i".into(),
            input.into(),
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
            output.into(),
        ]
    } else {
        vec![
            "ffmpeg".into(),
            "-y".into(),
            "-ss".into(),
            from.into(),
            "-to".into(),
            to.into(),
            "-i".into(),
            input.into(),
            "-c".into(),
            "copy".into(),
            output.into(),
        ]
    }
}

pub fn concat_argv(list_path: &str, output: &str, copy: bool) -> Vec<String> {
    let mut ffmpeg = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        list_path.into(),
    ];
    if copy {
        ffmpeg.extend(["-c".into(), "copy".into()]);
    } else {
        ffmpeg.extend(reencode_video_args());
    }
    ffmpeg.push(output.into());
    ffmpeg
}

pub fn resize_argv(input: &str, output: &str, vf: &str) -> Vec<String> {
    let mut ffmpeg = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-vf".into(),
        vf.into(),
    ];
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
        "copy".into(),
    ]);
    ffmpeg.push(output.into());
    ffmpeg
}

pub fn scale_pad(w: u32, h: u32) -> String {
    format!(
        "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
    )
}

pub fn preset_size(preset: &str) -> Option<(u32, u32)> {
    match preset {
        "tiktok" => Some((1080, 1920)),
        "youtube" | "twitter" => Some((1920, 1080)),
        "instagram" => Some((1080, 1350)),
        "square" => Some((1080, 1080)),
        _ => None,
    }
}

pub fn atempo_filter(mut factor: f64) -> String {
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

pub fn speed_argv(input: &str, output: &str, factor: f64) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-filter:v".into(),
        format!("setpts={}*PTS", 1.0 / factor),
        "-filter:a".into(),
        atempo_filter(factor),
        output.into(),
    ]
}

pub fn audio_codec(ext: &str) -> Option<&'static str> {
    match ext {
        "mp3" => Some("libmp3lame"),
        "wav" => Some("pcm_s16le"),
        "aac" => Some("aac"),
        "flac" => Some("flac"),
        "copy" => Some("copy"),
        _ => None,
    }
}

pub fn extract_audio_argv(input: &str, output: &str, codec: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-vn".into(),
        "-acodec".into(),
        codec.into(),
        output.into(),
    ]
}

pub fn mute_argv(input: &str, output: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-c:v".into(),
        "copy".into(),
        "-an".into(),
        output.into(),
    ]
}

pub fn overlay_expr(position: &str) -> Option<&'static str> {
    Some(match position {
        "top-left" => "overlay=10:10",
        "top-right" => "overlay=W-w-10:10",
        "bottom-left" => "overlay=10:H-h-10",
        "bottom-right" => "overlay=W-w-10:H-h-10",
        "center" => "overlay=(W-w)/2:(H-h)/2",
        _ => return None,
    })
}

pub fn overlay_argv(input: &str, image: &str, output: &str, expr: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-i".into(),
        image.into(),
        "-filter_complex".into(),
        expr.into(),
        "-c:a".into(),
        "copy".into(),
        output.into(),
    ]
}

pub fn compress_argv(input: &str, output: &str, crf: u8, preset: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-crf".into(),
        crf.to_string(),
        "-preset".into(),
        preset.into(),
        "-c:a".into(),
        "copy".into(),
        output.into(),
    ]
}

pub fn gif_passes(input: &str, output: &str) -> (Vec<String>, Vec<String>) {
    let pass1 = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-vf".into(),
        "fps=15,scale=480:-1:flags=lanczos,palettegen".into(),
        "palette.png".into(),
    ];
    let pass2 = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-i".into(),
        "palette.png".into(),
        "-filter_complex".into(),
        "fps=15,scale=480:-1:flags=lanczos[x];[x][1:v]paletteuse".into(),
        output.into(),
    ];
    (pass1, pass2)
}

pub fn replace_audio_argv(input: &str, audio: &str, output: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-i".into(),
        audio.into(),
        "-c:v".into(),
        "copy".into(),
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "1:a:0".into(),
        "-shortest".into(),
        output.into(),
    ]
}

pub fn mix_audio_argv(input: &str, audio: &str, output: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-i".into(),
        audio.into(),
        "-filter_complex".into(),
        "[0:a][1:a]amix=inputs=2:duration=first[a]".into(),
        "-map".into(),
        "0:v".into(),
        "-map".into(),
        "[a]".into(),
        "-c:v".into(),
        "copy".into(),
        output.into(),
    ]
}

pub fn convert_argv(input: &str, output: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        output.into(),
    ]
}

fn reencode_video_args() -> [String; 12] {
    [
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
    ]
}
