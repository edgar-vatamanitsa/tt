use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    /// A message that `main` prints as `ERROR: <message>`.
    Msg(String),
    /// The diagnostic was already printed where the problem was detected.
    Reported,
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! fail {
    ($($arg:tt)*) => {
        return Err($crate::error::Error::Msg(format!($($arg)*)))
    };
}
pub(crate) use fail;

pub trait Context<T> {
    fn context(self, what: impl Display) -> Result<T>;
}

impl<T> Context<T> for std::io::Result<T> {
    fn context(self, what: impl Display) -> Result<T> {
        self.map_err(|e| Error::Msg(format!("{what}: {e}")))
    }
}
