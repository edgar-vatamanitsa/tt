// tt - Task Tracker
//
// A cross-platform Rust rewrite of tatr (https://github.com/tsoding/tatr)
// by Alexey Kutepov.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

mod commands;
mod error;
mod flags;
mod huid;
mod paths;
mod query;
mod task;
mod term;
mod text;
mod time;
mod update;

use std::io;
use std::path::Path;
use std::process::ExitCode;

use commands::{Ctx, find_command, print_available_commands, run_command};
use error::Error;

fn main() -> ExitCode {
    let mut args = std::env::args();
    // Show `tt` rather than `C:\...\tt.exe` in usage messages.
    let program = args
        .next()
        .and_then(|p| Path::new(&p).file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "tt".to_string());
    let ctx = Ctx { program };
    let args: Vec<String> = args.collect();

    let Some(name) = args.first() else {
        let _ = print_available_commands(&mut io::stderr(), &ctx.program);
        eprintln!();
        eprintln!("ERROR: No command is provided");
        return ExitCode::FAILURE;
    };
    let name = match name.as_str() {
        "-help" | "--help" | "-h" => "help",
        "-version" | "--version" | "-V" => "version",
        name => name,
    };

    let Some(command) = find_command(name) else {
        let _ = print_available_commands(&mut io::stderr(), &ctx.program);
        eprintln!();
        eprintln!("ERROR: Unknown command `{name}`");
        return ExitCode::FAILURE;
    };

    match run_command(command, &ctx, &args[1..]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Msg(message)) => {
            eprintln!("ERROR: {message}");
            ExitCode::FAILURE
        }
        Err(Error::Reported) => ExitCode::FAILURE,
    }
}
