mod find;
mod graph;
mod help;
mod init;
mod ls;
mod new;
mod summary;
mod todo;

use std::io::{self, Write};

use crate::error::{Error, Result};
use crate::flags::{self, Args, Flag, HELP};

pub struct Ctx {
    /// How the user invoked us, used in usage messages.
    pub program: String,
}

pub struct Command {
    pub name: &'static str,
    pub signature: &'static str,
    pub description: &'static str,
    pub extra: Option<&'static str>,
    pub flags: &'static [Flag],
    /// Allow flags after positional arguments (`tt find <HUID> -path-only`).
    pub interleaved: bool,
    pub run: fn(&Ctx, &Args) -> Result<()>,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "init",
        signature: "[OPTIONS]",
        description: "Create tasks/ directory in the current working directory if it doesn't exist yet.",
        extra: None,
        flags: init::FLAGS,
        interleaved: false,
        run: init::run,
    },
    Command {
        name: "ls",
        signature: "[OPTIONS] [QUERY...]",
        description: "List and update the tasks.",
        extra: Some("See the README for the query language (TQL) and the update language."),
        flags: ls::FLAGS,
        interleaved: false,
        run: ls::run,
    },
    Command {
        name: "new",
        signature: "[OPTIONS] [TITLE...]",
        description: "Create a new task.",
        extra: None,
        flags: new::FLAGS,
        interleaved: false,
        run: new::run,
    },
    Command {
        name: "todo",
        signature: "[OPTIONS] <FILE...>",
        description: "Create tasks from all the TODOs in the provided files.",
        extra: Some("Each converted TODO is replaced with a TASK(<HUID>) reference."),
        flags: todo::FLAGS,
        interleaved: false,
        run: todo::run,
    },
    Command {
        name: "summary",
        signature: "[OPTIONS]",
        description: "Print the summary of the tasks.",
        extra: None,
        flags: summary::FLAGS,
        interleaved: false,
        run: summary::run,
    },
    Command {
        name: "find",
        signature: "[OPTIONS] <HUID>",
        description: "Find the task with a given HUID.",
        extra: Some("Unlike `tt ls <HUID>` this command finds the task regardless of whether it's closed or not."),
        flags: find::FIND_FLAGS,
        interleaved: true,
        run: find::find_run,
    },
    Command {
        name: "id",
        signature: "[OPTIONS]",
        description: "Print the id of the current task folder to stdout.",
        extra: Some(
            "Pipe it to your clipboard: `tt id | pbcopy` (macOS), `tt id | clip` (Windows), `tt id | xclip` (Linux).",
        ),
        flags: find::ID_FLAGS,
        interleaved: false,
        run: find::id_run,
    },
    Command {
        name: "ref",
        signature: "[OPTIONS] [HUID]",
        description: "Finds referers of the task.",
        extra: Some(
            "Searches for the Task ID over the entirety of the repo (skipping .git and binary files). If ran inside of a task folder automatically picks up the ID of that task.",
        ),
        flags: find::REF_FLAGS,
        interleaved: true,
        run: find::ref_run,
    },
    Command {
        name: "graph",
        signature: "[OPTIONS]",
        description: "Generate graph of tasks cross-referring to each other.",
        extra: Some("Writes graph.dot and renders it to graph.svg with Graphviz's `neato`."),
        flags: graph::FLAGS,
        interleaved: false,
        run: graph::run,
    },
    Command {
        name: "help",
        signature: "[OPTIONS] [COMMAND]",
        description: "Print this help message.",
        extra: None,
        flags: help::HELP_FLAGS,
        interleaved: false,
        run: help::help_run,
    },
    Command {
        name: "version",
        signature: "[OPTIONS]",
        description: "Print the current version and date of the build.",
        extra: None,
        flags: help::VERSION_FLAGS,
        interleaved: false,
        run: help::version_run,
    },
];

pub fn find_command(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

fn all_flags(command: &Command) -> Vec<Flag> {
    command.flags.iter().copied().chain([HELP]).collect()
}

pub fn print_usage(sink: &mut dyn Write, command: &Command, program: &str) -> io::Result<()> {
    writeln!(sink, "Usage:")?;
    writeln!(sink, "    {program} {} {}", command.name, command.signature)?;
    writeln!(sink)?;
    writeln!(sink, "Description:")?;
    match command.extra {
        Some(extra) => writeln!(sink, "    {} {extra}", command.description)?,
        None => writeln!(sink, "    {}", command.description)?,
    }
    writeln!(sink)?;
    writeln!(sink, "OPTIONS:")?;
    flags::print_options(sink, &all_flags(command))?;
    writeln!(sink)
}

pub fn print_available_commands(sink: &mut dyn Write, program: &str) -> io::Result<()> {
    let width = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    writeln!(sink, "Available commands:")?;
    for command in COMMANDS {
        writeln!(sink, "    {:<width$} - {}", command.name, command.description)?;
    }
    writeln!(sink)?;
    writeln!(sink, "Use `{program} <COMMAND> -help` to learn more about a command.")
}

/// Prints the usage of `command_name` followed by the error, like a bad flag would.
pub fn usage_error(ctx: &Ctx, command_name: &str, message: &str) -> Error {
    if let Some(command) = find_command(command_name) {
        let _ = print_usage(&mut io::stderr(), command, &ctx.program);
    }
    eprintln!("ERROR: {message}");
    Error::Reported
}

pub fn run_command(command: &Command, ctx: &Ctx, args: &[String]) -> Result<()> {
    match flags::parse(&all_flags(command), args, command.interleaved) {
        Err(e) => {
            let _ = print_usage(&mut io::stderr(), command, &ctx.program);
            eprintln!("{e}");
            Err(Error::Reported)
        }
        Ok(args) if args.bool("help") => {
            let mut stdout = io::stdout().lock();
            let _ = print_usage(&mut stdout, command, &ctx.program);
            Ok(())
        }
        Ok(args) => (command.run)(ctx, &args),
    }
}
