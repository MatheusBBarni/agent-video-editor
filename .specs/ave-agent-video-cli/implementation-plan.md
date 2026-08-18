# TDD Plan — `ave`

Approved 2026-04-10. Canonical requirements: `.specs/ave-agent-video-cli/plan.md`.

## Public interface

Agents only call the **`ave` binary**. That is the seam.

- Global: `--ffmpeg`, `--ffprobe`, `--human`, `--verbose`, `--progress`, `--dry-run`, `--copy-only`, `--no-overwrite`
- Verbs: `info`, `doctor`, `trim`, `concat`, `resize`, `speed`, `extract-audio`, `replace-audio`, `overlay`, `compress`, `convert`, `run`
- JSON envelope on stdout (`ok`, `op`, `output`, `duration_s`, `width`, `height`, `size_bytes`, `ffmpeg` argv; `run` adds `steps`)

Library modules stay private. Verb CLI and `run` share one op runner; tests do not import it.

`--dry-run` makes the skill recipes observable (the `ffmpeg` argv) without encoding. Real ffmpeg is only for slices that must write or probe a file.

## Seams to test

| Seam | How | Why |
|---|---|---|
| **CLI** (`ave`) | `assert_cmd` (or equivalent): exit code, stdout JSON, files on disk | What agents see |
| **ffmpeg spawn** | Real binary when the slice writes/probes; skip those tests if `ffmpeg`/`ffprobe` missing | Honest integration |

Do **not** test private argv builders or mock ffmpeg for recipe tests — dry-run stdout is the contract.

## Behaviors to test (in order)

**Tracer**

1. `ave trim <in> --from 30 --to 105 -o out.mp4 --dry-run` → exit 0, JSON `ok`, `op=trim`, argv matches skill copy recipe (`-y -ss 30 -to 105 -i … -c copy out`), **no output file**

**Safety + envelope**

2. Mutating trim without `-o` → non-zero, `ok: false`, nothing written
3. In-place trim (`-o` same as input) → refuse, input unchanged
4. Missing input → non-zero, no output
5. `--no-overwrite` when output exists → fail, existing output unchanged

**Doctor / info** (needs ffmpeg)

6. `ave doctor` → JSON with ffmpeg/ffprobe present and version strings
7. `ave info <video>` → duration / width / height from a real probe (tiny generated fixture)

**Encode policy on trim/concat**

8. `trim --accurate --dry-run` → re-encode argv (`libx264` / `aac`), not `-c copy`
9. `concat a b -o out --dry-run` → concat demuxer argv (`-f concat -safe 0`)
10. `concat` mismatched inputs actually re-encodes (or `--copy-only` fails on mismatch)
11. `--copy-only` on `resize --dry-run` → fail (must re-encode)

**Rest of the skill (dry-run argv first)**

12. `resize --preset tiktok` → scale+pad 1080×1920
13. `speed --factor 4` → `setpts=0.25*PTS` and chained `atempo=2.0,atempo=2.0`
14. `extract-audio` / `replace-audio --mute` / `overlay` / `compress` / `convert` to `.gif` (two-pass, no leftover palette)

**`ave run`**

15. `ave run plan.json --dry-run` → per-step argv; later steps may name earlier outputs that do not exist yet
16. `ave run -` reads stdin
17. Invalid JSON / unknown `op` → fail **before** any step
18. Step 2 fails → step 3 skipped, step 1 output **kept**, `failed_step` set

**One real write**

19. `ave trim` (no dry-run) writes a playable shorter file; `info` on it matches the cut

## Out of scope for this cycle

- `--human`, `--verbose`, `--progress` details
- Target-filesize compress, `$prev`, workspace layout
- Windows
- Auto-download ffmpeg
- Every preset / overlay position (one preset + one overlay position is enough)

## Deep module

`Op` + `run_plan(steps)` behind a thin clap front. JSON and flags deserialize to the same `Op`. Tests never see that split.
