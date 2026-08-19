use crate::error::{Error, print_json};
use clap::ValueEnum;
use serde::Serialize;
use std::io::ErrorKind;
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
    linked: Vec<String>,
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
        Ok(result) => print_json(&InstallEnvelope {
            ok: true,
            op: "install-skill",
            destinations: result.destinations,
            written: result.written,
            linked: result.linked,
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

struct InstallResult {
    destinations: Vec<String>,
    written: Vec<String>,
    linked: Vec<String>,
}

fn install_inner(
    dirs: &[PathBuf],
    providers: &[Provider],
    global: bool,
    dry_run: bool,
    no_overwrite: bool,
) -> Result<InstallResult, Error> {
    let roots = dest_roots(dirs, providers, global)?;
    let dests: Vec<PathBuf> = roots.into_iter().map(|root| root.join("ave")).collect();
    let destinations: Vec<String> = dests.iter().map(|d| d.display().to_string()).collect();

    let mut dests = dests.into_iter();
    let Some(canonical) = dests.next() else {
        return Ok(InstallResult {
            destinations,
            written: Vec::new(),
            linked: Vec::new(),
        });
    };
    let aliases: Vec<PathBuf> = dests.collect();
    let linked: Vec<String> = aliases.iter().map(|d| d.display().to_string()).collect();

    if dry_run {
        return Ok(InstallResult {
            destinations,
            written: Vec::new(),
            linked,
        });
    }

    let written = write_skill(&canonical, no_overwrite)?;
    for alias in &aliases {
        link_skill(alias, &canonical, no_overwrite)?;
    }

    Ok(InstallResult {
        destinations,
        written,
        linked,
    })
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
    let mut unique = Vec::new();
    for provider in selected {
        if !unique.contains(&provider) {
            unique.push(provider);
        }
    }

    Ok(unique
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
    if is_symlink(dest) {
        if no_overwrite {
            return Err(exists_error(dest));
        }
        remove_path(dest)?;
    }
    let skill = dest.join("SKILL.md");
    if no_overwrite && skill.exists() {
        return Err(exists_error(dest));
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

fn link_skill(dest: &Path, canonical: &Path, no_overwrite: bool) -> Result<(), Error> {
    if dest.symlink_metadata().is_ok() {
        if no_overwrite {
            return Err(exists_error(dest));
        }
        remove_path(dest)?;
    }
    let parent = dest.parent().ok_or_else(|| {
        Error::new(
            "install-skill",
            "write_failed",
            format!("cannot create symlink at {}", dest.display()),
        )
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|e| Error::new("install-skill", "write_failed", e.to_string()))?;
    let target = relative_path(&abs_path(parent), &abs_path(canonical));
    create_symlink(&target, dest)
        .map_err(|e| Error::new("install-skill", "write_failed", e.to_string()))
}

fn exists_error(dest: &Path) -> Error {
    Error::new(
        "install-skill",
        "output_exists",
        format!(
            "skill already exists and --no-overwrite was set: {}",
            dest.display()
        ),
    )
}

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn remove_path(path: &Path) -> Result<(), Error> {
    let meta = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Error::new("install-skill", "write_failed", err.to_string())),
    };
    let result = if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(path)
    } else {
        std::fs::remove_dir_all(path)
    };
    result.map_err(|e| Error::new("install-skill", "write_failed", e.to_string()))
}

fn abs_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => path.to_path_buf(),
        }
    }
}

fn relative_path(from_dir: &Path, to: &Path) -> PathBuf {
    if from_dir.is_absolute() != to.is_absolute() {
        return to.to_path_buf();
    }
    let from: Vec<_> = from_dir.components().collect();
    let to_comps: Vec<_> = to.components().collect();
    let mut i = 0;
    let shared = from.len().min(to_comps.len());
    while i < shared && from[i] == to_comps[i] {
        i += 1;
    }
    let mut rel = PathBuf::new();
    for _ in i..from.len() {
        rel.push("..");
    }
    for component in &to_comps[i..] {
        rel.push(component);
    }
    if rel.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        rel
    }
}

fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(std::io::Error::new(
            ErrorKind::Unsupported,
            "symlinks are not supported on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_pi_to_claude() {
        assert_eq!(
            relative_path(
                Path::new(".pi/agent/skills"),
                Path::new(".claude/skills/ave")
            ),
            Path::new("../../../.claude/skills/ave")
        );
    }

    #[test]
    fn relative_path_agents_to_claude() {
        assert_eq!(
            relative_path(Path::new(".agents/skills"), Path::new(".claude/skills/ave")),
            Path::new("../../.claude/skills/ave")
        );
    }

    #[test]
    fn relative_path_sibling_dirs() {
        assert_eq!(
            relative_path(Path::new("/tmp/b"), Path::new("/tmp/a/ave")),
            Path::new("../a/ave")
        );
    }
}
