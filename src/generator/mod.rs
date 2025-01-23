/*
    This module generates sentences
*/

use rand::prelude::*;
use std::path::PathBuf;
use std::{collections::HashMap, fmt::Display};

use crate::grammar::*;
use crate::error_handling::*;

#[derive(Debug, PartialEq)]
pub enum GenerateErrorType {
    // An undefined nonterminal was used
    UndefinedNonterminal(String),
}

impl ErrorType for GenerateErrorType {}

impl Display for GenerateErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerateErrorType::UndefinedNonterminal(nonterminal) => write!(f, "No definition for nonterminal `{}`", nonterminal),
            // The next line is here for when I need more GenerateErrors in the future
            // _ => write!(f, "{:#?}", self)
        }
    }
}

pub type GenerateError = Error<GenerateErrorType>;
pub type GenResult = Result<String, GenerateError>;

pub fn generate(grammar: &Grammar, file: PathBuf) -> GenResult {
    generate_nonterminal(&grammar.start_symbol, &grammar.rules, &Location {file, line: 0})
}

// Generates a sentence in the given grammar starting with the given symbol
pub fn generate_with_override(grammar: &Grammar, start: &String, file: PathBuf) -> GenResult {
    generate_nonterminal(start, &grammar.rules, &Location {file, line: 0})
}

fn generate_nonterminal(nonterminal: &String, rules: &HashMap<String, (Rewrite, Location)>, location: &Location) -> GenResult {
    let rewrite = rules
        .get(nonterminal)
        .ok_or_else(|| GenerateError {
            location: location.clone(),
            error: GenerateErrorType::UndefinedNonterminal(nonterminal.clone())
        })?;
    return generate_rewrite(&rewrite.0, rules, &rewrite.1);
}

fn generate_rewrite(rewrite: &Rewrite, rules: &HashMap<String, (Rewrite, Location)>, location: &Location) -> GenResult {
    let alternative = match rewrite.choose(&mut thread_rng()) {
        Some(a) => a,
        None => &Vec::new(),
    };

    let mut result = String::new();
    for token in alternative {
        result.push_str(&generate_symbol(token, rules, location)?);
    }

    return Ok(result);
}

fn generate_symbol(symbol: &Symbol, rules: &HashMap<String, (Rewrite, Location)>, location: &Location) -> GenResult {
    match symbol {
        Symbol::Nonterminal(t) => generate_nonterminal(t, rules, location),
        Symbol::Terminal(t) => Ok(t.clone()),
    }
}
