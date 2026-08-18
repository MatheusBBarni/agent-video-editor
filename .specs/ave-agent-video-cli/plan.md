# Requirements Document — `ave`

Approved 2026-04-10. Engine: spawn system `ffmpeg` / `ffprobe` (not `ffmpeg-next` / libav).

## Objective

Ship **`ave`**, a generic Rust CLI that lets agents edit video **without inventing ffmpeg flags**. It runs the video-edit skill recipes by spawning system `ffmpeg` / `ffprobe`, with typed args, JSON stdout, dry-run, safe file policy, and a **JSON command list** for multi-step edits.

Not a GUI. Not a `youtube-videos` workspace manager. Not libav bindings.

Guide: `../youtube-videos/.agents/skills/video-edit/SKILL.md` and `references/operations.md`.

## Stack

- Rust (edition 2021+)
- `clap` derive subcommands
- `serde` / `serde_json`
- Spawn `ffmpeg` / `ffprobe` (PATH, or `--ffmpeg` / `--ffprobe`)
- No auto-download
- Tests: `cargo test`; integration tests use real ffmpeg when present
- Official: **macOS + Linux**. Windows best-effort

## Public interface

```
ave [--ffmpeg PATH] [--ffprobe PATH] [--human] [--verbose] [--progress]
    [--dry-run] [--copy-only] [--no-overwrite]
    <COMMAND>
```

| Command | Required | Notes |
|---|---|---|
| `info <input>` | input | read-only |
| `doctor` | — | ffmpeg/ffprobe found? versions? |
| `trim <input> -o OUT` | `--from` and (`--to` or `--duration`) | |
| `concat <inputs...> -o OUT` | ≥2 inputs | we write the concat list (temp, cleaned up) |
| `resize <input> -o OUT` | `--preset` **or** `--width`+`--height` | |
| `speed <input> -o OUT` | `--factor` | chain `atempo` outside 0.5–2.0 |
| `extract-audio <input> -o OUT` | | codec from `--format` or `-o` ext |
| `replace-audio <input> -o OUT` | `--audio FILE` or `--mute` or `--mix FILE` | |
| `overlay <input> -o OUT` | `--image` | `--position` or x/y; optional opacity, time range |
| `compress <input> -o OUT` | | `--crf` 23, `--preset` medium |
| `convert <input> -o OUT` | | format from `-o` ext; `.gif` → two-pass |
| `run <plan.json\|->` | JSON object with `steps` | file or stdin |

**Resize presets:** `tiktok` 1080×1920, `youtube`/`twitter` 1920×1080, `instagram` 1080×1350, `square` 1080×1080. Scale + center pad. `--stretch` disables pad.

**Timestamps:** `HH:MM:SS`, `HH:MM:SS.mmm`, `MM:SS`, seconds.

## JSON command list (`ave run`)

Agents send the full plan in one shot. Same ops and params as the verbs. **Explicit paths only** — no `$prev`.

```json
{
  "steps": [
    {"op": "trim", "input": "in.mp4", "from": "00:00:00", "to": "00:00:12", "output": "a.mp4"},
    {"op": "trim", "input": "in.mp4", "from": "00:00:18", "to": "00:02:00", "output": "b.mp4"},
    {"op": "concat", "inputs": ["a.mp4", "b.mp4"], "output": "out.mp4"}
  ]
}
```

- `ave run plan.json` or `ave run -` (stdin)
- Paths: relative to **process cwd**, or absolute
- Sequential; **stop on first failure**; **keep** outputs already written
- Each step uses the same safety + encode rules
- A step may use an earlier step’s `output` as input; that path must exist when the step starts
- `--dry-run` validates the whole list and prints every argv; does not require outputs of earlier steps to exist yet
- Unknown `op` or missing required fields → fail before running anything (schema validation first)

Stdout for `run`: envelope plus `steps[]` (per-step result). On failure: `failed_step` index, that step’s `error`, `written` outputs so far.

## Safety

- Mutating commands **require** `-o` / `output`
- **Refuse in-place**
- **Never delete or overwrite inputs**
- Existing **output** overwritten (`-y`) unless `--no-overwrite`
- `--dry-run`: validate + print, write nothing
- `--copy-only`: fail if a step would re-encode

## Encode policy

- `trim` / `concat`: `-c copy` by default
- `trim --accurate` / `"accurate": true`: frame-accurate re-encode
- `concat`: probe; match → copy; mismatch → re-encode
- Must re-encode: `resize`, `speed`, `overlay`, `compress`, GIF `convert`
- Re-encode defaults: `libx264`, `yuv420p`, CRF 23, `medium`, `aac` when audio must change, `+faststart` on mp4
- No hidden YouTube-master extras

## Execution model

Rust parses CLI or JSON, builds the **exact ffmpeg/ffprobe argv** from the video-edit skill, **spawns** the binary, waits, probes the output, prints JSON. Does not link libav.

## Stdout / stderr

JSON stdout by default. `--human` for text. ffmpeg banners never on stdout.

Success: `ok`, `op`, `output`, `duration_s`, `width`, `height`, `size_bytes`, `ffmpeg` (argv). `run` adds `steps`. Failure: `ok: false`, `error`, `message`, non-zero exit.

- Default stderr: quiet
- `--verbose`: ffmpeg logs on stderr
- `--progress`: JSONL on stderr

## Edge cases

- Missing ffmpeg/ffprobe → error, non-zero
- Missing input → error, no output written
- `concat` with <2 files → error
- `speed` factor ≤ 0 → error
- `atempo` chained outside 0.5–2.0
- GIF two-pass; no leftover `palette.png`
- Concat list file is temp and always removed
- Invalid plan JSON / unknown op → fail before any step
- Failed step: later steps skipped; earlier outputs remain

## Out of scope (v1)

Color grade, captions, multi-cam, loudness normalize, target-filesize bitrate, implicit `$prev` wiring, workspace (`inbox`/`work`/`exports`), GUI/TUI, auto-download ffmpeg, Windows as a supported platform.

## Constraints

- Do not invent ffmpeg flags; recipes from the video-edit skill
- Prefer copy; re-encode only when the skill says so
- One binary; verb CLI and JSON plan share the same op implementations
