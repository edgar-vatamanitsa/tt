use std::fmt;
use std::io::{self, IsTerminal, Write};

/// `println!` that exits quietly when stdout is closed (e.g. `tt ls | head`)
/// instead of panicking.
macro_rules! outln {
    () => {
        $crate::term::write_stdout(b"\n")
    };
    ($($arg:tt)*) => {
        $crate::term::write_stdout_fmt(format_args!("{}\n", format_args!($($arg)*)))
    };
}
pub(crate) use outln;

macro_rules! out {
    ($($arg:tt)*) => {
        $crate::term::write_stdout_fmt(format_args!($($arg)*))
    };
}
pub(crate) use out;

fn handle_stdout_error(result: io::Result<()>) {
    if let Err(e) = result {
        if e.kind() == io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("ERROR: could not write to stdout: {e}");
        std::process::exit(1);
    }
}

pub fn write_stdout_fmt(args: fmt::Arguments) {
    handle_stdout_error(io::stdout().lock().write_fmt(args));
}

pub fn write_stdout(bytes: &[u8]) {
    handle_stdout_error(io::stdout().lock().write_all(bytes));
}

pub fn flush_stdout() {
    handle_stdout_error(io::stdout().flush());
}

/// Whether we may print ANSI colors to stdout. Honors https://no-color.org and
/// enables virtual terminal processing on Windows consoles.
pub fn stdout_supports_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    io::stdout().is_terminal() && enable_ansi()
}

#[cfg(windows)]
fn enable_ansi() -> bool {
    use std::os::windows::io::AsRawHandle;

    type Handle = *mut core::ffi::c_void;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleMode(handle: Handle, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
    }

    let handle = io::stdout().as_raw_handle() as Handle;
    let mut mode = 0u32;
    // SAFETY: `handle` is the process's stdout handle and `mode` is a valid out pointer.
    unsafe {
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0
            || SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

#[cfg(not(windows))]
fn enable_ansi() -> bool {
    true
}
