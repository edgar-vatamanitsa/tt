use std::fs;
use std::path::Path;

use super::Ctx;
use crate::error::{Context, Result};
use crate::flags::{Args, Flag};
use crate::term::outln;

pub const FLAGS: &[Flag] = &[Flag::bool("no-readme", "Do not create default README.md in the ./tasks/ folder")];

const TASKS_README_MD: &str = include_str!("../../assets/tasks-readme.md");

pub fn run(_ctx: &Ctx, args: &Args) -> Result<()> {
    let tasks_dir = Path::new("tasks");
    if !tasks_dir.is_dir() {
        fs::create_dir_all(tasks_dir).context("could not create ./tasks/")?;
        outln!("Created ./tasks/");
    }

    let readme = tasks_dir.join("README.md");
    if !args.bool("no-readme") && !readme.exists() {
        fs::write(&readme, TASKS_README_MD).context("could not write ./tasks/README.md")?;
        outln!("Created ./tasks/README.md");
    }
    Ok(())
}
