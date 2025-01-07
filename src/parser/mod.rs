/*
    This module parses BNF files
*/

mod lexer;

use crate::grammar;

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    MissingEquals,
    UnexpectedEquals,
    DefinedTerminal,
    UnmatchedQuote
}

pub type Result<T> = std::result::Result<T, CompileError>;

struct Rule {
    symbol: String,
    rewrite: grammar::Rewrite,
}