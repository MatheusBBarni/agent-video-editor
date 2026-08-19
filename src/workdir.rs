use crate::op::Op;
use std::collections::HashSet;
use std::path::Path;

pub fn apply(ops: &mut [Op], dir: &str) {
    let root = Path::new(dir);
    let mut produced = HashSet::new();
    for op in ops {
        rewrite_op(op, root, &mut produced);
    }
}

fn rewrite_op(op: &mut Op, root: &Path, produced: &mut HashSet<String>) {
    match op {
        Op::Trim { input, output, .. }
        | Op::CutOut { input, output, .. }
        | Op::Keep { input, output, .. }
        | Op::Resize { input, output, .. }
        | Op::Speed { input, output, .. }
        | Op::ExtractAudio { input, output, .. }
        | Op::Compress { input, output, .. }
        | Op::Convert { input, output, .. }
        | Op::Frame { input, output, .. }
        | Op::Rotate { input, output, .. }
        | Op::Volume { input, output, .. }
        | Op::Fade { input, output, .. }
        | Op::Text { input, output, .. }
        | Op::Crop { input, output, .. } => rewrite_io(vec![input], output, root, produced),
        Op::Concat { inputs, output } => {
            let refs: Vec<&mut String> = inputs.iter_mut().collect();
            rewrite_listed(refs, output, root, produced);
        }
        Op::ReplaceAudio {
            input,
            output,
            audio,
            mix,
            ..
        } => {
            let mut extras = vec![input];
            if let Some(audio) = audio {
                extras.push(audio);
            }
            if let Some(mix) = mix {
                extras.push(mix);
            }
            rewrite_listed(extras, output, root, produced);
        }
        Op::Overlay {
            input,
            image,
            output,
            ..
        } => rewrite_io(vec![input, image], output, root, produced),
        Op::Captions {
            input, srt, output, ..
        } => rewrite_io(vec![input, srt], output, root, produced),
        Op::Frames {
            input,
            output,
            sheet,
            ..
        } => {
            rewrite_io(vec![input], output, root, produced);
            if let Some(sheet) = sheet {
                let orig = sheet.clone();
                *sheet = resolve(sheet, true, root, produced);
                produced.insert(orig);
            }
        }
        Op::Info { input } | Op::Detect { input, .. } => {
            *input = resolve(input, false, root, produced);
        }
        Op::Doctor => {}
    }
}

fn rewrite_io(
    inputs: Vec<&mut String>,
    output: &mut String,
    root: &Path,
    produced: &mut HashSet<String>,
) {
    rewrite_listed(inputs, output, root, produced);
}

fn rewrite_listed(
    inputs: Vec<&mut String>,
    output: &mut String,
    root: &Path,
    produced: &mut HashSet<String>,
) {
    for input in inputs {
        *input = resolve(input, false, root, produced);
    }
    let orig = output.clone();
    *output = resolve(output, true, root, produced);
    produced.insert(orig);
}

fn resolve(path: &str, is_output: bool, root: &Path, produced: &HashSet<String>) -> String {
    if Path::new(path).is_absolute() {
        return path.to_string();
    }
    if is_output || produced.contains(path) {
        root.join(path).to_string_lossy().into_owned()
    } else {
        path.to_string()
    }
}
