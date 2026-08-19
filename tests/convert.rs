use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn convert_gif_dry_run_is_two_pass_and_leaves_no_palette() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["convert", "in.mp4", "-o", "out.gif", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "convert");
    assert!(
        stdout.contains("palettegen"),
        "gif convert must plan palettegen: {stdout}"
    );
    assert!(
        stdout.contains("paletteuse"),
        "gif convert must plan paletteuse: {stdout}"
    );

    let tokens = argv_tokens(&v);
    assert!(
        tokens.iter().all(|t| t != "palette.png"),
        "ffmpeg/passes must not use a bare palette.png: {tokens:?}"
    );
    let palette = tokens
        .iter()
        .find(|t| t.contains("ave-palette-") || t.ends_with("palette.png"))
        .expect("passes must include a palette path");
    let tmp = std::env::temp_dir();
    assert!(
        palette.contains("ave-palette-") || std::path::Path::new(palette).starts_with(&tmp),
        "palette path must be temp-shaped: {palette}"
    );

    let leftover: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        leftover,
        [std::ffi::OsString::from("in.mp4")],
        "dry-run must not write palette or output: {leftover:?}"
    );
}

#[test]
fn convert_gif_pass1_failure_leaves_user_palette() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("in.mp4"), b"placeholder").unwrap();
    fs::write(dir.path().join("palette.png"), b"user-palette").unwrap();

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["convert", "in.mp4", "-o", "out.gif"])
        .assert()
        .failure();

    assert_eq!(
        fs::read(dir.path().join("palette.png")).unwrap(),
        b"user-palette"
    );
    assert!(!dir.path().join("out.gif").exists());
    let extra: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| {
            let name = name.to_string_lossy();
            name.starts_with("palette") && name != "palette.png"
        })
        .collect();
    assert!(
        extra.is_empty(),
        "pass-1 failure left extra palette files: {extra:?}"
    );
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_fixture(path: &std::path::Path) {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=1",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn convert_gif_success_leaves_user_palette() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("clip.mp4"));
    fs::write(dir.path().join("palette.png"), b"user-palette").unwrap();

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["convert", "clip.mp4", "-o", "out.gif"])
        .assert()
        .success();

    assert!(dir.path().join("out.gif").exists());
    assert_eq!(
        fs::read(dir.path().join("palette.png")).unwrap(),
        b"user-palette"
    );
    let extra: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| {
            let name = name.to_string_lossy();
            name.starts_with("palette") && name != "palette.png"
        })
        .collect();
    assert!(
        extra.is_empty(),
        "successful convert left extra palette files: {extra:?}"
    );
}

fn argv_tokens(v: &Value) -> Vec<String> {
    let mut tokens = Vec::new();
    if let Some(ffmpeg) = v["ffmpeg"].as_array() {
        tokens.extend(ffmpeg.iter().filter_map(|x| x.as_str().map(str::to_string)));
    }
    if let Some(passes) = v["passes"].as_array() {
        for pass in passes {
            if let Some(argv) = pass.as_array() {
                tokens.extend(argv.iter().filter_map(|x| x.as_str().map(str::to_string)));
            }
        }
    }
    tokens
}
