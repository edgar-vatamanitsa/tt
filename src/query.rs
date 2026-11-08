// TQL - the query language of `tt ls`.
//
// <expr>       ::= <or>
// <or>         ::= <and> *('or' <and>)
// <and>        ::= <compare> *('and' <compare>)
// <compare>    ::= <primary> *(<compare-op> <primary>)
// <compare-op> ::= 'lt' | 'le' | 'gt' | 'ge' | 'eq' | 'ne'
// <primary>    ::= ':' <tag> | '[' <expr> ']' | 'not' <primary>
//                | '@' <assignee> | 'any' | 'tagged' | 'assigned'
//                | 'priority' | <number> | <huid>
//
// The query is compiled into a list of ops in reverse polish notation and
// evaluated on a stack for every task.

use crate::error::{Error, Result};
use crate::huid::is_valid_huid;
use crate::task::Task;
use crate::text::is_space;

/// A byte range inside the original query source, used for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    pub text: &'a str,
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src, pos: 0 }
    }

    /// Returns the next token, or an empty token positioned at the end of input.
    fn chop(&mut self) -> Token<'a> {
        let b = self.src.as_bytes();
        while self.pos < b.len() && is_space(b[self.pos]) {
            self.pos += 1;
        }
        let start = self.pos;
        if self.pos < b.len() && matches!(b[self.pos], b'[' | b']') {
            self.pos += 1;
        } else {
            while self.pos < b.len() && !is_space(b[self.pos]) && !matches!(b[self.pos], b'[' | b']') {
                self.pos += 1;
            }
        }
        Token { text: &self.src[start..self.pos], span: Span { start, len: self.pos - start } }
    }

    /// Consumes the next token only if it equals `text`.
    fn eat(&mut self, text: &str) -> Option<Token<'a>> {
        let saved = *self;
        let token = self.chop();
        if token.text == text {
            Some(token)
        } else {
            *self = saved;
            None
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let token = self.chop();
        (!token.text.is_empty()).then_some(token)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QOpKind {
    Any,
    Tag(String),
    Not,
    Or,
    And,
    Tagged,
    Assignee(String),
    Assigned,
    Id(String),
    Priority,
    Integer(i64),
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QOp {
    pub kind: QOpKind,
    pub span: Span,
}

impl std::fmt::Display for QOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            QOpKind::Any => write!(f, "QOP_ANY"),
            QOpKind::Tag(tag) => write!(f, "QOP_TAG {tag}"),
            QOpKind::Not => write!(f, "QOP_NOT"),
            QOpKind::Or => write!(f, "QOP_OR"),
            QOpKind::And => write!(f, "QOP_AND"),
            QOpKind::Tagged => write!(f, "QOP_TAGGED"),
            QOpKind::Assignee(name) => write!(f, "QOP_ASSIGNEE {name}"),
            QOpKind::Assigned => write!(f, "QOP_ASSIGNED"),
            QOpKind::Id(id) => write!(f, "QOP_ID {id}"),
            QOpKind::Priority => write!(f, "QOP_PRIORITY"),
            QOpKind::Integer(n) => write!(f, "QOP_INTEGER {n}"),
            QOpKind::Lt => write!(f, "QOP_LT"),
            QOpKind::Gt => write!(f, "QOP_GT"),
            QOpKind::Le => write!(f, "QOP_LTE"),
            QOpKind::Ge => write!(f, "QOP_GTE"),
            QOpKind::Eq => write!(f, "QOP_EQ"),
            QOpKind::Ne => write!(f, "QOP_NEQ"),
        }
    }
}

/// Prints the source with a caret under the offending token, followed by the message.
pub fn report_diagnostic(src: &str, span: Span, message: &str) {
    let column = src[..span.start].chars().count();
    eprintln!("{src}");
    eprintln!("{}^", " ".repeat(column));
    eprintln!("{message}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
    Boolean,
    Integer,
}

impl Type {
    fn name(self) -> &'static str {
        match self {
            Type::Boolean => "boolean",
            Type::Integer => "integer",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Query {
    pub ops: Vec<QOp>,
}

struct Compiler<'a> {
    src: &'a str,
    lexer: Lexer<'a>,
    ops: Vec<QOp>,
}

impl<'a> Compiler<'a> {
    fn emit(&mut self, kind: QOpKind, token: Token) {
        self.ops.push(QOp { kind, span: token.span });
    }

    fn error(&self, span: Span, message: &str) -> Error {
        report_diagnostic(self.src, span, message);
        Error::Reported
    }

    fn primary(&mut self) -> Result<()> {
        let token = self.lexer.chop();
        let text = token.text;

        // `.` used to refer to tags before. Still accepted for backward compatibility.
        if text.starts_with(':') || text.starts_with('.') {
            if text.starts_with('.') {
                report_diagnostic(
                    self.src,
                    token.span,
                    "WARNING: Using `.` to refer to tags is deprecated and will be removed in the future. Use `:` instead.",
                );
            }
            if text.len() == 1 {
                return Err(self.error(token.span, "ERROR: empty tag"));
            }
            self.emit(QOpKind::Tag(text[1..].to_string()), token);
            return Ok(());
        }

        if let Some(name) = text.strip_prefix('@') {
            if name.is_empty() {
                return Err(self.error(token.span, "ERROR: empty assignee"));
            }
            self.emit(QOpKind::Assignee(name.to_string()), token);
            return Ok(());
        }

        match text {
            "[" => {
                self.expr()?;
                let close = self.lexer.chop();
                if close.text != "]" {
                    return Err(self.error(close.span, "ERROR: Expected `]`."));
                }
            }
            "not" => {
                self.primary()?;
                self.emit(QOpKind::Not, token);
            }
            "any" => self.emit(QOpKind::Any, token),
            "tagged" => self.emit(QOpKind::Tagged, token),
            "assigned" => self.emit(QOpKind::Assigned, token),
            "priority" => self.emit(QOpKind::Priority, token),
            _ if is_valid_huid(text) => self.emit(QOpKind::Id(text.to_string()), token),
            _ => match text.parse::<i64>() {
                Ok(n) => self.emit(QOpKind::Integer(n), token),
                Err(_) => {
                    eprintln!("What are primary expressions:");
                    eprintln!();
                    eprintln!("    :<tag>         - checks if task has a tag");
                    eprintln!("    @<assignee>    - checks if task is assigned to someone");
                    eprintln!("    [ <expr> ]     - groups expressions (square brackets need no escaping in shells)");
                    eprintln!("    not <primary>  - negation of a primary expression");
                    eprintln!("    any            - expression that always returns true");
                    eprintln!("    tagged         - checks if a task is tagged");
                    eprintln!("    assigned       - checks if a task is assigned to anyone");
                    eprintln!("    priority       - priority of a task as an integer");
                    eprintln!("    <number>       - signed integer");
                    eprintln!("    <huid>         - valid id of a task");
                    eprintln!();
                    let message = if text.is_empty() {
                        "ERROR: Primary expression is expected here.".to_string()
                    } else {
                        format!("ERROR: Unexpected start of a primary expression `{text}`.")
                    };
                    return Err(self.error(token.span, &message));
                }
            },
        }
        Ok(())
    }

    fn compare(&mut self) -> Result<()> {
        self.primary()?;
        loop {
            let saved = self.lexer;
            let token = self.lexer.chop();
            let kind = match token.text {
                "lt" => QOpKind::Lt,
                "le" => QOpKind::Le,
                "gt" => QOpKind::Gt,
                "ge" => QOpKind::Ge,
                "eq" => QOpKind::Eq,
                "ne" => QOpKind::Ne,
                _ => {
                    self.lexer = saved;
                    return Ok(());
                }
            };
            self.primary()?;
            self.emit(kind, token);
        }
    }

    fn and(&mut self) -> Result<()> {
        self.compare()?;
        while let Some(token) = self.lexer.eat("and") {
            self.compare()?;
            self.emit(QOpKind::And, token);
        }
        Ok(())
    }

    fn or(&mut self) -> Result<()> {
        self.and()?;
        while let Some(token) = self.lexer.eat("or") {
            self.and()?;
            self.emit(QOpKind::Or, token);
        }
        Ok(())
    }

    fn expr(&mut self) -> Result<()> {
        self.or()
    }
}

/// Simulates the evaluation with types only, so type errors are reported once at
/// compile time instead of for every task.
fn type_check(src: &str, ops: &[QOp]) -> Result<()> {
    let mut stack: Vec<(Type, Span)> = Vec::new();
    let pop = |stack: &mut Vec<(Type, Span)>, expected: Type| -> Result<()> {
        let (actual, span) = stack.pop().expect("compiler emits well-formed RPN");
        if actual != expected {
            let message = format!("ERROR: Expected {} but got {}", expected.name(), actual.name());
            report_diagnostic(src, span, &message);
            return Err(Error::Reported);
        }
        Ok(())
    };
    for op in ops {
        let result = match op.kind {
            QOpKind::Any
            | QOpKind::Tag(_)
            | QOpKind::Tagged
            | QOpKind::Assignee(_)
            | QOpKind::Assigned
            | QOpKind::Id(_) => Type::Boolean,
            QOpKind::Priority | QOpKind::Integer(_) => Type::Integer,
            QOpKind::Not => {
                pop(&mut stack, Type::Boolean)?;
                Type::Boolean
            }
            QOpKind::Or | QOpKind::And => {
                pop(&mut stack, Type::Boolean)?;
                pop(&mut stack, Type::Boolean)?;
                Type::Boolean
            }
            QOpKind::Lt | QOpKind::Gt | QOpKind::Le | QOpKind::Ge | QOpKind::Eq | QOpKind::Ne => {
                pop(&mut stack, Type::Integer)?;
                pop(&mut stack, Type::Integer)?;
                Type::Boolean
            }
        };
        stack.push((result, op.span));
    }
    pop(&mut stack, Type::Boolean)
}

impl Query {
    pub fn compile(src: &str) -> Result<Query> {
        let mut compiler = Compiler { src, lexer: Lexer::new(src), ops: Vec::new() };
        compiler.expr()?;
        let rest = compiler.lexer.chop();
        if !rest.text.is_empty() {
            eprintln!("Supported infix operators:");
            eprintln!();
            eprintln!("    and  or                  - logical operators");
            eprintln!("    lt  le  gt  ge  eq  ne   - comparison operators");
            eprintln!();
            let message = format!("ERROR: Unexpected infix operator `{}`", rest.text);
            return Err(compiler.error(rest.span, &message));
        }
        type_check(src, &compiler.ops)?;
        Ok(Query { ops: compiler.ops })
    }

    pub fn matches(&self, task: &Task) -> bool {
        #[derive(Clone, Copy)]
        enum Value {
            Bool(bool),
            Int(i64),
        }
        fn pop_bool(stack: &mut Vec<Value>) -> bool {
            match stack.pop() {
                Some(Value::Bool(b)) => b,
                _ => unreachable!("query is type checked"),
            }
        }
        fn pop_int(stack: &mut Vec<Value>) -> i64 {
            match stack.pop() {
                Some(Value::Int(n)) => n,
                _ => unreachable!("query is type checked"),
            }
        }

        let mut stack = Vec::with_capacity(self.ops.len());
        for op in &self.ops {
            let value = match &op.kind {
                QOpKind::Any => Value::Bool(true),
                QOpKind::Tag(tag) => Value::Bool(task.has_tag(tag)),
                QOpKind::Tagged => Value::Bool(!task.tags.is_empty()),
                QOpKind::Assignee(name) => Value::Bool(task.is_assigned_to(name)),
                QOpKind::Assigned => Value::Bool(!task.assignees.is_empty()),
                QOpKind::Id(id) => Value::Bool(task.id == *id),
                QOpKind::Priority => Value::Int(task.priority),
                QOpKind::Integer(n) => Value::Int(*n),
                QOpKind::Not => Value::Bool(!pop_bool(&mut stack)),
                QOpKind::Or => {
                    let (a, b) = (pop_bool(&mut stack), pop_bool(&mut stack));
                    Value::Bool(a || b)
                }
                QOpKind::And => {
                    let (a, b) = (pop_bool(&mut stack), pop_bool(&mut stack));
                    Value::Bool(a && b)
                }
                cmp => {
                    let b = pop_int(&mut stack);
                    let a = pop_int(&mut stack);
                    Value::Bool(match cmp {
                        QOpKind::Lt => a < b,
                        QOpKind::Gt => a > b,
                        QOpKind::Le => a <= b,
                        QOpKind::Ge => a >= b,
                        QOpKind::Eq => a == b,
                        QOpKind::Ne => a != b,
                        _ => unreachable!(),
                    })
                }
            };
            stack.push(value);
        }
        pop_bool(&mut stack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(src: &str) -> Vec<String> {
        Query::compile(src).unwrap().ops.iter().map(|op| op.to_string()).collect()
    }

    fn task(priority: i64, tags: &[&str]) -> Task {
        Task::new("20260101-000000".into(), "T".into(), priority, tags.iter().map(|t| t.to_string()).collect())
    }

    #[test]
    fn tokenizes_brackets_without_spaces() {
        let tokens: Vec<_> = Lexer::new("not[:bug and :test] and :query").map(|t| t.text).collect();
        assert_eq!(tokens, ["not", "[", ":bug", "and", ":test", "]", "and", ":query"]);
    }

    #[test]
    fn and_binds_tighter_than_or() {
        assert_eq!(ops("tagged or :a and :b"), ["QOP_TAGGED", "QOP_TAG a", "QOP_TAG b", "QOP_AND", "QOP_OR"]);
    }

    #[test]
    fn evaluates_queries() {
        let bug = task(10, &["bug"]);
        let ui_bug = task(60, &["bug", "ui"]);
        let untagged = task(100, &[]);
        let q = Query::compile(":bug and not :ui").unwrap();
        assert!(q.matches(&bug) && !q.matches(&ui_bug) && !q.matches(&untagged));
        let q = Query::compile("not tagged").unwrap();
        assert!(q.matches(&untagged) && !q.matches(&bug));
        let q = Query::compile(":bug and priority lt 50").unwrap();
        assert!(q.matches(&bug) && !q.matches(&ui_bug));
        let q = Query::compile("priority ge -5 and priority ne 10").unwrap();
        assert!(!q.matches(&bug) && q.matches(&ui_bug));
        let q = Query::compile("20260101-000000").unwrap();
        assert!(q.matches(&bug));
    }

    #[test]
    fn evaluates_assignee_queries() {
        let mut alice_bug = task(10, &["bug"]);
        alice_bug.assignees = vec!["alice".into()];
        let nobody = task(10, &["bug"]);
        let q = Query::compile("@alice and :bug").unwrap();
        assert!(q.matches(&alice_bug) && !q.matches(&nobody));
        let q = Query::compile("not assigned").unwrap();
        assert!(!q.matches(&alice_bug) && q.matches(&nobody));
        assert_eq!(ops("@bob or assigned"), ["QOP_ASSIGNEE bob", "QOP_ASSIGNED", "QOP_OR"]);
    }

    #[test]
    fn rejects_bad_queries() {
        for src in [":", "@", "@a lt 1", "[:bug", ":bug lt 5", "priority", "foo", ":a :b", "not"] {
            assert!(Query::compile(src).is_err(), "{src} should not compile");
        }
    }
}
