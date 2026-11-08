use std::fs;

use super::new::{create_new_task, prompt, tags_from_flags};
use super::{Ctx, usage_error};
use crate::error::{Context, Result};
use crate::flags::{Args, Flag};
use crate::paths::TasksDir;
use crate::task::DEFAULT_PRIORITY;
use crate::term::{outln, stdout_supports_color, write_stdout};
use crate::text::{find_bytes, trim_bytes};

pub const FLAGS: &[Flag] = &[Flag::list("t", "Tags to add to the new task"), Flag::str("s", "Task ID optional suffix")];

// Spelled in pieces so running `tt todo` on this very file doesn't find it.
const MARKER_WORD: &str = concat!("TO", "DO");
const MARKER: &str = concat!("TO", "DO:");

const OPTIONS: &[(char, &str)] =
    &[('y', "yes"), ('n', "no"), ('q', "quit"), ('a', "yes until the end of file"), ('c', "no until the end of file")];

/// Splits like the original tool: a trailing newline does not produce an empty last line.
fn split_lines(input: &[u8]) -> Vec<Vec<u8>> {
    let mut lines: Vec<Vec<u8>> = input.split(|&b| b == b'\n').map(<[u8]>::to_vec).collect();
    if input.is_empty() || input.ends_with(b"\n") {
        lines.pop();
    }
    lines
}

pub fn run(ctx: &Ctx, args: &Args) -> Result<()> {
    if args.rest.is_empty() {
        return Err(usage_error(ctx, "todo", "no files were provided"));
    }
    let tags = tags_from_flags(args.list("t"));
    let use_color = stdout_supports_color();
    let db = TasksDir::find()?;
    let priority = DEFAULT_PRIORITY as i64;

    for file_path in &args.rest {
        // Source files are handled as raw bytes so non-UTF-8 files and CRLF line
        // endings survive the rewrite untouched.
        let input = fs::read(file_path).context(format_args!("could not read {file_path}"))?;
        let mut lines = split_lines(&input);
        let mut yes_until_end = false;

        for i in 0..lines.len() {
            let line = lines[i].clone();
            let Some(index) = find_bytes(&line, MARKER.as_bytes()) else { continue };
            let prefix = &line[..index];
            let suffix = &line[index + MARKER.len()..];
            let title = String::from_utf8_lossy(trim_bytes(suffix)).into_owned();

            outln!("{file_path}:{}: Found a {MARKER} {title}", i + 1);
            for (j, context_line) in lines.iter().enumerate().take(i + 3).skip(i.saturating_sub(2)) {
                if j != i {
                    write_stdout(context_line);
                } else if use_color {
                    write_stdout(prefix);
                    write_stdout(format!("\x1b[1;31m{MARKER_WORD}\x1b[0m:\x1b[36m").as_bytes());
                    write_stdout(suffix);
                    write_stdout(b"\x1b[0m");
                } else {
                    write_stdout(&line);
                }
                write_stdout(b"\n");
            }

            if !yes_until_end {
                match prompt("Create a task from it?", OPTIONS)? {
                    Some('y') => {}
                    Some('n') => continue,
                    Some('a') => yes_until_end = true,
                    Some('c') => break,
                    _ => return Ok(()),
                }
            }

            let id = create_new_task(&db, args.str("s"), &title, priority, tags.clone())?;
            let mut replaced = prefix.to_vec();
            replaced.extend_from_slice(format!("TASK({id}):").as_bytes());
            replaced.extend_from_slice(suffix);
            lines[i] = replaced;

            // Save after every task so quitting midway never loses a created reference.
            let mut output = Vec::with_capacity(input.len() + 32);
            for line in &lines {
                output.extend_from_slice(line);
                output.push(b'\n');
            }
            fs::write(file_path, output).context(format_args!("could not write {file_path}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::split_lines;

    #[test]
    fn splits_lines_like_the_original() {
        assert_eq!(split_lines(b"a\nb\n"), [b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(split_lines(b"a\n\nb"), [b"a".to_vec(), b"".to_vec(), b"b".to_vec()]);
        assert!(split_lines(b"").is_empty());
    }
}
