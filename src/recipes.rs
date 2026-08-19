pub fn with_bin(mut argv: Vec<String>, bin: &str) -> Vec<String> {
    if let Some(first) = argv.first_mut() {
        *first = bin.to_string();
    }
    argv
}

pub fn trim_argv(
    from: &str,
    end_flag: &str,
    end_val: &str,
    input: &str,
    output: &str,
    accurate: bool,
    has_audio: bool,
) -> Vec<String> {
    if accurate {
        let mut ffmpeg = vec![
            "ffmpeg".into(),
            "-y".into(),
            "-accurate_seek".into(),
            "-ss".into(),
            from.into(),
            end_flag.into(),
            end_val.into(),
            "-i".into(),
            input.into(),
        ];
        ffmpeg.extend(reencode_video_args(has_audio));
        ffmpeg.push(output.into());
        ffmpeg
    } else {
        vec![
            "ffmpeg".into(),
            "-y".into(),
            "-ss".into(),
            from.into(),
            end_flag.into(),
            end_val.into(),
            "-i".into(),
            input.into(),
            "-c".into(),
            "copy".into(),
            output.into(),
        ]
    }
}

pub fn mpegts_video_bsf(codec: &str) -> Option<&'static str> {
    match codec {
        "h264" => Some("h264_mp4toannexb"),
        "hevc" | "h265" => Some("hevc_mp4toannexb"),
        _ => None,
    }
}

pub fn concat_copy_to_mpegts_argv(input: &str, ts_out: &str, video_bsf: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-c".into(),
        "copy".into(),
        "-bsf:v".into(),
        video_bsf.into(),
        ts_out.into(),
    ]
}

pub fn concat_from_mpegts_argv(
    list_path: &str,
    output: &str,
    audio_bsf: Option<&str>,
) -> Vec<String> {
    let mut ffmpeg = concat_demuxer_prefix(list_path);
    ffmpeg.extend(["-c".into(), "copy".into()]);
    if let Some(bsf) = audio_bsf {
        ffmpeg.extend(["-bsf:a".into(), bsf.into()]);
    }
    ffmpeg.push(output.into());
    ffmpeg
}

pub fn concat_argv(list_path: &str, output: &str) -> Vec<String> {
    let mut ffmpeg = concat_demuxer_prefix(list_path);
    ffmpeg.extend(reencode_video_args(true));
    ffmpeg.push(output.into());
    ffmpeg
}

fn concat_demuxer_prefix(list_path: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        list_path.into(),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Pad,
    Crop,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPos {
    LowerThird,
    Center,
    Top,
}

impl TextPos {
    pub fn xy(self) -> (&'static str, &'static str) {
        match self {
            Self::LowerThird => ("(w-text_w)/2", "h-th-80"),
            Self::Center => ("(w-text_w)/2", "(h-text_h)/2"),
            Self::Top => ("(w-text_w)/2", "80"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotateDeg {
    D90,
    D180,
    D270,
}

impl RotateDeg {
    pub fn vf(self) -> &'static str {
        match self {
            Self::D90 => "transpose=clock",
            Self::D180 => "transpose=clock,transpose=clock",
            Self::D270 => "transpose=cclock",
        }
    }
}

pub fn vf_reencode_argv(input: &str, output: &str, vf: &str, has_audio: bool) -> Vec<String> {
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
    ]);
    push_audio_copy(&mut ffmpeg, has_audio);
    ffmpeg.push(output.into());
    ffmpeg
}

pub fn scale_pad(w: u32, h: u32) -> String {
    format!(
        "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
    )
}

pub fn scale_crop(w: u32, h: u32) -> String {
    format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}")
}

pub fn scale_stretch(w: u32, h: u32) -> String {
    format!("scale={w}:{h}")
}

pub fn resize_filter(w: u32, h: u32, fit: Fit) -> String {
    match fit {
        Fit::Pad => scale_pad(w, h),
        Fit::Crop => scale_crop(w, h),
        Fit::Stretch => scale_stretch(w, h),
    }
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

pub fn speed_argv(input: &str, output: &str, factor: f64, has_audio: bool) -> Vec<String> {
    let mut ffmpeg = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-filter:v".into(),
        format!("setpts={}*PTS", 1.0 / factor),
    ];
    if has_audio {
        ffmpeg.extend(["-filter:a".into(), atempo_filter(factor)]);
    }
    ffmpeg.push(output.into());
    ffmpeg
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

pub fn overlay_argv(
    input: &str,
    image: &str,
    output: &str,
    expr: &str,
    has_audio: bool,
) -> Vec<String> {
    let mut ffmpeg = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-i".into(),
        image.into(),
        "-filter_complex".into(),
        expr.into(),
    ];
    push_audio_copy(&mut ffmpeg, has_audio);
    ffmpeg.push(output.into());
    ffmpeg
}

pub fn compress_argv(
    input: &str,
    output: &str,
    crf: u8,
    preset: &str,
    has_audio: bool,
) -> Vec<String> {
    let mut ffmpeg = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-crf".into(),
        crf.to_string(),
        "-preset".into(),
        preset.into(),
    ];
    push_audio_copy(&mut ffmpeg, has_audio);
    ffmpeg.push(output.into());
    ffmpeg
}

pub fn gif_passes(input: &str, output: &str, palette: &str) -> (Vec<String>, Vec<String>) {
    let pass1 = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-vf".into(),
        "fps=15,scale=480:-1:flags=lanczos,palettegen".into(),
        palette.into(),
    ];
    let pass2 = vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-i".into(),
        palette.into(),
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

pub fn volume_argv(input: &str, output: &str, db: f64) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-i".into(),
        input.into(),
        "-filter:a".into(),
        format!("volume={db}dB"),
        "-c:v".into(),
        "copy".into(),
        output.into(),
    ]
}

pub fn fade_vf(fade_in: Option<f64>, fade_out: Option<(f64, f64)>) -> String {
    let mut parts = Vec::new();
    if let Some(d) = fade_in {
        parts.push(format!("fade=t=in:st=0:d={d}"));
    }
    if let Some((d, start)) = fade_out {
        parts.push(format!("fade=t=out:st={start}:d={d}"));
    }
    parts.join(",")
}

pub fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace(':', "\\:")
}

pub fn text_vf(text: &str, position: TextPos, span: Option<&(String, String)>) -> String {
    let (x, y) = position.xy();
    let mut vf = format!(
        "drawtext=text='{}':fontsize=48:fontcolor=white:x={x}:y={y}",
        escape_drawtext(text)
    );
    if let Some((from, to)) = span {
        vf.push_str(&format!(":enable='between(t,{from},{to})'"));
    }
    vf
}

pub fn captions_vf(srt: &str) -> String {
    let escaped = srt.replace('\\', "\\\\").replace(':', "\\:");
    format!("subtitles={escaped}")
}

pub fn contact_sheet_argv(inputs: &[&str], output: &str) -> Vec<String> {
    let mut argv = vec!["ffmpeg".into(), "-y".into()];
    for input in inputs {
        argv.extend(["-i".into(), (*input).into()]);
    }
    let n = inputs.len().max(1);
    let cols = ((n as f64).sqrt().ceil() as usize).max(1);
    let rows = n.div_ceil(cols);
    argv.extend([
        "-filter_complex".into(),
        format!("tile={cols}x{rows}"),
        output.into(),
    ]);
    argv
}

pub fn frame_argv(input: &str, at: &str, output: &str) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-y".into(),
        "-ss".into(),
        at.into(),
        "-i".into(),
        input.into(),
        "-frames:v".into(),
        "1".into(),
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

fn push_audio_copy(argv: &mut Vec<String>, has_audio: bool) {
    if has_audio {
        argv.extend(["-c:a".into(), "copy".into()]);
    }
}

fn reencode_video_args(has_audio: bool) -> Vec<String> {
    let mut args = vec![
        "-c:v".into(),
        "libx264".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-crf".into(),
        "23".into(),
        "-preset".into(),
        "medium".into(),
    ];
    if has_audio {
        args.extend(["-c:a".into(), "aac".into()]);
    }
    args.extend(["-movflags".into(), "+faststart".into()]);
    args
}
