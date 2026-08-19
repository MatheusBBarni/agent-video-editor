mod common;

use common::ffmpeg_gate;

#[test]
fn ffmpeg_gate_continues_when_available() {
    assert!(ffmpeg_gate(true, false));
    assert!(ffmpeg_gate(true, true));
}

#[test]
fn ffmpeg_gate_skips_when_missing_and_not_required() {
    assert!(!ffmpeg_gate(false, false));
}

#[test]
#[should_panic(expected = "ffmpeg required")]
fn ffmpeg_gate_panics_when_missing_and_required() {
    let _ = ffmpeg_gate(false, true);
}

#[test]
fn ci_workflow_requires_ffmpeg_on_linux_and_macos() {
    let yml = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/ci.yml"
    ));
    assert!(yml.contains("AVE_REQUIRE_FFMPEG"), "CI must set AVE_REQUIRE_FFMPEG");
    assert!(yml.contains("ubuntu-latest"), "CI must run on Linux");
    assert!(yml.contains("macos-latest"), "CI must run on macOS");
    assert!(yml.contains("cargo test --workspace"), "CI must run cargo test --workspace");
    assert!(yml.contains("cargo fmt"), "CI must run rustfmt");
    assert!(yml.contains("clippy"), "CI must run clippy");
}
