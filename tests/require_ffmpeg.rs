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
