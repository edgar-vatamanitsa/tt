use std::io;

use super::{Ctx, find_command, print_available_commands, print_usage};
use crate::error::{Error, Result};
use crate::flags::{Args, Flag};
use crate::term::outln;

pub const HELP_FLAGS: &[Flag] = &[];
pub const VERSION_FLAGS: &[Flag] = &[];

pub fn help_run(ctx: &Ctx, args: &Args) -> Result<()> {
    let Some(name) = args.rest.first() else {
        outln!("tt - Task Tracker");
        outln!("Usage: {} <command> [OPTIONS]", ctx.program);
        outln!();
        let _ = print_available_commands(&mut io::stdout().lock(), &ctx.program);
        return Ok(());
    };
    match find_command(name) {
        Some(command) => {
            let _ = print_usage(&mut io::stdout().lock(), command, &ctx.program);
            Ok(())
        }
        None => {
            let _ = print_available_commands(&mut io::stderr(), &ctx.program);
            eprintln!();
            eprintln!("ERROR: Unknown command `{name}`");
            Err(Error::Reported)
        }
    }
}

pub fn version_run(_ctx: &Ctx, _args: &Args) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    match option_env!("TT_GIT_HASH") {
        Some(hash) => outln!("tt {version} (commit {hash})"),
        None => outln!("tt {version}"),
    }
    if let Some(time) = option_env!("TT_BUILD_TIME") {
        outln!("Build time: {time}");
    }
    if let Some(compiler) = option_env!("TT_COMPILER_VERSION") {
        outln!("Compiler: {compiler}");
    }
    outln!("Target: {}-{}", std::env::consts::ARCH, std::env::consts::OS);
    Ok(())
}
