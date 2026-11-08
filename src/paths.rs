// Locating the tasks/ folder and rendering paths relative to the current directory.
//
// Relative paths are always rendered with `/`. Windows accepts forward slashes, and
// it keeps the `path:line:` output identical on every platform.

use std::path::{Component, Path, PathBuf};

use crate::error::{Context, Error, Result};

pub struct TasksDir {
    pub abs: PathBuf,
    /// Path of the tasks/ folder relative to the current directory, like `./tasks`.
    pub rel: String,
    pub cwd: PathBuf,
}

impl TasksDir {
    /// Searches the current directory and its ancestors for a `tasks/` folder.
    pub fn find() -> Result<TasksDir> {
        let cwd = std::env::current_dir().context("could not get the current directory")?;
        for dir in cwd.ancestors() {
            let candidate = dir.join("tasks");
            if candidate.is_dir() {
                let rel = relative(&cwd, &candidate);
                return Ok(TasksDir { abs: candidate, rel, cwd });
            }
        }
        Err(Error::Msg("Could not find tasks/ folder".to_string()))
    }

    /// The HUID of the task folder we are currently in, if the cwd is `tasks/<HUID>/`.
    pub fn current_task_id(&self) -> Option<String> {
        if self.cwd.parent()? != self.abs {
            return None;
        }
        Some(self.cwd.file_name()?.to_string_lossy().into_owned())
    }
}

#[derive(PartialEq, Eq)]
enum Part {
    Prefix(String),
    Root,
    Name(String),
}

fn normalized_parts(path: &Path) -> Vec<Part> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            // Drive letters are case-insensitive on Windows.
            Component::Prefix(p) => parts.push(Part::Prefix(p.as_os_str().to_string_lossy().to_lowercase())),
            Component::RootDir => parts.push(Part::Root),
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(parts.last(), Some(Part::Name(_))) {
                    parts.pop();
                }
            }
            Component::Normal(name) => parts.push(Part::Name(name.to_string_lossy().into_owned())),
        }
    }
    parts
}

/// Renders `target` relative to `base`. Both are expected to be absolute.
/// Returns `.` for the same directory and prefixes downward paths with `./`
/// (`./tasks`, `../tasks`, `../..`).
pub fn relative(base: &Path, target: &Path) -> String {
    let base_parts = normalized_parts(base);
    let target_parts = normalized_parts(target);

    // Different drives (or a relative input) have no relative path between them.
    if base_parts.first() != target_parts.first() {
        return target.to_string_lossy().replace('\\', "/");
    }

    let common = base_parts.iter().zip(&target_parts).take_while(|(a, b)| a == b).count();
    let mut out: Vec<&str> = vec![".."; base_parts.len() - common];
    if out.is_empty() {
        out.push(".");
    }
    for part in &target_parts[common..] {
        if let Part::Name(name) = part {
            out.push(name);
        }
    }
    out.join("/")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        let cases = [
            ("/home/rexim/sofren", "/home/rexim/sofren/tasks", "./tasks"),
            ("/home/rexim/sofren", "/home/rexim/sofren/tasks/", "./tasks"),
            ("/home/rexim/sofren/tasks", "/home/rexim/sofren/tasks", "."),
            ("/", "/", "."),
            ("/home/rexim/sofren/tasks/20250831-161356/", "/home/rexim/sofren/tasks", ".."),
            ("/home/rexim/sofren/src", "/home/rexim/sofren/tasks", "../tasks"),
            ("/home/rexim/sofren/src/game/", "/home/rexim/sofren/tasks", "../../tasks"),
            ("/home_/rexim/sofren/tasks", "/home/rexim/sofren/tasks", "../../../../home/rexim/sofren/tasks"),
            ("/home/rexim/sofren/tasks", "/", "../../../.."),
            ("/", "/home/rexim/sofren/tasks", "./home/rexim/sofren/tasks"),
            ("/home//rexim/./x/../sofren", "/home/rexim/sofren/tasks", "./tasks"),
        ];
        for (base, target, expected) in cases {
            assert_eq!(relative(Path::new(base), Path::new(target)), expected, "{base} -> {target}");
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        let cases = [
            (r"C:\Users\me\proj", r"C:\Users\me\proj\tasks", "./tasks"),
            (r"C:\Users\me\proj\src", r"c:\Users\me\proj\tasks", "../tasks"),
            (r"C:\Users\me\proj\tasks\20250831-161356", r"C:\Users\me\proj\tasks", ".."),
            (r"D:\other", r"C:\Users\me\proj\tasks", "C:/Users/me/proj/tasks"),
        ];
        for (base, target, expected) in cases {
            assert_eq!(relative(Path::new(base), Path::new(target)), expected, "{base} -> {target}");
        }
    }
}
