---
name: ave
description: >
  Edit videos with the ave CLI (ffmpeg recipes, JSON stdout). Use when trimming,
  cutting, concatenating, resizing for TikTok/YouTube/Instagram, changing speed,
  extracting or replacing audio, overlaying a logo, compressing, converting
  (including GIF), burning captions, drawing a title, fading, changing volume,
  rotating, cropping edge strips (taskbar / browser chrome), detecting silence/black/scenes,
  grabbing a still, probing video info, or running a multi-step edit plan.
  Prefer ave over raw ffmpeg. Do not invent ffmpeg flags.
---

# ave

Local video edits via **`ave`**. It spawns system `ffmpeg`/`ffprobe`. Typed args, JSON on stdout, `--dry-run`.

Do not write ffmpeg commands. Do not invent flags. If `ave` is missing, install it (below) or tell the user.

## Setup

Need `ffmpeg` and `ffprobe` on PATH (`brew install ffmpeg` / `apt install ffmpeg`).

```bash
# from this repo
cargo install --path .
ave doctor
ave install-skill --provider claude,agents   # files in claude; agents is a symlink
ave install-skill --provider all --global
```

`doctor` must return `"ok": true`. If not, fix PATH or pass `--ffmpeg` / `--ffprobe`.

## Loop

1. Read this skill.
2. Prefer the file the user named. Else the newest video in cwd.
3. `ave info <file>` — recap duration, coded size, codecs, fps, audio, rotation, display size.
4. Vague ask → ask what to change. Concrete ask → plan, then run.
5. Do not guess cut points. Call `ave detect --kind silence|black|scenes` first, then `cut-out` / `keep --ranges` / `trim`. One hole → `cut-out`. User listed N cuts → `keep --ranges`, not N trims + concat. Hide a taskbar or browser chrome → `crop --bottom N` (or `--top` / `--left` / `--right`), not a free-form `crop=W:H:X:Y`. Then `resize` if you need a preset. Several other verbs → one `ave run` plan. Review many timestamps → `frames --at` / `--every`, not an `fps=` dump.
6. `--dry-run` first when unsure. Then run for real.
7. Reply with output path, duration, resolution, size from the JSON.

## Safety

- Mutating commands **require** `-o` / `"output"`.
- Never edit in place (output must not be an input).
- Never delete inputs. Existing **outputs** are overwritten unless `--no-overwrite`.
- `--dry-run` writes nothing. `--copy-only` fails if a re-encode would happen.
- Paths are cwd-relative or absolute.
- Always pass flags the CLI documents. Unknown flags and clap usage errors are JSON on stdout (`error: "usage"`). `--help` stays human.

## Commands

```text
ave [--dry-run] [--copy-only] [--no-overwrite] [--ffmpeg PATH] [--ffprobe PATH] <CMD>
```

| Cmd | Args | Notes |
|---|---|---|
| `info` | `<in>` | duration, coded `width`/`height`, `video_codec`, `audio_codec`, `fps`, `has_video`, `has_audio`, `rotate_deg`, `display_width`, `display_height` |
| `detect` | `<in> --kind KIND` | `silence` / `black` / `scenes`. Read-only. No `-o`. JSON `segments` `{start_s,end_s,kind}`. Video-only + `silence` → `no_audio`. Not valid in `run` |
| `doctor` | | ffmpeg/ffprobe versions |
| `trim` | `<in> --from T --to T -o OUT` | or `--duration T` instead of `--to`. `--accurate` = input `-ss` + `-accurate_seek` + re-encode |
| `cut-out` | `<in> --from T --to T -o OUT` | delete `[from, to)`; keeps the rest and joins. Probes `end`. `--accurate` applies to both trims |
| `keep` | `<in> --ranges A-B,C-end -o OUT` | keep those ranges and join. `end` is probed. User listed N cuts → `keep --ranges` |
| `concat` | `<in...> -o OUT` | ≥2 files; copy only if every input probes and codec, size, fps, and `rotate_deg` match. Do not remux clips to `.ts` yourself; `ave concat` resets timestamps. |
| `resize` | `<in> --preset NAME -o OUT` | `tiktok` `youtube` `twitter` `instagram` `square`. `--fit pad` (default) `crop` `stretch`. Or `--width W --height H` |
| `speed` | `<in> --factor N -o OUT` | `4` = 4×; `0.5` = half |
| `extract-audio` | `<in> -o OUT` | codec from `--format` or `-o` ext (`mp3` `wav` `aac` `flac` `copy`). Video-only → `no_audio` |
| `replace-audio` | `<in> -o OUT` | `--mute` or `--audio FILE` or `--mix FILE` |
| `overlay` | `<in> --image IMG -o OUT` | `--position` `top-right` (default) `top-left` `bottom-left` `bottom-right` `center` |
| `compress` | `<in> -o OUT` | `--crf 23` `--preset medium` |
| `convert` | `<in> -o OUT` | format from `-o` ext; `.gif` = two-pass |
| `frame` | `<in> --at T -o STILL` | one still; ext is `jpg`/`png`/`webp`. `--copy-only` fails |
| `frames` | `<in> --at T,T -o DIR` | or `--every SEC`. Writes `t-<ts>.jpg`. Optional `--sheet PATH`. Not valid in `run` |
| `captions` | `<in> --srt FILE -o OUT` | burn `.srt` or `.vtt`. No styling DSL |
| `text` | `<in> --text STR -o OUT` | `--position lower-third` (default) `center` `top`. Optional `--from` `--to` |
| `fade` | `<in> --in SEC --out SEC -o OUT` | at least one of `--in` / `--out` |
| `volume` | `<in> --db N -o OUT` | signed dB (`-6`, `3`) |
| `rotate` | `<in> --deg 90 -o OUT` | `90` `180` `270` only; re-encodes with `transpose` |
| `crop` | `<in> --bottom N -o OUT` | or `--top` / `--left` / `--right` (pixels, coded frame). At least one edge. Empties the frame → `bad_range`. Not `resize --fit crop` |
| `run` | `plan.json` or `-` | JSON step list; see `references/plans.md`. No `info` / `doctor` / `frames` / `detect` steps |

Timestamps: `HH:MM:SS`, `HH:MM:SS.mmm`, `MM:SS`, or seconds (`90`, `90.5`). Invalid values fail with `bad_timestamp`; `from >= to` or `duration <= 0` fail with `bad_range`.

Presets: tiktok 1080×1920, youtube/twitter 1920×1080, instagram 1080×1350, square 1080×1080. `--fit pad` letterboxes (default). `--fit crop` fills and center-crops. `--fit stretch` scales with no pad.

More examples: `references/commands.md`.

## JSON stdout

Edit success includes `ok`, `op`, `output`, `duration_s`, `width`, `height`, `size_bytes`, `ffmpeg` (argv). `info` is additive: those probe fields plus `video_codec`, `audio_codec`, `fps`, `has_video`, `has_audio`, `rotate_deg`, `display_width`, `display_height`. `width`/`height` are coded samples; display size applies rotation. `detect` is `ok`, `op`, `kind`, `input`, `segments`, `ffmpeg`. `run` adds `steps`. Failure: `ok: false`, `error`, `message`, non-zero exit.

Parse stdout as JSON. Do not scrape ffmpeg banners (they are not on stdout).

## Encode policy

- `trim` / `concat`: stream-copy by default.
- `trim --accurate` / `"accurate": true`: input `-ss` + `-accurate_seek` + re-encode. Not `-c copy`. Do not move `-ss` after `-i`.
- `concat`: probe every input; mismatch or any failed probe (including mixed `rotate_deg`) → re-encode. ffmpeg autorotates on that transcode. Matching copy remuxes through MPEG-TS so DTS stays monotonic.
- Always re-encode: `resize`, `speed`, `overlay`, `compress`, GIF `convert`, `captions`, `text`, `fade`, `volume`, `rotate`, `crop`, `frame`.

## Do not

- Invent ffmpeg filter graphs.
- Guess cut points or match boundaries. Call `detect` first.
- Download stock media unless asked.
- Use this for color grade, multi-cam, karaoke, or YouTube upload.
