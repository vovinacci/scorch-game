//! Embeds the build version as `SCORCH_BUILD`, following the rules in
//! "Versioning" in docs/GUIDE.md:
//!   0.1.0                   built from tag v0.1.0, no local changes
//!   0.1.0+3.gabc1234        3 commits after the tag
//!   0.1.0+3.gabc1234.dirty  ...with uncommitted changes
//!   0.1.0+gabc1234          no release tag yet
//!   0.1.0+nogit             not built from a git checkout

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    for path in ["src", "docs", "Cargo.toml"] {
        println!("cargo:rerun-if-changed={path}");
    }
    if let Some(git_dir) = git(&["rev-parse", "--git-dir"]) {
        for file in ["HEAD", "index", "refs"] {
            println!("cargo:rerun-if-changed={git_dir}/{file}");
        }
    }

    let build = match git(&["rev-parse", "--short=7", "HEAD"]) {
        None => format!("{version}+nogit"),
        Some(sha) => {
            let dirty = git(&["status", "--porcelain", "--untracked-files=no"])
                .is_some_and(|s| !s.is_empty());
            let tag = format!("v{version}");
            let mut parts = Vec::new();
            match git(&["rev-list", "--count", &format!("{tag}..HEAD")]) {
                Some(n) if n == "0" => {}
                Some(n) => parts.push(format!("{n}.g{sha}")),
                None => parts.push(format!("g{sha}")),
            }
            if dirty {
                parts.push("dirty".to_string());
            }
            if parts.is_empty() {
                version.to_string()
            } else {
                format!("{version}+{}", parts.join("."))
            }
        }
    };
    println!("cargo:rustc-env=SCORCH_BUILD={build}");
}
