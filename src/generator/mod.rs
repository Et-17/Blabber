/*
    This module generates sentences
*/

use rand::prelude::*;
use std::collections::HashMap;

use crate::grammar::*;

#[derive(Debug)]
pub enum GenerateError {
    // An undefined nonterminal was used
    UndefinedNonterminal(String),
}

pub type GenResult = Result<String, GenerateError>;

pub fn generate(grammar: Grammar) -> GenResult {
    generate_nonterminal(&grammar.start_symbol, &grammar.rules)
}

fn generate_nonterminal(nonterminal: &String, rules: &HashMap<String, Rewrite>) -> GenResult {
    let rewrite = rules
        .get(nonterminal)
        .ok_or_else(|| GenerateError::UndefinedNonterminal(nonterminal.clone()))?;
    return generate_rewrite(rewrite, rules);
}

fn generate_rewrite(rewrite: &Rewrite, rules: &HashMap<String, Rewrite>) -> GenResult {
    let alternative = match rewrite.choose(&mut thread_rng()) {
        Some(a) => a,
        None => &Vec::new(),
    };

    let mut result = String::new();
    for token in alternative {
        result.push_str(&generate_symbol(token, rules)?);
    }

    return Ok(result);
}

fn generate_symbol(symbol: &Symbol, rules: &HashMap<String, Rewrite>) -> GenResult {
    match symbol {
        Symbol::Nonterminal(t) => generate_nonterminal(t, rules),
        Symbol::Terminal(t) => Ok(t.clone()),
    }
}
