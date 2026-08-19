# ave command examples

All examples assume `ave` is on PATH and cwd holds the files. Add `--dry-run` to print argv without writing.

## info

```bash
ave info clip.mp4
```

JSON includes `duration_s`, coded `width`/`height`, `size_bytes`, `video_codec`, `audio_codec`, `fps`, `has_video`, `has_audio`, `rotate_deg`, `display_width`, `display_height`. Missing streams use `""` / `false` / `0`. `rotate_deg` is `0`, `90`, `180`, or `270`.

## detect

```bash
ave detect clip.mp4 --kind silence
ave detect clip.mp4 --kind black --dry-run
ave detect clip.mp4 --kind scenes
```

Read-only. No `-o`. `--kind` is required: `silence` (`silencedetect=noise=-30dB:d=0.5`), `black` (`blackdetect=d=0.5:pix_th=0.10`), or `scenes` (`scdet`). Unknown kind → `unknown_kind`. Video-only + `silence` → `no_audio`. `--copy-only` / `--no-overwrite` are ignored. JSON `segments` is `{ start_s, end_s, kind }`. Empty `segments` is success. Not valid inside `ave run`.

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

## keep

```bash
ave keep clip.mp4 --ranges 0-2,5-8 -o kept.mp4
ave keep vod.mp4 --ranges 0-18:40,19:17-1:18:20,1:18:24-end -o vod2.mp4
```

Keeps each FROM-TO and joins them. end as TO means the probed duration. Overlap, FROM >= TO, or TO past the file end is bad_range. User listed N cuts uses keep --ranges, not N trims + concat.

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

JSON may use `"preset"` or `"width"` + `"height"`. `--fit pad` (default) letterboxes. `--fit crop` fills and center-crops. `--fit stretch` scales with no pad.

```bash
ave resize clip.mp4 --preset tiktok --fit crop -o short.mp4
ave resize clip.mp4 --width 640 --height 360 --fit stretch -o wide.mp4
```

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

## frame

```bash
ave frame clip.mp4 --at 1 -o poster.jpg
ave frame clip.mp4 --at 00:00:12.5 -o still.png
```

One still. Format from `-o` (`jpg` / `png` / `webp`). `--copy-only` fails.

## frames

```bash
ave frames clip.mp4 --at 1,18:40 -o review
ave frames clip.mp4 --every 30 -o review
ave frames clip.mp4 --at 1,2 --sheet sheet.jpg -o review
```

`-o` is a directory (`t-1.jpg`, `t-18-40.jpg`). `--at` xor `--every`. `--every SEC` is `0, SEC, 2*SEC, …` while `t <= duration` (`floor(duration/SEC)+1` stills). `--sheet` is an extra contact-sheet image. Not valid inside `ave run`. Do not invent `fps=` dumps.

## captions

```bash
ave captions clip.mp4 --srt subs.srt -o burned.mp4
ave captions clip.mp4 --srt subs.vtt -o burned.mp4
```

Burns the file. No styling DSL. `.srt` or `.vtt` only.

## text

```bash
ave text clip.mp4 --text "Hello" --position lower-third -o titled.mp4
ave text clip.mp4 --text "Hello" --position center --from 0 --to 3 -o titled.mp4
```

One locked drawtext. Positions: `lower-third` (default), `center`, `top`.

## fade

```bash
ave fade clip.mp4 --in 0.5 --out 0.5 -o faded.mp4
ave fade clip.mp4 --in 1 -o in.mp4
```

At least one of `--in` / `--out`.

## volume

```bash
ave volume clip.mp4 --db -6 -o quieter.mp4
ave volume clip.mp4 --db 3 -o louder.mp4
```

## rotate

```bash
ave rotate clip.mp4 --deg 90 -o turned.mp4
```

`90`, `180`, or `270`. Re-encodes with `transpose`. Other degrees are `bad_range`.

## crop

```bash
ave crop clip.mp4 --bottom 40 -o no-taskbar.mp4
ave crop clip.mp4 --top 80 --bottom 40 -o no-chrome.mp4
```

Pixels in the coded frame. At least one of `--top` / `--bottom` / `--left` / `--right`. Hide a Windows taskbar with `--bottom 40`, then `resize` if you still need 1080p. This is not `resize --fit crop` (center-fill). A crop that empties the frame is `bad_range`. `--copy-only` fails.

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
