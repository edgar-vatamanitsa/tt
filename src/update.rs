// The update language of `tt ls -update`: a comma/whitespace separated list of
// actions applied to every task selected by the query.
//
//   +tag   add tag
//   -tag   remove tag
//   +@name assign the task to name
//   -@name unassign name from the task
//   close  close the task
//   open   open the task

use crate::error::{Error, Result};
use crate::query::{Span, report_diagnostic};
use crate::task::Task;
use crate::text::split_list;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UOp {
    Close,
    Open,
    Tag(String),
    Untag(String),
    Assign(String),
    Unassign(String),
}

impl std::fmt::Display for UOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UOp::Close => write!(f, "UOP_CLOSE"),
            UOp::Open => write!(f, "UOP_OPEN"),
            UOp::Tag(tag) => write!(f, "UOP_TAG {tag}"),
            UOp::Untag(tag) => write!(f, "UOP_UNTAG {tag}"),
            UOp::Assign(name) => write!(f, "UOP_ASSIGN {name}"),
            UOp::Unassign(name) => write!(f, "UOP_UNASSIGN {name}"),
        }
    }
}

pub fn tokens(src: &str) -> impl Iterator<Item = &str> {
    split_list(src).map(|(_, token)| token)
}

pub fn compile_update(src: &str) -> Result<Vec<UOp>> {
    let mut ops = Vec::new();
    for (start, action) in split_list(src) {
        let span = Span { start, len: action.len() };
        let op = match action {
            "open" => UOp::Open,
            "close" => UOp::Close,
            "+" | "-" => {
                report_diagnostic(src, span, "ERROR: empty tag");
                return Err(Error::Reported);
            }
            "+@" | "-@" => {
                report_diagnostic(src, span, "ERROR: empty assignee");
                return Err(Error::Reported);
            }
            _ if action.starts_with("+@") => UOp::Assign(action[2..].to_string()),
            _ if action.starts_with("-@") => UOp::Unassign(action[2..].to_string()),
            _ if action.starts_with('+') => UOp::Tag(action[1..].to_string()),
            _ if action.starts_with('-') => UOp::Untag(action[1..].to_string()),
            _ => {
                eprintln!("Supported actions:");
                eprintln!();
                eprintln!("    +<tag>    - add tag to the task");
                eprintln!("    -<tag>    - remove tag from the task");
                eprintln!("    +@<name>  - assign the task to name");
                eprintln!("    -@<name>  - unassign name from the task");
                eprintln!("    close     - close the task");
                eprintln!("    open      - open the task");
                eprintln!();
                report_diagnostic(src, span, &format!("ERROR: Unknown action `{action}`"));
                return Err(Error::Reported);
            }
        };
        ops.push(op);
    }
    Ok(ops)
}

/// Returns whether the task actually changed.
pub fn apply_update(task: &mut Task, ops: &[UOp]) -> bool {
    let mut updated = false;
    for op in ops {
        match op {
            UOp::Close | UOp::Open => {
                let status = if *op == UOp::Close { "CLOSED" } else { "OPEN" };
                if task.status != status {
                    task.status = status.to_string();
                    updated = true;
                }
            }
            UOp::Tag(tag) => {
                if !task.has_tag(tag) {
                    task.tags.push(tag.clone());
                    updated = true;
                }
            }
            UOp::Untag(tag) => {
                let before = task.tags.len();
                task.tags.retain(|t| t != tag);
                updated |= task.tags.len() != before;
            }
            UOp::Assign(name) => {
                if !task.is_assigned_to(name) {
                    task.assignees.push(name.clone());
                    updated = true;
                }
            }
            UOp::Unassign(name) => {
                let before = task.assignees.len();
                task.assignees.retain(|a| a != name);
                updated |= task.assignees.len() != before;
            }
        }
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_actions() {
        let ops = compile_update("close,-scope +release").unwrap();
        assert_eq!(ops, [UOp::Close, UOp::Untag("scope".into()), UOp::Tag("release".into())]);
        assert!(compile_update("reopen").is_err());
        assert!(compile_update("+").is_err());
        assert!(compile_update("+@").is_err());
        assert_eq!(
            compile_update("+@alice,-@bob").unwrap(),
            [UOp::Assign("alice".into()), UOp::Unassign("bob".into())]
        );
    }

    #[test]
    fn applies_and_reports_changes() {
        let mut task = Task::new("x".into(), "T".into(), 1, vec!["a".into(), "b".into(), "c".into()]);
        assert!(apply_update(&mut task, &[UOp::Untag("a".into())]));
        assert_eq!(task.tags, ["b", "c"], "untag keeps the order of the other tags");
        assert!(!apply_update(&mut task, &[UOp::Open, UOp::Tag("b".into())]));
        assert!(apply_update(&mut task, &[UOp::Close]));
        assert!(task.is_closed());
    }
}
