use std::fs;
use std::io;
use std::thread;
use std::time::Duration;

use super::Ctx;
use crate::error::{Context, Result, fail};
use crate::flags::{Args, Flag};
use crate::huid::{is_valid_huid, new_huid};
use crate::paths::TasksDir;
use crate::task::{DEFAULT_TASK_TITLE, Task, load_tasks, parse_tags};
use crate::term::{flush_stdout, out, outln};
use crate::text::trim;

pub const FLAGS: &[Flag] = &[
    Flag::list("t", "Tags to add to the new task"),
    Flag::uint("p", DEFAULT_PRIORITY_FLAG, "Priority of the new task"),
    Flag::str("s", "Task ID optional suffix"),
    Flag::bool("y", "Always answer 'y' to any prompts of this command"),
];

// Flag defaults are strings; checked against DEFAULT_PRIORITY by the test below.
const DEFAULT_PRIORITY_FLAG: &str = "100";

pub fn run(_ctx: &Ctx, args: &Args) -> Result<()> {
    let joined_title = args.rest.join(" ");
    let title = match trim(&joined_title) {
        "" => DEFAULT_TASK_TITLE,
        title => title,
    };
    let tags = tags_from_flags(args.list("t"));
    let priority = i64::try_from(args.uint("p")).unwrap_or(i64::MAX);

    let db = TasksDir::find()?;

    if !args.bool("y") {
        let tasks = load_tasks(&db.abs)?;
        if let Some(existing) = tasks.iter().find(|t| t.title == title) {
            let question = format!("Task with this exact title already exists ({}). Create it anyway?", existing.id);
            match prompt(&question, &[('y', "yes"), ('n', "no")])? {
                Some('y') => {}
                _ => return Ok(()),
            }
        }
    }

    create_new_task(&db, args.str("s"), title, priority, tags)?;
    Ok(())
}

pub fn tags_from_flags(values: &[String]) -> Vec<String> {
    values.iter().flat_map(|v| parse_tags(v)).collect()
}

/// Creates `tasks/<HUID>/TASK.md`, prints its report line and returns the new HUID.
pub fn create_new_task(
    db: &TasksDir,
    suffix: Option<&str>,
    title: &str,
    priority: i64,
    tags: Vec<String>,
) -> Result<String> {
    let mut id = new_huid(suffix);
    if !is_valid_huid(&id) {
        fail!(
            "`{}` is not a valid task id suffix. It may only contain the characters [a-zA-Z0-9-]",
            suffix.unwrap_or("")
        );
    }
    let mut task_dir = db.abs.join(&id);
    if task_dir.exists() {
        // We hit an already existing HUID. Wait one second to acquire the next one.
        thread::sleep(Duration::from_secs(1));
        id = new_huid(suffix);
        task_dir = db.abs.join(&id);
        if task_dir.exists() {
            fail!("{id} already exists. You are probably creating tasks too fast, or your clock is broken");
        }
    }
    fs::create_dir(&task_dir).context(format_args!("could not create {}", task_dir.display()))?;

    let task = Task::new(id.clone(), title.to_string(), priority, tags);
    let task_md = task_dir.join("TASK.md");
    fs::write(&task_md, task.render()).context(format_args!("could not write {}", task_md.display()))?;
    outln!("{}", task.report(&db.rel, None));
    Ok(id)
}

/// Asks a single-letter question. Returns `None` when stdin is closed.
pub fn prompt(question: &str, options: &[(char, &str)]) -> Result<Option<char>> {
    loop {
        out!("{question} [");
        for (c, _) in options {
            out!("{c},");
        }
        out!("?] ");
        flush_stdout();

        let mut answer = String::new();
        let n = io::stdin().read_line(&mut answer).context("could not read stdin")?;
        if n == 0 {
            outln!();
            return Ok(None);
        }
        let answer = trim(&answer).to_lowercase();
        if answer == "?" {
            for (c, description) in options {
                outln!("{c} - {description}");
            }
            continue;
        }
        let mut chars = answer.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if options.iter().any(|(o, _)| *o == c) {
                return Ok(Some(c));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_priority_flag_matches_constant() {
        assert_eq!(super::DEFAULT_PRIORITY_FLAG, crate::task::DEFAULT_PRIORITY.to_string());
    }
}
