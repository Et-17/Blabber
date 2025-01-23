use std::path::PathBuf;
use std::fmt::Display;

pub trait ErrorType: Display + PartialEq {}

#[derive(Debug, PartialEq, Clone)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.file.display())
        } else {
            write!(f, "{}:{}", self.file.display(), self.line)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Error<T: ErrorType> {
    pub location: Location,
    pub error: T
}

impl<T: ErrorType> Display for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\x1b[31;49;1m[{}]\x1b[39;49;1m  {}\x1b[0m", self.location, self.error)
    }
}

pub type Errors<T> = Vec<Error<T>>;