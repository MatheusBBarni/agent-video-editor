mod common;

use assert_cmd::Command;
use common::{require_ffmpeg, write_clip};
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

    let passes = v["passes"]
        .as_array()
        .expect("gif convert must emit passes");
    assert_eq!(passes.len(), 2, "gif convert is two-pass: {passes:?}");
    let pass1 = str_tokens(&passes[0]);
    let pass2 = str_tokens(&passes[1]);
    assert!(
        pass1.iter().any(|t| t.contains("palettegen")),
        "pass 1 must plan palettegen: {pass1:?}"
    );
    assert!(
        pass2.iter().any(|t| t.contains("paletteuse")),
        "pass 2 must plan paletteuse: {pass2:?}"
    );

    let palettes: Vec<&str> = pass1
        .iter()
        .chain(pass2.iter())
        .copied()
        .filter(|t| t.contains("ave-palette-"))
        .collect();
    assert_eq!(
        palettes.len(),
        2,
        "both passes must name the temp palette: pass1={pass1:?} pass2={pass2:?}"
    );
    assert_eq!(
        palettes[0], palettes[1],
        "both passes must share one palette"
    );
    let palette = palettes[0];
    assert_ne!(palette, "palette.png");
    assert!(
        std::path::Path::new(palette).starts_with(std::env::temp_dir()),
        "palette path must be under temp_dir: {palette}"
    );
    assert!(
        str_tokens(&v["ffmpeg"])
            .into_iter()
            .chain(pass1)
            .chain(pass2)
            .all(|t| t != "palette.png"),
        "ffmpeg/passes must not use a bare palette.png"
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
    let extra = extra_palette_files(dir.path());
    assert!(
        extra.is_empty(),
        "pass-1 failure left extra palette files: {extra:?}"
    );
}

#[test]
fn convert_gif_success_leaves_user_palette() {
    if !require_ffmpeg() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_clip(&dir.path().join("clip.mp4"));
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
    let extra = extra_palette_files(dir.path());
    assert!(
        extra.is_empty(),
        "successful convert left extra palette files: {extra:?}"
    );
}

fn str_tokens(v: &Value) -> Vec<&str> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default()
}

fn extra_palette_files(dir: &std::path::Path) -> Vec<std::ffi::OsString> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| {
            let name = name.to_string_lossy();
            name.starts_with("palette") && name != "palette.png"
        })
        .collect()
}
