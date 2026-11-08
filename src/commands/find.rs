// Commands that address a single task by its HUID: find, id, ref.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use super::{Ctx, usage_error};
use crate::error::{Error, Result, fail};
use crate::flags::{Args, Flag};
use crate::huid::{HUID_REGEXP_FOR_USER_REPORT_PURPOSES, is_valid_huid};
use crate::paths::{TasksDir, relative};
use crate::task::load_tasks;
use crate::term::{out, outln, write_stdout};
use crate::text::find_bytes;

pub const FIND_FLAGS: &[Flag] = &[Flag::bool("path-only", "Print only path to TASK.md")];
pub const ID_FLAGS: &[Flag] = &[];
pub const REF_FLAGS: &[Flag] = &[];

/// The single optional HUID positional argument of `find` and `ref`.
fn huid_arg<'a>(ctx: &Ctx, command: &str, args: &'a Args) -> Result<Option<&'a str>> {
    match args.rest.as_slice() {
        [] => Ok(None),
        [huid] if is_valid_huid(huid) => Ok(Some(huid)),
        [huid] => Err(usage_error(
            ctx,
            command,
            &format!("`{huid}` is not a valid HUID. Valid HUID matches regexp {HUID_REGEXP_FOR_USER_REPORT_PURPOSES}"),
        )),
        _ => Err(usage_error(ctx, command, "Several HUIDs are not supported")),
    }
}

pub fn find_run(ctx: &Ctx, args: &Args) -> Result<()> {
    let Some(huid) = huid_arg(ctx, "find", args)? else {
        return Err(usage_error(ctx, "find", "No HUID was provided"));
    };
    let db = TasksDir::find()?;
    let tasks = load_tasks(&db.abs)?;
    let Some(task) = tasks.iter().find(|t| t.id == huid) else {
        fail!("No task with HUID `{huid}` was found");
    };
    if args.bool("path-only") {
        outln!("{}/{}/TASK.md", db.rel, task.id);
    } else {
        outln!("{}", task.report(&db.rel, None));
    }
    Ok(())
}

pub fn id_run(_ctx: &Ctx, _args: &Args) -> Result<()> {
    let db = TasksDir::find()?;
    let Some(id) = db.current_task_id() else {
        fail!("You must be inside of a specific task folder like {}/<HUID>/", db.rel);
    };
    // No trailing newline so it can be piped straight into the clipboard.
    out!("{id}");
    Ok(())
}

pub fn ref_run(ctx: &Ctx, args: &Args) -> Result<()> {
    let huid_from_args = huid_arg(ctx, "ref", args)?;
    let db = TasksDir::find()?;
    let huid = match huid_from_args {
        Some(huid) => huid.to_string(),
        None => match db.current_task_id() {
            Some(id) => id,
            None => fail!(
                "You should be inside of a specific task folder like {}/<HUID>. Alternatively you can pass the <HUID> of the task as an argument to search for referers of that task.",
                db.rel
            ),
        },
    };

    let root = db.abs.parent().unwrap_or(&db.abs);
    let root_display = relative(&db.cwd, root);
    let mut found = 0;
    search_dir(root, &root_display, huid.as_bytes(), &mut found);
    if found == 0 {
        eprintln!("No references to {huid} were found");
        return Err(Error::Reported);
    }
    Ok(())
}

/// Like `grep -Irn --exclude-dir=.git`, but portable.
fn search_dir(dir: &Path, display: &str, needle: &[u8], found: &mut usize) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("WARNING: could not read {display}: {e}");
            return;
        }
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let display = format!("{display}/{name}");
        // Symlinks are not followed, which also protects against cycles.
        let Ok(file_type) = entry.file_type() else { continue };
        if file_type.is_dir() {
            if name != ".git" {
                search_dir(&path, &display, needle, found);
            }
        } else if file_type.is_file() {
            search_file(&path, &display, needle, found);
        }
    }
}

fn search_file(path: &Path, display: &str, needle: &[u8], found: &mut usize) {
    // Peek at the start first so big binaries (build artifacts, images) are skipped cheaply.
    const BINARY_PROBE: usize = 8000;
    let mut content = Vec::new();
    let read = File::open(path).and_then(|mut file| {
        (&mut file).take(BINARY_PROBE as u64).read_to_end(&mut content)?;
        if content.contains(&0) {
            return Ok(false);
        }
        file.read_to_end(&mut content)?;
        Ok(true)
    });
    match read {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            eprintln!("WARNING: could not read {display}: {e}");
            return;
        }
    }
    if find_bytes(&content, needle).is_none() {
        return;
    }
    for (index, line) in content.split(|&b| b == b'\n').enumerate() {
        if find_bytes(line, needle).is_some() {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            write_stdout(format!("{display}:{}:", index + 1).as_bytes());
            write_stdout(line);
            write_stdout(b"\n");
            *found += 1;
        }
    }
}
