# ave

A small CLI that runs ffmpeg for you. Agents kept pasting broken filter graphs, so this wraps the usual edits (trim, concat, resize, speed, audio, overlay, compress, convert) as typed commands and prints JSON.

It shells out to whatever `ffmpeg` / `ffprobe` is on your PATH. It does not link libav.

## Install

You need Rust and ffmpeg. On a Mac:

```bash
brew install ffmpeg
cargo install --path .
ave doctor
```

`doctor` should print `"ok": true`. If it doesn't, ffmpeg isn't on PATH. Pass `--ffmpeg` / `--ffprobe` or fix the install.

The binary lands in `~/.cargo/bin/ave`.

## Usage

Always give an output path. ave will not overwrite the input.

```bash
ave info clip.mp4
ave trim clip.mp4 --from 30 --to 105 -o keep.mp4 --dry-run
ave trim clip.mp4 --from 30 --to 105 -o keep.mp4
ave resize keep.mp4 --preset tiktok -o short.mp4
```

`--dry-run` prints the ffmpeg argv and writes nothing. That's the one I use when I'm not sure the cut is right.

Several steps in one go:

```bash
ave run plan.json
```

```json
{
  "steps": [
    {"op": "trim", "input": "in.mp4", "from": "0", "to": "12", "output": "a.mp4"},
    {"op": "trim", "input": "in.mp4", "from": "18", "to": "120", "output": "b.mp4"},
    {"op": "concat", "inputs": ["a.mp4", "b.mp4"], "output": "out.mp4"}
  ]
}
```

Paths are relative to the current directory. Later steps can use earlier outputs. If a step fails, ave stops and leaves whatever it already wrote.

## Commands

| Command | What it does |
|---|---|
| `info` | Probe duration, size, codecs |
| `doctor` | Check ffmpeg / ffprobe |
| `trim` | Cut a range. `--accurate` re-encodes for frame-accurate in/out |
| `concat` | Join clips. Stream-copies when they match; re-encodes when they don't |
| `resize` | `--preset tiktok`, `youtube`, `twitter`, `instagram`, or `square`. Pads, does not stretch |
| `speed` | `--factor 2` is twice as fast, `0.5` is half |
| `extract-audio` | Pull audio. Format from `--format` or the output extension |
| `replace-audio` | `--mute`, `--audio FILE`, or `--mix FILE` |
| `overlay` | Logo/image. `--position top-right` (default), `top-left`, `bottom-left`, `bottom-right`, `center` |
| `compress` | CRF 23, preset medium unless you pass `--crf` / `--preset` |
| `convert` | Container from the output extension. `.gif` is a two-pass palette |
| `run` | JSON plan, or `-` for stdin |
| `install-skill` | Copy the agent skill into a provider folder |

Timestamps accept `HH:MM:SS`, `MM:SS`, or seconds.

Global flags: `--dry-run`, `--copy-only` (fail instead of re-encoding), `--no-overwrite`, `--ffmpeg PATH`, `--ffprobe PATH`.

Stdout is JSON. ffmpeg banners go to the captured process, not stdout. On failure you get `"ok": false`, an `error` code, and a non-zero exit.

## Agent skill

There is a skills.sh-style skill in `skills/ave/`. Install it into the agent folders you actually use:

```bash
ave install-skill --provider claude
ave install-skill --provider claude,pi,agents
ave install-skill --provider all --global
```

Providers: `agents`, `claude`, `pi`, `cursor`, or `all`. `--global` writes under `~/`. `--dir` copies to a custom folder and ignores providers.

If you run `ave install-skill` with no `--provider` and no `--dir`, it prints the list and exits 1.

## Safety

Mutating commands require `-o`. Same path as an input is rejected. Inputs are never deleted. Existing outputs are overwritten unless you pass `--no-overwrite`.

Trim and concat copy streams when they can. Resize, speed, overlay, compress, and GIF convert always re-encode.

This will not color grade, burn captions, cut a multi-cam show, or upload to YouTube.
