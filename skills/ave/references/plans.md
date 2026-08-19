# ave run plans

Use `ave run` when the job is more than one verb. One hole → `cut-out`. N cuts → `keep --ranges`, not N trims + concat.

```bash
ave run plan.json
ave run plan.json --dry-run
ave run plan.json --workdir tmp
ave schema
ave run - < plan.json
```

`ave schema` prints the JSON Schema for this plan. `--workdir DIR` puts relative `output` paths (and later inputs that reuse those names) under `DIR`. Absolute paths stay as written. `--dry-run` does not create `DIR`.

## Schema

```json
{
  "hw": "videotoolbox",
  "steps": [ { "op": "<verb>", "...": "..." } ]
}
```

Optional plan-root `hw` is `none` / `videotoolbox` / `nvenc`. CLI `--hw` wins if both are set.

Explicit paths only. No `$prev`. A later step may name an earlier `"output"` as `"input"` / `"inputs"`. On `--dry-run` those files need not exist yet.

Unknown `op` or missing required fields → fail **before** any step. Step failure → stop, keep files already written, `failed_step` is the 0-based index.

## Field map

| op | required | optional |
|---|---|---|
| `trim` | `input`, `from`, `output`, and exactly one of `to` or `duration` | `accurate` |
| `cut-out` | `input`, `from`, `to`, `output` | `accurate` |
| `keep` | `input`, `ranges`, `output` | `accurate` |
| `concat` | `inputs` (≥2), `output` | |
| `resize` | `input`, `output`, `preset` **or** `width`+`height` | `fit` (`pad`/`crop`/`stretch`), `stretch` |
| `frame` | `input`, `at`, `output` | |
| `captions` | `input`, `srt`, `output` | |
| `text` | `input`, `text`, `output` | `position`, `from`, `to` |
| `fade` | `input`, `output`, and at least one of `in` / `out` | |
| `volume` | `input`, `db`, `output` | |
| `rotate` | `input`, `deg`, `output` | |
| `crop` | `input`, `output`, and at least one of `top` / `bottom` / `left` / `right` | |
| `speed` | `input`, `output`, `factor` | |
| `extract-audio` | `input`, `output` | `format` |
| `replace-audio` | `input`, `output`, and one of `mute` / `audio` / `mix` | |
| `overlay` | `input`, `image`, `output` | `position` or `x`+`y`, `opacity`, `from`, `to` |
| `compress` | `input`, `output` | `crf`, `preset` |
| `convert` | `input`, `output` | |

`info`, `doctor`, `frames`, and `detect` are CLI verbs only. They are not valid plan ops (`unsupported_in_run`).

## Delete a middle section

```bash
ave cut-out in.mp4 --from 12 --to 18 -o out.mp4
```

```json
{
  "steps": [
    {"op": "cut-out", "input": "in.mp4", "from": "12", "to": "18", "output": "out.mp4"}
  ]
}
```

Do not invert the hole into keep-ranges or guess `end` from `info`.

## Drop a taskbar then 1080p

```json
{
  "steps": [
    {"op": "crop", "input": "game.mp4", "bottom": 40, "output": "cropped.mp4"},
    {"op": "resize", "input": "cropped.mp4", "preset": "youtube", "output": "out.mp4"}
  ]
}
```

Do not invent `crop=W:H:X:Y`. After `crop`, call `resize` if you need a preset.

## Trim then TikTok

```json
{
  "steps": [
    {"op": "trim", "input": "vlog.mp4", "from": "12", "to": "48", "output": "cut.mp4"},
    {"op": "resize", "input": "cut.mp4", "preset": "tiktok", "output": "short.mp4"}
  ]
}
```
