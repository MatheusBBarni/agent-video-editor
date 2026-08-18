# ave

CLI for agents that need to edit video without inventing ffmpeg flags. Spawns system `ffmpeg` / `ffprobe`. JSON on stdout.

## Install

```bash
brew install ffmpeg   # or apt install ffmpeg
cargo install --path .
ave doctor
ave install-skill --provider claude,pi
ave install-skill --provider all --global
```

## Skill (skills.sh)

Agents should read **`skills/ave/SKILL.md`**.

```
skills/ave/SKILL.md
skills/ave/references/commands.md
skills/ave/references/plans.md
```

## Quick use

```bash
ave info clip.mp4
ave trim clip.mp4 --from 30 --to 105 -o keep.mp4 --dry-run
ave trim clip.mp4 --from 30 --to 105 -o keep.mp4
ave run plan.json
```

See the skill for verbs, safety, and JSON plans.
