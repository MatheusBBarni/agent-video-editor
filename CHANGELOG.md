# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ave schema` prints the `run` plan JSON Schema.
- `ave run --workdir DIR` keeps relative plan outputs out of cwd.
- `install-skill --provider` adds `codex`, `continue`, `windsurf`, and `copilot`.
- `resize --width` / `--height` xor `--preset`, plus `--stretch` to skip letterbox pad.
- Overlay `--x` / `--y`, `--opacity`, and `--from` / `--to`.
- Global `--human`, `--verbose`, and `--progress`.
- `ave detect` for silence, black frames, and scene cuts.
- `ave --version` prints the package version.
- GitHub Actions CI: rustfmt, clippy, and `cargo test` on Linux and macOS with ffmpeg.

### Changed

- Video re-encodes share one `libx264` / `yuv420p` / CRF 23 / `medium` recipe, with `+faststart` on `.mp4`.

## [0.1.0] - 2026-08-18

### Added

- Initial `ave` CLI: typed ffmpeg recipes with JSON on stdout.

[Unreleased]: https://github.com/MatheusBBarni/agent-video-editor/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/MatheusBBarni/agent-video-editor/releases/tag/v0.1.0
