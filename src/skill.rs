use crate::error::{Error, print_json};
use clap::ValueEnum;
use serde::Serialize;
use std::path::{Path, PathBuf};

const SKILL_MD: &str = include_str!("../skills/ave/SKILL.md");
const COMMANDS_MD: &str = include_str!("../skills/ave/references/commands.md");
const PLANS_MD: &str = include_str!("../skills/ave/references/plans.md");

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Provider {
    /// AGENTS.md / Codex / generic (.agents/skills)
    Agents,
    /// Claude Code (.claude/skills)
    Claude,
    /// Pi (.pi/agent/skills)
    Pi,
    /// Cursor (.cursor/skills)
    Cursor,
    /// Every built-in provider
    All,
}

#[derive(Clone, Serialize)]
struct ProviderInfo {
    id: &'static str,
    project: &'static str,
    global: &'static str,
}

#[derive(Serialize)]
struct InstallEnvelope {
    ok: bool,
    op: &'static str,
    destinations: Vec<String>,
    written: Vec<String>,
    providers: Vec<ProviderInfo>,
}

#[derive(Serialize)]
struct InstallFailEnvelope {
    ok: bool,
    op: &'static str,
    error: &'static str,
    message: String,
    providers: Vec<ProviderInfo>,
}

const PROVIDERS: [ProviderInfo; 4] = [
    ProviderInfo {
        id: "agents",
        project: ".agents/skills",
        global: "~/.agents/skills",
    },
    ProviderInfo {
        id: "claude",
        project: ".claude/skills",
        global: "~/.claude/skills",
    },
    ProviderInfo {
        id: "pi",
        project: ".pi/agent/skills",
        global: "~/.pi/agent/skills",
    },
    ProviderInfo {
        id: "cursor",
        project: ".cursor/skills",
        global: "~/.cursor/skills",
    },
];

pub fn install(
    dirs: &[PathBuf],
    providers: &[Provider],
    global: bool,
    dry_run: bool,
    no_overwrite: bool,
) {
    match install_inner(dirs, providers, global, dry_run, no_overwrite) {
        Ok((destinations, written)) => print_json(&InstallEnvelope {
            ok: true,
            op: "install-skill",
            destinations,
            written,
            providers: PROVIDERS.to_vec(),
        }),
        Err(err) if err.code == "missing_provider" => {
            print_json(&InstallFailEnvelope {
                ok: false,
                op: "install-skill",
                error: err.code,
                message: err.message,
                providers: PROVIDERS.to_vec(),
            });
            std::process::exit(1);
        }
        Err(err) => crate::error::fail(err),
    }
}

fn install_inner(
    dirs: &[PathBuf],
    providers: &[Provider],
    global: bool,
    dry_run: bool,
    no_overwrite: bool,
) -> Result<(Vec<String>, Vec<String>), Error> {
    let roots = dest_roots(dirs, providers, global)?;
    let mut destinations = Vec::new();
    let mut written = Vec::new();

    for root in roots {
        let dest = root.join("ave");
        destinations.push(dest.display().to_string());
        if dry_run {
            continue;
        }
        written.extend(write_skill(&dest, no_overwrite)?);
    }

    Ok((destinations, written))
}

fn dest_roots(
    dirs: &[PathBuf],
    providers: &[Provider],
    global: bool,
) -> Result<Vec<PathBuf>, Error> {
    if !dirs.is_empty() {
        return Ok(dirs.to_vec());
    }
    if providers.is_empty() {
        return Err(Error::new(
            "install-skill",
            "missing_provider",
            "choose one or more --provider values (repeatable or comma-separated)",
        ));
    }

    let home = if global {
        Some(std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
            Error::new(
                "install-skill",
                "no_home",
                "could not resolve home directory",
            )
        })?)
    } else {
        None
    };

    let mut selected = Vec::new();
    for provider in providers {
        match provider {
            Provider::All => {
                selected.extend([
                    Provider::Agents,
                    Provider::Claude,
                    Provider::Pi,
                    Provider::Cursor,
                ]);
            }
            other => selected.push(*other),
        }
    }
    selected.sort_by_key(|p| *p as u8);
    selected.dedup();

    Ok(selected
        .into_iter()
        .map(|p| provider_root(p, home.as_deref()))
        .collect())
}

fn provider_root(provider: Provider, home: Option<&Path>) -> PathBuf {
    let rel = match provider {
        Provider::Agents => Path::new(".agents/skills"),
        Provider::Claude => Path::new(".claude/skills"),
        Provider::Pi => Path::new(".pi/agent/skills"),
        Provider::Cursor => Path::new(".cursor/skills"),
        Provider::All => unreachable!("All is expanded before this"),
    };
    match home {
        Some(home) => home.join(rel),
        None => rel.to_path_buf(),
    }
}

fn write_skill(dest: &Path, no_overwrite: bool) -> Result<Vec<String>, Error> {
    let skill = dest.join("SKILL.md");
    if no_overwrite && skill.exists() {
        return Err(Error::new(
            "install-skill",
            "output_exists",
            format!(
                "skill already exists and --no-overwrite was set: {}",
                dest.display()
            ),
        ));
    }
    std::fs::create_dir_all(dest.join("references"))
        .map_err(|e| Error::new("install-skill", "write_failed", e.to_string()))?;
    let files = [
        (skill, SKILL_MD),
        (dest.join("references/commands.md"), COMMANDS_MD),
        (dest.join("references/plans.md"), PLANS_MD),
    ];
    let mut written = Vec::new();
    for (path, body) in files {
        std::fs::write(&path, body)
            .map_err(|e| Error::new("install-skill", "write_failed", e.to_string()))?;
        written.push(path.display().to_string());
    }
    Ok(written)
}
