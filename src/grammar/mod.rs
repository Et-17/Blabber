/*
    This module is for storing and manipulating grammars
*/

use std::collections::HashMap;

// The base unit in a grammar rule
#[derive(Debug, PartialEq)]
pub enum Symbol {
    Terminal(String),
    Nonterminal(String),
}

// The symbols in a single alternative
pub type Alternative = Vec<Symbol>;

// The alternatives of a rewrite rule
pub type Rewrite = Vec<Alternative>;

#[derive(Debug, PartialEq)]
pub struct Grammar {
    pub start_symbol: String,
    pub rules: HashMap<String, Rewrite>,
}

