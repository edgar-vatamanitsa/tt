use std::fs;

use super::Ctx;
use crate::error::{Context, Error, Result};
use crate::flags::{Args, Flag};
use crate::paths::TasksDir;
use crate::query::{Lexer, Query};
use crate::task::{load_tasks, sort_tasks};
use crate::term::outln;
use crate::text::trim;
use crate::update::{self, apply_update, compile_update};

pub const FLAGS: &[Flag] = &[
    Flag::bool("c", "List closed tasks"),
    Flag::bool("a", "List tasks in ascending order"),
    Flag::bool("id", "Sort tasks by id"),
    Flag::list("update", "Apply update operations on the resulting list"),
    Flag::bool("debug", "Output opcodes of the query for debug purpose"),
];

pub fn run(_ctx: &Ctx, args: &Args) -> Result<()> {
    let joined_query = args.rest.join(" ");
    let written_query = trim(&joined_query);
    let query_is_empty = written_query.is_empty();
    let query_src = if query_is_empty { "any" } else { written_query };
    let query = Query::compile(query_src)?;

    let joined_update = args.list("update").join(" ");
    let update_src = trim(&joined_update);
    let update = compile_update(update_src)?;

    if args.bool("debug") {
        print_debug(written_query, &query, update_src, &update);
        return Ok(());
    }

    if !update.is_empty() && query_is_empty {
        eprintln!("ERROR: -update with empty query is not allowed!");
        eprintln!("NOTE: if you do want to update all the tasks use \"any\" query");
        return Err(Error::Reported);
    }

    let db = TasksDir::find()?;
    let mut tasks = load_tasks(&db.abs)?;
    sort_tasks(&mut tasks, args.bool("id"));
    if args.bool("a") {
        tasks.reverse();
    }

    let closed = args.bool("c");
    let mut matched = 0;
    let mut updated = 0;
    for task in tasks.iter_mut().filter(|t| t.is_closed() == closed) {
        if !query.matches(task) {
            continue;
        }
        matched += 1;

        if !apply_update(task, &update) {
            outln!("{}", task.report(&db.rel, None));
            continue;
        }
        if !task.well_formed {
            eprintln!(
                "{}/{}/TASK.md:1: ERROR: not updating the task because its TASK.md doesn't start with `# <title>`",
                db.rel, task.id
            );
            continue;
        }
        let path = db.abs.join(&task.id).join("TASK.md");
        fs::write(&path, task.render()).context(format_args!("could not write {}", path.display()))?;
        outln!("{}", task.report(&db.rel, Some(update_src)));
        updated += 1;
    }

    if matched == 0 {
        outln!("No tasks were found");
    } else if updated > 0 {
        outln!("{updated} tasks were updated");
    }
    Ok(())
}

fn print_debug(query_src: &str, query: &Query, update_src: &str, update: &[update::UOp]) {
    outln!("QUERY TOKENS:");
    for token in Lexer::new(query_src) {
        outln!("    {}", token.text);
    }
    outln!();
    outln!("QUERY OPS:");
    for op in &query.ops {
        outln!("    {op}");
    }
    if !update.is_empty() {
        outln!();
        outln!("UPDATE TOKENS:");
        for token in update::tokens(update_src) {
            outln!("    {token}");
        }
        outln!();
        outln!("UPDATE OPS:");
        for op in update {
            outln!("    {op}");
        }
    }
}
