// A tiny command line flag parser with the same conventions as tatr's flag.h:
//
//   -name            boolean flag
//   -name value      string/integer/list flag (also `-name=value`)
//   -name a -name b  list flags accumulate
//   -/name ...       parsed but ignored (handy to comment out a flag)
//   --               end of flags
//
// `--name` is accepted as an alias of `-name`. Parsing stops at the first positional
// argument unless the command asks for interleaved flags.

use std::fmt;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagKind {
    Bool,
    Str,
    Uint,
    List,
}

#[derive(Debug, Clone, Copy)]
pub struct Flag {
    pub name: &'static str,
    pub kind: FlagKind,
    pub desc: &'static str,
    pub default: Option<&'static str>,
}

impl Flag {
    pub const fn bool(name: &'static str, desc: &'static str) -> Flag {
        Flag { name, kind: FlagKind::Bool, desc, default: None }
    }

    pub const fn str(name: &'static str, desc: &'static str) -> Flag {
        Flag { name, kind: FlagKind::Str, desc, default: None }
    }

    pub const fn uint(name: &'static str, default: &'static str, desc: &'static str) -> Flag {
        Flag { name, kind: FlagKind::Uint, desc, default: Some(default) }
    }

    pub const fn list(name: &'static str, desc: &'static str) -> Flag {
        Flag { name, kind: FlagKind::List, desc, default: None }
    }
}

pub const HELP: Flag = Flag::bool("help", "Print this help message");

#[derive(Debug, Clone)]
enum Value {
    Bool(bool),
    Str(Option<String>),
    Uint(u64),
    List(Vec<String>),
}

#[derive(Debug)]
pub struct Args {
    values: Vec<(&'static str, Value)>,
    pub rest: Vec<String>,
}

impl Args {
    fn value(&self, name: &str) -> &Value {
        self.values
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("flag -{name} is not defined for this command"))
    }

    pub fn bool(&self, name: &str) -> bool {
        matches!(self.value(name), Value::Bool(true))
    }

    pub fn str(&self, name: &str) -> Option<&str> {
        match self.value(name) {
            Value::Str(s) => s.as_deref(),
            _ => None,
        }
    }

    pub fn uint(&self, name: &str) -> u64 {
        match self.value(name) {
            Value::Uint(n) => *n,
            _ => 0,
        }
    }

    pub fn list(&self, name: &str) -> &[String] {
        match self.value(name) {
            Value::List(items) => items,
            _ => &[],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FlagError {
    Unknown(String),
    NoValue(String),
    InvalidNumber(String),
    IntegerOverflow(String),
}

impl fmt::Display for FlagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlagError::Unknown(name) => write!(f, "ERROR: -{name}: unknown flag"),
            FlagError::NoValue(name) => write!(f, "ERROR: -{name}: no value provided"),
            FlagError::InvalidNumber(name) => write!(f, "ERROR: -{name}: invalid number"),
            FlagError::IntegerOverflow(name) => write!(f, "ERROR: -{name}: integer overflow"),
        }
    }
}

pub fn parse(flags: &[Flag], args: &[String], interleaved: bool) -> Result<Args, FlagError> {
    let mut values: Vec<(&'static str, Value)> = flags
        .iter()
        .map(|flag| {
            let value = match flag.kind {
                FlagKind::Bool => Value::Bool(false),
                FlagKind::Str => Value::Str(flag.default.map(str::to_string)),
                FlagKind::Uint => Value::Uint(flag.default.and_then(|d| d.parse().ok()).unwrap_or(0)),
                FlagKind::List => Value::List(Vec::new()),
            };
            (flag.name, value)
        })
        .collect();
    let mut rest = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--" {
            rest.extend(iter.cloned());
            break;
        }
        if !arg.starts_with('-') || arg == "-" {
            rest.push(arg.clone());
            if interleaved {
                continue;
            }
            rest.extend(iter.cloned());
            break;
        }

        let mut name = arg.strip_prefix("--").unwrap_or(&arg[1..]);
        let ignore = name.starts_with('/');
        if ignore {
            name = &name[1..];
        }
        let (name, inline_value) = match name.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (name, None),
        };

        let Some(index) = flags.iter().position(|f| f.name == name) else {
            return Err(FlagError::Unknown(name.to_string()));
        };
        let flag = &flags[index];
        if flag.kind == FlagKind::Bool {
            if !ignore {
                values[index].1 = Value::Bool(true);
            }
            continue;
        }

        let value = match inline_value {
            Some(value) => value,
            None => iter.next().cloned().ok_or_else(|| FlagError::NoValue(name.to_string()))?,
        };
        let parsed = match flag.kind {
            FlagKind::Str => Value::Str(Some(value)),
            FlagKind::Uint => Value::Uint(value.parse().map_err(|e: std::num::ParseIntError| match e.kind() {
                std::num::IntErrorKind::PosOverflow => FlagError::IntegerOverflow(name.to_string()),
                _ => FlagError::InvalidNumber(name.to_string()),
            })?),
            FlagKind::List => {
                let Value::List(mut items) = values[index].1.clone() else { unreachable!() };
                items.push(value);
                Value::List(items)
            }
            FlagKind::Bool => unreachable!(),
        };
        if !ignore {
            values[index].1 = parsed;
        }
    }

    Ok(Args { values, rest })
}

pub fn print_options(sink: &mut dyn Write, flags: &[Flag]) -> std::io::Result<()> {
    for flag in flags {
        match flag.kind {
            FlagKind::List => writeln!(sink, "    -{0} <str> ... -{0} <str> ...", flag.name)?,
            FlagKind::Bool => writeln!(sink, "    -{}", flag.name)?,
            FlagKind::Uint => writeln!(sink, "    -{} <int>", flag.name)?,
            FlagKind::Str => writeln!(sink, "    -{} <str>", flag.name)?,
        }
        writeln!(sink, "        {}", flag.desc)?;
        if let Some(default) = flag.default {
            writeln!(sink, "        Default: {default}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAGS: &[Flag] = &[
        Flag::bool("c", "closed"),
        Flag::list("t", "tags"),
        Flag::uint("p", "100", "priority"),
        Flag::str("s", "suffix"),
    ];

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_flags_and_rest() {
        let a = parse(FLAGS, &args(&["-c", "-t", "a,b", "-t=c", "-p", "69", "hello", "-c"]), false).unwrap();
        assert!(a.bool("c"));
        assert_eq!(a.list("t"), ["a,b", "c"]);
        assert_eq!(a.uint("p"), 69);
        assert_eq!(a.str("s"), None);
        assert_eq!(a.rest, ["hello", "-c"]);
    }

    #[test]
    fn defaults_double_dash_and_ignore() {
        let a = parse(FLAGS, &args(&["--s", "x", "-/p", "5", "--", "-1"]), false).unwrap();
        assert_eq!(a.str("s"), Some("x"));
        assert_eq!(a.uint("p"), 100);
        assert_eq!(a.rest, ["-1"]);
    }

    #[test]
    fn interleaved() {
        let a = parse(FLAGS, &args(&["one", "-c", "two"]), true).unwrap();
        assert!(a.bool("c"));
        assert_eq!(a.rest, ["one", "two"]);
    }

    #[test]
    fn errors() {
        assert_eq!(parse(FLAGS, &args(&["-x"]), false).unwrap_err(), FlagError::Unknown("x".into()));
        assert_eq!(parse(FLAGS, &args(&["-t"]), false).unwrap_err(), FlagError::NoValue("t".into()));
        assert_eq!(parse(FLAGS, &args(&["-p", "x"]), false).unwrap_err(), FlagError::InvalidNumber("p".into()));
        assert_eq!(
            parse(FLAGS, &args(&["-p", "99999999999999999999"]), false).unwrap_err(),
            FlagError::IntegerOverflow("p".into())
        );
    }
}
