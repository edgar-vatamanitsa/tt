use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::process::Command;

use super::Ctx;
use crate::error::{Context, Result, fail};
use crate::flags::{Args, Flag};
use crate::huid::huid_prefix_len;
use crate::paths::TasksDir;
use crate::task::load_tasks;
use crate::term::outln;

pub const FLAGS: &[Flag] = &[];

const DOT_PATH: &str = "graph.dot";
const OUT_PATH: &str = "graph.svg";

pub fn run(_ctx: &Ctx, _args: &Args) -> Result<()> {
    let db = TasksDir::find()?;
    let tasks = load_tasks(&db.abs)?;

    // BTree collections keep the generated file stable between runs.
    let mut graph: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for task in &tasks {
        let refs = graph.entry(&task.id).or_default();
        let bytes = task.content.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match huid_prefix_len(&bytes[i..]) {
                Some(len) => {
                    // HUIDs are pure ASCII, so these are char boundaries.
                    let huid = &task.content[i..i + len];
                    if db.abs.join(huid).join("TASK.md").is_file() {
                        refs.insert(huid);
                    }
                    i += len;
                }
                None => i += 1,
            }
        }
    }

    let mut dot = String::from("digraph {\n");
    for (from, refs) in &graph {
        for to in refs {
            dot += &format!("    \"{from}\" -> \"{to}\";\n");
        }
    }
    dot += "}\n";
    fs::write(DOT_PATH, dot).context(format_args!("could not write {DOT_PATH}"))?;
    outln!("Generated {DOT_PATH}");

    let status = Command::new("neato").args(["-Goverlap=scale", "-Tsvg", DOT_PATH, &format!("-o{OUT_PATH}")]).status();
    match status {
        Ok(status) if status.success() => {
            outln!("Generated {OUT_PATH}");
            Ok(())
        }
        Ok(status) => fail!("neato exited with {status}"),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            fail!("could not find `neato`. Install Graphviz (https://graphviz.org/download/) to render {DOT_PATH}")
        }
        Err(e) => fail!("could not run neato: {e}"),
    }
}
