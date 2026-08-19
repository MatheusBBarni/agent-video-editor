# ave run plans

Use `ave run` when the job is more than one verb. One hole → `cut-out`. N cuts → `keep --ranges`, not N trims + concat.

```bash
ave run plan.json
ave run plan.json --dry-run
ave run - < plan.json
```

## Schema

```json
{
  "steps": [ { "op": "<verb>", "...": "..." } ]
}
```

Explicit paths only. No `$prev`. A later step may name an earlier `"output"` as `"input"` / `"inputs"`. On `--dry-run` those files need not exist yet.

Unknown `op` or missing required fields → fail **before** any step. Step failure → stop, keep files already written, `failed_step` is the 0-based index.

## Field map

| op | required | optional |
|---|---|---|
| `trim` | `input`, `from`, `output`, and exactly one of `to` or `duration` | `accurate` |
| `cut-out` | `input`, `from`, `to`, `output` | `accurate` |
| `keep` | `input`, `ranges`, `output` | `accurate` |
| `concat` | `inputs` (≥2), `output` | |
| `resize` | `input`, `output`, `preset` **or** `width`+`height` | |
| `speed` | `input`, `output`, `factor` | |
| `extract-audio` | `input`, `output` | `format` |
| `replace-audio` | `input`, `output`, and one of `mute` / `audio` / `mix` | |
| `overlay` | `input`, `image`, `output` | `position` |
| `compress` | `input`, `output` | `crf`, `preset` |
| `convert` | `input`, `output` | |

`info` and `doctor` are CLI verbs only. They are not valid plan ops (`unsupported_in_run`).

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

## Trim then TikTok

```json
{
  "steps": [
    {"op": "trim", "input": "vlog.mp4", "from": "12", "to": "48", "output": "cut.mp4"},
    {"op": "resize", "input": "cut.mp4", "preset": "tiktok", "output": "short.mp4"}
  ]
}
```
