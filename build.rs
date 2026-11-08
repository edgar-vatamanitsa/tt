// Bakes the git commit, build time and compiler version into the binary for `tt version`.
// Every piece is optional: building from a source tarball without git still works.

use std::process::Command;

#[path = "src/time.rs"]
#[allow(dead_code)]
mod time;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    for git_path in [".git/HEAD", ".git/refs", ".git/packed-refs"] {
        if std::path::Path::new(git_path).exists() {
            println!("cargo:rerun-if-changed={git_path}");
        }
    }

    if let Some(hash) = command_stdout("git", &["rev-parse", "--short", "HEAD"]) {
        println!("cargo:rustc-env=TT_GIT_HASH={hash}");
    }

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    if let Some(version) = command_stdout(&rustc, &["--version"]) {
        println!("cargo:rustc-env=TT_COMPILER_VERSION={version}");
    }

    let t = time::UtcDateTime::now();
    println!(
        "cargo:rustc-env=TT_BUILD_TIME={:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    );
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().next()?.trim().to_string();
    (!line.is_empty()).then_some(line)
}
