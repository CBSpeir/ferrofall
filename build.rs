use std::env;
use std::path::PathBuf;
use std::process::Command;

const SHA_OVERRIDE: &str = "OXIDEFALL_GIT_SHA";
const DIRTY_OVERRIDE: &str = "OXIDEFALL_GIT_DIRTY";

fn main() {
    println!("cargo:rerun-if-env-changed={SHA_OVERRIDE}");
    println!("cargo:rerun-if-env-changed={DIRTY_OVERRIDE}");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    watch_repository_state();

    let revision = env::var(SHA_OVERRIDE)
        .ok()
        .filter(|value| valid_revision(value))
        .or_else(git_revision)
        .or_else(|| {
            env::var("GITHUB_SHA")
                .ok()
                .filter(|value| valid_revision(value))
        })
        .unwrap_or_default();
    let dirty = env::var(DIRTY_OVERRIDE)
        .ok()
        .and_then(|value| parse_bool(&value))
        .unwrap_or_else(git_is_dirty);

    println!("cargo:rustc-env=OXIDEFALL_GIT_SHA={revision}");
    println!("cargo:rustc-env=OXIDEFALL_GIT_DIRTY={dirty}");
}

fn git_revision() -> Option<String> {
    git_output(["rev-parse", "HEAD"])
        .map(|value| value.trim().to_owned())
        .filter(|value| valid_revision(value))
}

fn git_is_dirty() -> bool {
    git_output(["status", "--porcelain=v1", "--untracked-files=no"])
        .is_some_and(|output| !output.trim().is_empty())
}

fn watch_repository_state() {
    if let Some(files) = git_output(["ls-files"]) {
        for file in files.lines().filter(|file| !file.is_empty()) {
            println!("cargo:rerun-if-changed={file}");
        }
    }

    for git_path in ["HEAD", "packed-refs"] {
        if let Some(path) = git_output(["rev-parse", "--git-path", git_path]) {
            let path = PathBuf::from(path.trim());
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    if let Some(reference) = git_output(["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git_output(["rev-parse", "--git-path", reference.trim()])
    {
        let path = PathBuf::from(path.trim());
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn git_output<const N: usize>(arguments: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn valid_revision(value: &str) -> bool {
    value.len() >= 7 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}
