# ave command examples

All examples assume `ave` is on PATH and cwd holds the files. Add `--dry-run` to print argv without writing.

## info

```bash
ave info clip.mp4
```

JSON includes `duration_s`, coded `width`/`height`, `size_bytes`, `video_codec`, `audio_codec`, `fps`, `has_video`, `has_audio`, `rotate_deg`, `display_width`, `display_height`. Missing streams use `""` / `false` / `0`. `rotate_deg` is `0`, `90`, `180`, or `270`.

## trim

```bash
ave trim clip.mp4 --from 30 --to 105 -o keep.mp4
ave trim clip.mp4 --from 30 --duration 5 -o keep.mp4
ave trim clip.mp4 --from 00:00:30 --to 00:01:45 --accurate -o keep.mp4
```

`--from` plus exactly one of `--to` or `--duration`. `--to` becomes ffmpeg `-to`; `--duration` becomes `-t`. Do not pass both (`conflicting_fields`). Default trim stream-copies (`-ss` and `-to`/`-t` before `-i`, `-c copy`). `--accurate` keeps those as input options, adds `-accurate_seek`, and re-encodes (`libx264` / `aac`). Do not move `-ss` after `-i`.

## cut-out

```bash
ave cut-out clip.mp4 --from 12 --to 18 -o kept.mp4
ave cut-out clip.mp4 --from 12 --to 18 --accurate -o kept.mp4
```

Deletes from-to and joins what remains. `end` comes from ffprobe. Do not pass it, and do not write two trims plus concat. `--accurate` applies to both internal trims. `from >= to` or `to` past the file end is `bad_range`.

## concat

```bash
ave concat a.mp4 b.mp4 c.mp4 -o joined.mp4
```

Stream-copies when every input probes and codec, coded size, fps, and `rotate_deg` match. Any missing or unreadable input re-encodes. Mixed rotation (e.g. 0° + 90°) re-encodes so ffmpeg can autorotate. Matching copy remuxes through MPEG-TS so joined timestamps stay monotonic. Do not remux clips to `.ts` yourself.

## resize

```bash
ave resize clip.mp4 --preset tiktok -o short.mp4
ave resize clip.mp4 --preset youtube -o yt.mp4
```

JSON may use `"preset"` or `"width"` + `"height"` (scale + center pad).

## speed

```bash
ave speed clip.mp4 --factor 2 -o fast.mp4
ave speed clip.mp4 --factor 0.5 -o slow.mp4
ave speed clip.mp4 --factor 4 -o fast4x.mp4
```

## extract-audio

```bash
ave extract-audio clip.mp4 -o audio.mp3
ave extract-audio clip.mp4 --format wav -o audio.wav
```

Fails with `no_audio` when the input probes and has no audio stream.

## replace-audio

```bash
ave replace-audio clip.mp4 --mute -o silent.mp4
ave replace-audio clip.mp4 --audio voice.mp3 -o dubbed.mp4
ave replace-audio clip.mp4 --mix music.mp3 -o mixed.mp4
```

## overlay

```bash
ave overlay clip.mp4 --image logo.png --position top-right -o marked.mp4
```

## compress

```bash
ave compress clip.mp4 -o small.mp4
ave compress clip.mp4 --crf 28 --preset slow -o smaller.mp4
```

## convert

```bash
ave convert clip.mov -o clip.mp4
ave convert clip.mp4 -o clip.gif
```

GIF uses a two-pass palette. Do not leave `palette.png` around; `ave` removes it.

## install-skill

Installs the bundled skill so agents can load it. The first `--provider` or `--dir` gets the files; every extra destination is a symlink to that folder.

```bash
ave install-skill --provider claude
ave install-skill --provider claude,pi,agents
ave install-skill --provider all --global
ave install-skill --dir /path/to/skills
ave --dry-run install-skill --provider cursor
```

`--provider` is multi-select (`agents`, `claude`, `pi`, `cursor`, `all`). Repeat the flag or use commas. No `--provider` and no `--dir` → JSON lists the choices and exits 1.

`--global` uses `~/…` instead of the project. `--dir DIR` writes `DIR/ave` and skips providers. Extra `--dir` values become symlinks to the first.

## doctor

```bash
ave doctor
ave --ffmpeg /opt/homebrew/bin/ffmpeg --ffprobe /opt/homebrew/bin/ffprobe doctor
```

## globals

```bash
ave --dry-run trim in.mp4 --from 0 --to 10 -o out.mp4
ave --no-overwrite trim in.mp4 --from 0 --to 10 -o out.mp4
ave --copy-only concat a.mp4 b.mp4 -o out.mp4
```
