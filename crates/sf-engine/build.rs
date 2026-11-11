use std::path::Path;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn watch_git_path(name: &str) {
    if let Some(path) = git(&["rev-parse", "--git-path", name]) {
        // Resolve through Git so linked worktrees also watch the shared refs.
        // Missing packed-refs must not make every Cargo invocation rebuild.
        if Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    watch_git_path("HEAD");
    watch_git_path("refs");
    watch_git_path("packed-refs");
    if let Some(branch) = git(&["symbolic-ref", "--quiet", "HEAD"]) {
        watch_git_path(&branch);
    }

    let sha = git(&["rev-parse", "--short=8", "HEAD"]).unwrap_or_else(|| "nogit".into());
    println!("cargo:rustc-env=SF1_GIT_SHA={sha}");
}
