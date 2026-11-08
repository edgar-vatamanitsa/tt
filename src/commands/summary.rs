use std::collections::HashMap;
use std::fs;

use super::Ctx;
use crate::error::{Context, Result};
use crate::flags::{Args, Flag};
use crate::paths::TasksDir;
use crate::task::load_tasks;
use crate::term::outln;
use crate::text::{is_space, trim, trim_end};

pub const FLAGS: &[Flag] = &[Flag::bool("c", "Summarize closed tasks")];

/// Parses the optional `tasks/tags` file: `<tag-name> [,] <tag-description>` per line.
fn parse_tags_file(content: &str, display_path: &str) -> HashMap<String, String> {
    let is_sep = |b: u8| is_space(b) || b == b',';
    let mut descriptions = HashMap::new();
    for (index, line) in content.split('\n').enumerate() {
        let line = trim(line);
        if line.is_empty() {
            continue;
        }
        let tag_end = line.bytes().position(is_sep).unwrap_or(line.len());
        let desc_start = line[tag_end..].bytes().position(|b| !is_sep(b)).map_or(line.len(), |n| tag_end + n);
        let tag = &line[..tag_end];
        let desc = trim_end(&line[desc_start..]);
        if descriptions.insert(tag.to_string(), desc.to_string()).is_some() {
            eprintln!("{display_path}:{}: WARNING: redefinition of tag description '{tag}'", index + 1);
        }
    }
    descriptions
}

pub fn run(_ctx: &Ctx, args: &Args) -> Result<()> {
    let closed = args.bool("c");
    let db = TasksDir::find()?;
    let tasks = load_tasks(&db.abs)?;

    let tags_path = db.abs.join("tags");
    let descriptions = if tags_path.is_file() {
        let content = fs::read(&tags_path).context(format_args!("could not read {}", tags_path.display()))?;
        parse_tags_file(&String::from_utf8_lossy(&content), &format!("{}/tags", db.rel))
    } else {
        HashMap::new()
    };

    let mut total = 0;
    let mut untagged = 0;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for task in tasks.iter().filter(|t| t.is_closed() == closed) {
        total += 1;
        if task.tags.is_empty() {
            untagged += 1;
        }
        for tag in &task.tags {
            *counts.entry(tag).or_default() += 1;
        }
    }

    let mut counts: Vec<(&str, usize)> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let width = counts.iter().map(|(tag, _)| tag.chars().count()).max().unwrap_or(0);

    outln!("STATUS:   {}", if closed { "CLOSED" } else { "OPEN" });
    outln!("TOTAL:    {total}");
    if untagged > 0 {
        outln!("UNTAGGED: {untagged}");
    }
    if !counts.is_empty() {
        outln!("TAGGED:");
        for (tag, count) in counts {
            match descriptions.get(tag) {
                Some(desc) => outln!("    {tag:>width$} => {count:3} - {desc}"),
                None => outln!("    {tag:>width$} => {count:3}"),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_tags_file;

    #[test]
    fn parses_tags_file() {
        let tags = parse_tags_file("bug , unintended behavior\n\ntql,, Query Language  \nspec\n", "tags");
        assert_eq!(tags["bug"], "unintended behavior");
        assert_eq!(tags["tql"], "Query Language");
        assert_eq!(tags["spec"], "");
    }
}
