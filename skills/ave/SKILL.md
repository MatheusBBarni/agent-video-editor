---
name: ave
description: >
  Edit videos with the ave CLI (ffmpeg recipes, JSON stdout). Use when trimming,
  cutting, concatenating, resizing for TikTok/YouTube/Instagram, changing speed,
  extracting or replacing audio, overlaying a logo, compressing, converting
  (including GIF), probing video info, or running a multi-step edit plan.
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
5. Multi-step (cut a middle, several trims + join) → one `ave run` plan. Single op → the verb.
6. `--dry-run` first when unsure. Then run for real.
7. Reply with output path, duration, resolution, size from the JSON.

## Safety

- Mutating commands **require** `-o` / `"output"`.
- Never edit in place (output must not be an input).
- Never delete inputs. Existing **outputs** are overwritten unless `--no-overwrite`.
- `--dry-run` writes nothing. `--copy-only` fails if a re-encode would happen.
- Paths are cwd-relative or absolute.
- Always pass flags the CLI documents. Clap usage errors go to stderr (not JSON).

## Commands

```text
ave [--dry-run] [--copy-only] [--no-overwrite] [--ffmpeg PATH] [--ffprobe PATH] <CMD>
```

| Cmd | Args | Notes |
|---|---|---|
| `info` | `<in>` | duration, coded `width`/`height`, `video_codec`, `audio_codec`, `fps`, `has_video`, `has_audio`, `rotate_deg`, `display_width`, `display_height` |
| `doctor` | | ffmpeg/ffprobe versions |
| `trim` | `<in> --from T --to T -o OUT` | or `--duration T` instead of `--to`. `--accurate` = input `-ss` + `-accurate_seek` + re-encode |
| `concat` | `<in...> -o OUT` | ≥2 files; copy if shapes match |
| `resize` | `<in> --preset NAME -o OUT` | `tiktok` `youtube` `twitter` `instagram` `square` |
| `speed` | `<in> --factor N -o OUT` | `4` = 4×; `0.5` = half |
| `extract-audio` | `<in> -o OUT` | codec from `--format` or `-o` ext (`mp3` `wav` `aac` `flac` `copy`) |
| `replace-audio` | `<in> -o OUT` | `--mute` or `--audio FILE` or `--mix FILE` |
| `overlay` | `<in> --image IMG -o OUT` | `--position` `top-right` (default) `top-left` `bottom-left` `bottom-right` `center` |
| `compress` | `<in> -o OUT` | `--crf 23` `--preset medium` |
| `convert` | `<in> -o OUT` | format from `-o` ext; `.gif` = two-pass |
| `run` | `plan.json` or `-` | JSON step list; see `references/plans.md` |

Timestamps: `HH:MM:SS`, `HH:MM:SS.mmm`, `MM:SS`, or seconds (`90`, `90.5`).

Presets pad, they do not stretch: tiktok 1080×1920, youtube/twitter 1920×1080, instagram 1080×1350, square 1080×1080.

More examples: `references/commands.md`.

## JSON stdout

Edit success includes `ok`, `op`, `output`, `duration_s`, `width`, `height`, `size_bytes`, `ffmpeg` (argv). `info` is additive: those probe fields plus `video_codec`, `audio_codec`, `fps`, `has_video`, `has_audio`, `rotate_deg`, `display_width`, `display_height`. `width`/`height` are coded samples; display size applies rotation. `run` adds `steps`. Failure: `ok: false`, `error`, `message`, non-zero exit.

Parse stdout as JSON. Do not scrape ffmpeg banners (they are not on stdout).

## Encode policy

- `trim` / `concat`: stream-copy by default.
- `trim --accurate` / `"accurate": true`: input `-ss` + `-accurate_seek` + re-encode. Not `-c copy`. Do not move `-ss` after `-i`.
- `concat`: probe; mismatch → re-encode.
- Always re-encode: `resize`, `speed`, `overlay`, `compress`, GIF `convert`.

## Do not

- Invent ffmpeg filter graphs.
- Guess cut points or match boundaries.
- Download stock media unless asked.
- Use this for color grade, captions, multi-cam, or YouTube upload.
