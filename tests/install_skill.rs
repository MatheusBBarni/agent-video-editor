use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn install_skill_writes_skill_files_to_dir() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("skills-root");

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .args(["install-skill", "--dir", dest.to_str().unwrap()])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], "install-skill");
    assert_eq!(v["linked"], serde_json::json!([]));

    let skill = dest.join("ave/SKILL.md");
    let commands = dest.join("ave/references/commands.md");
    let plans = dest.join("ave/references/plans.md");
    assert!(skill.exists(), "missing {}", skill.display());
    assert!(commands.exists(), "missing {}", commands.display());
    assert!(plans.exists(), "missing {}", plans.display());
    let body = fs::read_to_string(skill).unwrap();
    assert!(
        body.contains("name: ave"),
        "SKILL.md should be the ave skill"
    );
}

#[test]
fn install_skill_dry_run_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("skills-root");

    Command::cargo_bin("ave")
        .unwrap()
        .args([
            "--dry-run",
            "install-skill",
            "--dir",
            dest.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        !dest.join("ave").exists(),
        "dry-run must not write the skill"
    );
}

#[test]
fn install_skill_without_provider_lists_choices() {
    let assert = Command::cargo_bin("ave")
        .unwrap()
        .args(["install-skill"])
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "missing_provider");
    let ids: Vec<&str> = v["providers"]
        .as_array()
        .expect("providers")
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"claude"));
    assert!(ids.contains(&"agents"));
    assert!(ids.contains(&"pi"));
    assert!(ids.contains(&"cursor"));
}

#[test]
fn install_skill_provider_writes_only_that_folder() {
    let dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["install-skill", "--provider", "claude", "--provider", "pi"])
        .assert()
        .success();

    let claude = dir.path().join(".claude/skills/ave");
    let pi = dir.path().join(".pi/agent/skills/ave");
    assert!(claude.join("SKILL.md").is_file());
    assert!(pi.join("SKILL.md").exists());
    assert!(is_symlink(&pi), "extra providers must be a symlink");
    assert!(!is_symlink(&claude), "first provider must be a real folder");
    assert_eq!(
        fs::canonicalize(&pi).unwrap(),
        fs::canonicalize(&claude).unwrap()
    );
    assert!(!dir.path().join(".agents/skills/ave/SKILL.md").exists());
    assert!(!dir.path().join(".cursor/skills/ave/SKILL.md").exists());
}

#[test]
fn install_skill_first_provider_is_canonical() {
    let dir = tempfile::tempdir().unwrap();

    let assert = Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["install-skill", "--provider", "pi,claude"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(v["linked"], serde_json::json!([".claude/skills/ave"]));

    let pi = dir.path().join(".pi/agent/skills/ave");
    let claude = dir.path().join(".claude/skills/ave");
    assert!(pi.join("SKILL.md").is_file());
    assert!(!is_symlink(&pi));
    assert!(is_symlink(&claude));
    assert_eq!(
        fs::canonicalize(&claude).unwrap(),
        fs::canonicalize(&pi).unwrap()
    );
}

#[test]
fn install_skill_dirs_symlink_extras() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("one");
    let second = dir.path().join("two");

    Command::cargo_bin("ave")
        .unwrap()
        .args([
            "install-skill",
            "--dir",
            first.to_str().unwrap(),
            "--dir",
            second.to_str().unwrap(),
        ])
        .assert()
        .success();

    let canonical = first.join("ave");
    let linked = second.join("ave");
    assert!(canonical.join("SKILL.md").is_file());
    assert!(!is_symlink(&canonical));
    assert!(is_symlink(&linked));
    assert_eq!(
        fs::canonicalize(&linked).unwrap(),
        fs::canonicalize(&canonical).unwrap()
    );
}

#[test]
fn install_skill_replaces_existing_copy_with_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let leftover = dir.path().join(".pi/agent/skills/ave");
    fs::create_dir_all(leftover.join("references")).unwrap();
    fs::write(leftover.join("SKILL.md"), "old").unwrap();

    Command::cargo_bin("ave")
        .unwrap()
        .current_dir(dir.path())
        .args(["install-skill", "--provider", "claude,pi"])
        .assert()
        .success();

    let claude = dir.path().join(".claude/skills/ave");
    assert!(is_symlink(&leftover));
    assert_eq!(
        fs::canonicalize(&leftover).unwrap(),
        fs::canonicalize(&claude).unwrap()
    );
    let body = fs::read_to_string(leftover.join("SKILL.md")).unwrap();
    assert!(body.contains("name: ave"));
}

fn is_symlink(path: &std::path::Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}
