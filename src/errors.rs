//! Errors generated by the compiler.

use std::rc::Rc;

use itertools::Itertools;

use crate::{core, core::Id, frontend};

/// A span of the input program.
/// Used for reporting location-based errors.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Reference to input program source.
    input: Rc<str>,
    /// Name of the input file
    file: Rc<str>,
    /// The start of the span.
    start: usize,
    /// The end of the span.
    end: usize,
}

impl Span {
    /// Create a new `Error::Span` from a `pest::Span` and
    /// the input string.
    pub fn new(span: pest::Span, file: Rc<str>, input: Rc<str>) -> Span {
        Span {
            input,
            file,
            start: span.start(),
            end: span.end(),
        }
    }

    /// Returns the
    /// 1. lines associated with this span
    /// 2. start position of the first line in span
    /// 3. line number of the span
    fn get_lines(&self) -> (Vec<&str>, usize, usize) {
        let lines = self.input.split('\n').collect_vec();
        let mut pos: usize = 0;
        let mut linum: usize = 1;
        let mut collect_lines = false;
        let mut buf = Vec::new();

        let mut out_line: usize = 0;
        let mut out_idx: usize = 0;
        for l in lines {
            let next_pos = pos + l.len();
            if self.start >= pos && self.start <= next_pos {
                out_line = linum;
                out_idx = pos;
                collect_lines = true;
            }
            if collect_lines && self.end >= pos {
                buf.push(l)
            }
            if self.end <= next_pos {
                break;
            }
            pos = next_pos + 1;
            linum += 1;
        }
        (buf, out_idx, out_line)
    }

    /// Format this Span with a the error message `err_msg`
    pub fn format(&self, err_msg: &str) -> String {
        let (lines, pos, linum) = self.get_lines();
        let mut buf = self.file.to_string();

        let l = lines[0];
        let linum_text = format!("{} ", linum);
        let linum_space: String = " ".repeat(linum_text.len());
        let mark: String = "^".repeat(std::cmp::min(
            self.end - self.start,
            l.len() - (self.start - pos),
        ));
        let space: String = " ".repeat(self.start - pos);
        buf += "\n";
        buf += &format!("{}|{}\n", linum_text, l);
        buf += &format!("{}|{}{} {}", linum_space, space, mark, err_msg);
        buf
    }

    /// Visualizes the span without any message or mkaring
    pub fn show(&self) -> String {
        let (lines, _, linum) = self.get_lines();
        let l = lines[0];
        let linum_text = format!("{} ", linum);
        format!("{}|{}\n", linum_text, l)
    }
}

/// An IR node that may contain position information.
pub trait WithPos {
    /// Add span information to this node
    fn set_span(self, sp: Option<Span>) -> Self;

    /// Copy the span associated with this node.
    fn copy_span(&self) -> Option<Span>;
}

pub struct Error {
    kind: ErrorKind,
    pos: Option<Span>,
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.pos {
            None => write!(f, "{}", self.kind)?,
            Some(pos) => write!(f, "{}", pos.format(&self.kind.to_string()))?,
        }
        Ok(())
    }
}

impl Error {
    pub fn with_pos(mut self, pos: Option<Span>) -> Self {
        self.pos = pos;
        self
    }

    pub fn parse_error(err: pest_consume::Error<frontend::Rule>) -> Self {
        Self {
            kind: ErrorKind::ParseError(err),
            pos: None,
        }
    }

    pub fn invalid_file(f: String) -> Self {
        Self {
            kind: ErrorKind::InvalidFile(f),
            pos: None,
        }
    }

    pub fn write_error(e: String) -> Self {
        Self {
            kind: ErrorKind::WriteError(e),
            pos: None,
        }
    }

    pub fn malformed<S: ToString>(msg: S) -> Self {
        Self {
            kind: ErrorKind::Malformed(msg.to_string()),
            pos: None,
        }
    }

    pub fn undefined(name: Id, kind: String) -> Self {
        Self {
            kind: ErrorKind::Undefined(name, kind),
            pos: None,
        }
    }

    pub fn already_bound(name: Id, kind: String) -> Self {
        Self {
            kind: ErrorKind::AlreadyBound(name, kind),
            pos: None,
        }
    }

    pub fn cannot_prove(fact: core::Constraint<core::FsmIdxs>) -> Self {
        Self {
            kind: ErrorKind::CannotProve(fact),
            pos: None,
        }
    }

    pub fn misc(msg: String) -> Self {
        Self {
            kind: ErrorKind::Misc(msg),
            pos: None,
        }
    }
}

/// Standard error type for Calyx errors.
#[allow(clippy::large_enum_variant)]
enum ErrorKind {
    /// Error while parsing a Calyx program.
    ParseError(pest_consume::Error<frontend::Rule>),
    /// The input file is invalid (does not exist).
    InvalidFile(String),
    /// Failed to write the output
    WriteError(String),

    /// The program is malformed
    Malformed(String),

    /// The name has not been bound
    Undefined(Id, String),
    /// The name has already been bound.
    AlreadyBound(Id, String),

    /// Failed to prove a fact
    CannotProve(core::Constraint<core::FsmIdxs>),

    /// A miscellaneous error. Should be replaced with a more precise error.
    #[allow(unused)]
    Misc(String),
}

/// Convience wrapper to represent success or meaningul compiler error.
pub type FilamentResult<T> = std::result::Result<T, Error>;

/// A span of the input program.
impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ErrorKind::*;
        match self {
            AlreadyBound(name, bound_by) => {
                write!(f, "Name `{}' is already bound by {}", name, bound_by)
            }
            Undefined(name, typ) => {
                write!(f, "Undefined {} name: {}", typ, name)
            }
            ParseError(err) => write!(f, "Filament Parser: {}", err),
            InvalidFile(msg) | Misc(msg) | WriteError(msg) => {
                write!(f, "{}", msg)
            }
            CannotProve(fact) => {
                write!(f, "Cannot prove fact {:?}", fact)
            }
            Malformed(msg) => write!(f, "{msg}"),
        }
    }
}

// Conversions from other error types to our error type so that
// we can use `?` in all the places.

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::invalid_file(err.to_string())
    }
}

impl From<pest_consume::Error<frontend::Rule>> for Error {
    fn from(e: pest_consume::Error<frontend::Rule>) -> Self {
        Error::parse_error(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(_e: std::io::Error) -> Self {
        Error::write_error("IO Error".to_string())
    }
}

impl From<rsmt2::errors::Error> for Error {
    fn from(e: rsmt2::errors::Error) -> Self {
        Error::misc(format!("SMT Error: {}", e))
    }
}
