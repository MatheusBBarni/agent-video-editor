use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn concat_dry_run_uses_concat_demuxer_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.mp4"), b"a").unwrap();
    fs::write(dir.path().join("b.mp4"), b"b").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["concat", "a.mp4", "b.mp4", "-o", "out.mp4", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "concat");
    let argv: Vec<&str> = v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        argv.windows(2).any(|w| w == ["-f", "concat"]),
        "expected concat demuxer in {argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w == ["-safe", "0"]),
        "expected -safe 0 in {argv:?}"
    );
    assert!(
        !argv.windows(2).any(|w| w == ["-c", "copy"]),
        "unprobeable concat must re-encode: {argv:?}"
    );
    assert!(argv.contains(&"libx264"), "expected libx264 in {argv:?}");
    assert!(!dir.path().join("out.mp4").exists());

    let leftover: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        leftover.len(),
        2,
        "dry-run must not leave concat list or output: {leftover:?}"
    );
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ffmpeg(args: &[&str]) {
    let status = std::process::Command::new("ffmpeg")
        .args(args)
        .output()
        .expect("spawn ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg fixture failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn write_fixture(path: &std::path::Path, size: &str) {
    let input = format!("testsrc=duration=1:size={size}:rate=30");
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        &input,
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
    ]);
}

#[test]
fn concat_mismatch_dry_run_reencodes() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("a.mp4"), "320x240");
    write_fixture(&dir.path().join("b.mp4"), "640x360");

    let argv = concat_dry_run_argv(&dir, "a.mp4", "b.mp4");
    assert!(
        !argv.windows(2).any(|w| w == ["-c", "copy"]),
        "mismatched concat must re-encode: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "libx264"),
        "expected libx264 in {argv:?}"
    );
}

fn write_rotated_fixture(path: &std::path::Path, size: &str) {
    let src = path.with_file_name(format!(
        "src-{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    write_fixture(&src, size);
    ffmpeg(&[
        "-y",
        "-display_rotation",
        "90",
        "-i",
        src.to_str().unwrap(),
        "-c",
        "copy",
        path.to_str().unwrap(),
    ]);
}

fn concat_dry_run_argv(dir: &tempfile::TempDir, a: &str, b: &str) -> Vec<String> {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["concat", a, b, "-o", "out.mp4", "--dry-run"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    v["ffmpeg"]
        .as_array()
        .expect("ffmpeg argv")
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn concat_mixed_rotation_dry_run_reencodes() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("unrotated.mp4"), "320x240");
    write_rotated_fixture(&dir.path().join("rotated.mp4"), "320x240");

    let argv = concat_dry_run_argv(&dir, "unrotated.mp4", "rotated.mp4");
    assert!(
        !argv.windows(2).any(|w| w == ["-c", "copy"]),
        "mixed rotation must re-encode: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "libx264"),
        "expected libx264 in {argv:?}"
    );
}

#[test]
fn concat_same_rotation_dry_run_still_copies() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_rotated_fixture(&dir.path().join("a.mp4"), "320x240");
    write_rotated_fixture(&dir.path().join("b.mp4"), "320x240");

    let argv = concat_dry_run_argv(&dir, "a.mp4", "b.mp4");
    assert!(
        argv.windows(2).any(|w| w == ["-c", "copy"]),
        "matching 90 degree clips must still copy: {argv:?}"
    );
}

#[test]
fn concat_matching_fixtures_dry_run_copies() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("a.mp4"), "320x240");
    write_fixture(&dir.path().join("b.mp4"), "320x240");

    let argv = concat_dry_run_argv(&dir, "a.mp4", "b.mp4");
    assert!(
        argv.windows(2).any(|w| w == ["-c", "copy"]),
        "matching probed clips must still copy: {argv:?}"
    );
}

#[test]
fn concat_unprobeable_copy_only_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.mp4"), b"a").unwrap();
    fs::write(dir.path().join("b.mp4"), b"b").unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "concat",
            "a.mp4",
            "b.mp4",
            "-o",
            "out.mp4",
            "--dry-run",
            "--copy-only",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "copy_only");
}

#[test]
fn concat_mid_gop_copy_trims_have_monotonic_dts() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_gop_fixture(&dir.path().join("src.mp4"));

    ave_ok(
        &dir,
        &[
            "trim",
            "src.mp4",
            "--from",
            "0.4",
            "--duration",
            "2",
            "-o",
            "a.mp4",
        ],
    );
    ave_ok(
        &dir,
        &[
            "trim",
            "src.mp4",
            "--from",
            "3.4",
            "--duration",
            "2",
            "-o",
            "b.mp4",
        ],
    );

    let log = dir.path().join("ffmpeg.err");
    let wrapper = write_ffmpeg_stderr_logger(dir.path(), &log);
    ave_ok(
        &dir,
        &[
            "--ffmpeg",
            wrapper.to_str().unwrap(),
            "concat",
            "a.mp4",
            "b.mp4",
            "-o",
            "out.mp4",
        ],
    );

    let stderr = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !stderr.contains("Non-monotonic DTS"),
        "copy concat must reset timestamps:\n{stderr}"
    );
    assert!(
        dir.path().join("out.mp4").exists(),
        "concat must write out.mp4"
    );

    let leftover: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        leftover.iter().all(|name| {
            !name.ends_with(".ts") && name != "concat-list.txt" && !name.starts_with("ave-concat-")
        }),
        "concat must not leave remux temps in cwd: {leftover:?}"
    );
}

#[test]
fn concat_matching_dry_run_shows_mpegts_remux() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("a.mp4"), "320x240");
    write_fixture(&dir.path().join("b.mp4"), "320x240");

    let v = ave_ok(
        &dir,
        &["concat", "a.mp4", "b.mp4", "-o", "out.mp4", "--dry-run"],
    );
    let passes = v["passes"]
        .as_array()
        .expect("copy concat remux must emit passes");
    assert_eq!(passes.len(), 3, "two remuxes + concat: {passes:?}");

    let remux: Vec<Vec<&str>> = passes[..2]
        .iter()
        .map(|p| {
            p.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect()
        })
        .collect();
    for argv in &remux {
        assert!(
            argv.windows(2).any(|w| w == ["-bsf:v", "h264_mp4toannexb"]),
            "remux pass must use annex-B: {argv:?}"
        );
        let ts = argv
            .last()
            .copied()
            .filter(|t| t.ends_with(".ts"))
            .expect("remux pass must write a .ts temp");
        assert!(
            std::path::Path::new(ts).starts_with(std::env::temp_dir()),
            "remux temp must be under temp_dir: {ts}"
        );
    }

    let ffmpeg: Vec<&str> = v["ffmpeg"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(
        ffmpeg.windows(2).any(|w| w == ["-bsf:a", "aac_adtstoasc"]),
        "final concat must apply aac_adtstoasc, not a bare -c copy: {ffmpeg:?}"
    );
    assert!(
        ffmpeg.windows(2).any(|w| w == ["-c", "copy"]),
        "final concat must still copy: {ffmpeg:?}"
    );
    assert!(!dir.path().join("out.mp4").exists());
    let leftover: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        leftover.len(),
        2,
        "dry-run must not write remux temps or output: {leftover:?}"
    );
}

#[test]
fn concat_matching_copy_only_still_succeeds() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg not on PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixture(&dir.path().join("a.mp4"), "320x240");
    write_fixture(&dir.path().join("b.mp4"), "320x240");

    let v = ave_ok(
        &dir,
        &[
            "concat",
            "a.mp4",
            "b.mp4",
            "-o",
            "out.mp4",
            "--dry-run",
            "--copy-only",
        ],
    );
    assert_eq!(v["op"], "concat");
}

#[test]
fn skill_tells_agents_not_to_remux_to_ts() {
    let root = env!("CARGO_MANIFEST_DIR");
    let skill = fs::read_to_string(format!("{root}/skills/ave/SKILL.md")).unwrap();
    let commands = fs::read_to_string(format!("{root}/skills/ave/references/commands.md")).unwrap();
    assert!(
        skill.contains("Do not remux clips to `.ts` yourself"),
        "skill must forbid agent MPEG-TS remux"
    );
    assert!(
        commands.contains("Do not remux clips to `.ts` yourself"),
        "commands.md must forbid agent MPEG-TS remux"
    );
}

fn write_gop_fixture(path: &std::path::Path) {
    ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=duration=10:size=320x240:rate=30",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=1000:duration=10",
        "-c:v",
        "libx264",
        "-g",
        "30",
        "-keyint_min",
        "30",
        "-sc_threshold",
        "0",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        path.to_str().unwrap(),
    ]);
}

fn write_ffmpeg_stderr_logger(dir: &std::path::Path, log: &std::path::Path) -> std::path::PathBuf {
    let wrapper = dir.join("ave-ffmpeg");
    fs::write(
        &wrapper,
        format!("#!/bin/sh\nexec ffmpeg \"$@\" 2>>\"{}\"\n", log.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&wrapper).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper, perms).unwrap();
    }
    wrapper
}

fn ave_ok(dir: &tempfile::TempDir, args: &[&str]) -> Value {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(args)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true, "{stdout}");
    v
}
