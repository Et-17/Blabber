use std::collections::HashMap;

use crate::grammar::Symbol::Nonterminal;
use super::CompileErrorType::UndefinedNonterminal;
use super::{Alternative, CompileError, CompileErrors, FileResult, Location, Rewrite};

pub type IntermediateRuleset = HashMap<String, (Rewrite, Location)>;

fn get_alternative_undefined_symbols(alternative: &Alternative, location: &Location, rules: &IntermediateRuleset) -> CompileErrors {
    // Filter out everything but nonterminals and unwrap the text from the
    // nonterminals. Then filter out all the undefined nonterminals.
    alternative.iter()
        .filter_map(|symbol| match symbol {
            Nonterminal(symbol) => Some(symbol),
            _ => None
        })
        .filter(|symbol| !rules.contains_key(*symbol))
        .map(|symbol_text| CompileError {
            location: location.to_owned(),
            error: UndefinedNonterminal(symbol_text.to_owned())
        })
        .collect()
}

fn get_rewrite_undefined_symbols(rewrite: &Rewrite, location: &Location, rules: &IntermediateRuleset) -> CompileErrors {
    // Get the undefined nonterminals in each alternative, while flattening
    // into all the undefined nonterminals in the rewrite
    rewrite.iter()
        .flat_map(|alternative| get_alternative_undefined_symbols(alternative, location, rules))
        .collect()
}

fn get_undefined_symbols(rules: &IntermediateRuleset) -> CompileErrors {
    // Get the undefined nonterminals in each rewrite, while flattening
    // into all the undefined nonterminals in the hashmap
    rules.iter()
        .flat_map(|(_, (rewrite, location))| get_rewrite_undefined_symbols(rewrite, location, rules))
        .collect()
}

pub fn verify_rules(rules: &IntermediateRuleset) -> FileResult<()> {
    let mut errors = Vec::new();

    errors.extend(get_undefined_symbols(&rules).into_iter());

    if errors.len() > 0 {
        Err(errors)
    } else {
        Ok(())
    }
}