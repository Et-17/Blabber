/*
    This module is for storing and manipulating grammars
*/

use std::collections::HashMap;

// The base unit in a grammar rule
#[derive(Debug)]
pub enum Symbol {
    Terminal(String),
    Nonterminal(String),
}

// The symbols in a single alternative
type Alternative = Vec<Symbol>;

// The alternatives of a rewrite rule
type Rule = Vec<Alternative>;

#[derive(Debug)]
pub struct Grammar {
    pub start_symbol: String,
    pub rules: HashMap<String, Rule>,
}

