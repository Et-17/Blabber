mod grammar;
mod parser;

use std::collections::HashMap;

use grammar::Grammar;
use grammar::Symbol::{Terminal, Nonterminal};

fn main() {
    let mut rules = HashMap::new();
    rules.insert("postal.address".to_string(), vec![vec![Nonterminal("name.part".to_string()), Nonterminal("street.address".to_string()), Nonterminal("zip.part".to_string())]]);
    rules.insert("personal.part".to_string(), vec![vec![Nonterminal("first.name".to_string())], vec![Nonterminal("initial".to_string()), Terminal(".".to_string())]]);

    let test_grammar = Grammar {
        start_symbol: "bloop".to_string(),
        rules: rules,
    };
    println!("{:#?}", test_grammar);
}
