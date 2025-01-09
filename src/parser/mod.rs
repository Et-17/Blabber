/*
    This module parses BNF files
*/

mod lexer;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::path::Path;

use crate::grammar::*;
use lexer::*;

#[derive(Debug)]
pub enum CompileError {
    // A line which should contain a rule does not
    MissingEquals,
    // A rule has multiple equals signs
    UnexpectedEquals,
    // The user starts a rule line with something other than a nonterminal
    MissingNonterminal,
    // There is an unclosed quote
    UnmatchedQuote,
    // Somehow a full rewrite was parsed as a base alternative
    // This is a problem with blabber, not the grammar
    UnsplitRewrite,
    // A blank line got too deep into the parser
    // This is a problem with blabber, not the grammar
    UnexpectedBlankLine,
    // There was an issue with reading a file
    FileError(std::io::Error),
}

impl PartialEq for CompileError {
    fn eq(&self, other: &Self) -> bool {
        if let CompileError::FileError(a) = self {
            if let CompileError::FileError(b) = other {
                return a.kind() == b.kind();
            }
        }
        return std::mem::discriminant(self) == std::mem::discriminant(other);
    }
}

pub type Result<T> = std::result::Result<T, CompileError>;

#[derive(PartialEq, Debug)]
struct Rule {
    symbol: String,
    rewrite: Rewrite,
}

fn parse_alternative(tokens: &[Token]) -> Result<Alternative> {
    tokens.iter().map(|t| match t {
        Token::Equals => Err(CompileError::UnexpectedEquals),
        Token::Or => Err(CompileError::UnsplitRewrite),
        Token::Nonterminal(s) => Ok(Symbol::Nonterminal(s.clone())),
        Token::Terminal(s) => Ok(Symbol::Terminal(s.clone()))
    }).collect()
}

fn parse_rewrite(tokens: &[Token]) -> Result<Rewrite> {
    tokens.split(|t| *t == Token::Or).map(parse_alternative).collect()
}

fn parse_rule(tokens: &[Token]) -> Result<Rule> {
    // Try to get the token the rule is for. The match returns a result which
    // is then unwrapped with the ? operator
    let symbol = match tokens.get(0) {
        Some(Token::Nonterminal(s)) => Ok(s.clone()),
        Some(_) => Err(CompileError::MissingNonterminal),
        None => Err(CompileError::UnexpectedBlankLine)
    }?;

    // Verify the presence of the equals sign
    match tokens.get(1) {
        Some(Token::Equals) => Ok(()),
        _ => Err(CompileError::MissingEquals)
    }?;

    let rewrite = parse_rewrite(&tokens[2..])?;

    return Ok(Rule {
        symbol,
        rewrite
    });
}

pub fn parse_file(path: &impl AsRef<Path>) -> Result<Grammar> {
    let file = File::open(path).map_err(|e| CompileError::FileError(e))?;
    let buffer = std::io::BufReader::new(file).lines();

    let mut first_rule = true;
    let mut start_symbol = String::new();
    let mut rules: HashMap<String, Rewrite> = HashMap::new();
    
    for rule in buffer {
        let unwrapped_rule = rule.map_err(|e| CompileError::FileError(e))?;
        let lexed_rule = lex_line(&unwrapped_rule)?;
        let parsed_rule = parse_rule(&lexed_rule[..])?;
        rules.insert(parsed_rule.symbol.clone(), parsed_rule.rewrite);
        if first_rule {
            start_symbol = parsed_rule.symbol.clone();
            first_rule = false;
        }
    }

    return Ok(Grammar {
        start_symbol,
        rules
    });
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use super::*;

    #[test]
    fn parse_normal_alternative() {
        let lines = vec![
            vec![
                Token::Nonterminal("personal.part".to_string()),
                Token::Nonterminal("last.name".to_string()),
                Token::Nonterminal("opt.suffix.name".to_string()),
                Token::Terminal("\\n".to_string())
            ],
            vec![
                Token::Nonterminal("town.name".to_string()),
                Token::Terminal(",".to_string())
            ]
        ];
        let answers = vec![
            vec![
                Symbol::Nonterminal("personal.part".to_string()),
                Symbol::Nonterminal("last.name".to_string()),
                Symbol::Nonterminal("opt.suffix.name".to_string()),
                Symbol::Terminal("\\n".to_string())
            ],
            vec![
                Symbol::Nonterminal("town.name".to_string()),
                Symbol::Terminal(",".to_string())
            ]
        ];

        for (line, answer) in zip(lines, answers) {
            assert_eq!(parse_alternative(&line[..]).unwrap(), answer);
        }
    }

    #[test]
    fn parse_malformed_alternative() {
        assert_eq!(parse_alternative(&[Token::Equals]), Err(CompileError::UnexpectedEquals));
        assert_eq!(parse_alternative(&[Token::Or]), Err(CompileError::UnsplitRewrite));
    }

    #[test]
    fn parse_normal_rule() {
        let text = "personal.part = first.name | initial \".\"";
        let lexed = lexer::lex_line(text).unwrap();

        let answer = Rule {
            symbol: "personal.part".to_string(),
            rewrite: vec![
                vec![Symbol::Nonterminal("first.name".to_string())],
                vec![
                    Symbol::Nonterminal("initial".to_string()),
                    Symbol::Terminal(".".to_string())
                ]
            ]
        };

        assert_eq!(parse_rule(&lexed[..]), Ok(answer));
    }

    #[test]
    fn parse_malformed_rule() {
        // Blank
        assert_eq!(parse_rule(&[]), Err(CompileError::UnexpectedBlankLine));

        // Missing equals
        assert_eq!(parse_rule(
            &lexer::lex_line("alpha bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingEquals));

        // Improper definition
        assert_eq!(parse_rule(
            &lexer::lex_line("\"alpha\" = bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingNonterminal));
        assert_eq!(parse_rule(
            &lexer::lex_line("| = alpha bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingNonterminal));
        assert_eq!(parse_rule(
            &lexer::lex_line("= alpha bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingNonterminal));
    }

    #[test]
    fn parse_normal_file() {
        let example_path = "example_data/postal_address.bnf";
        let example_parsed = parse_file(&example_path).unwrap();
        let mut rules = HashMap::new();

        rules.insert("postal.address".to_string(), vec![vec![
            Symbol::Nonterminal("name.part".to_string()),
            Symbol::Nonterminal("street.address".to_string()),
            Symbol::Nonterminal("zip.part".to_string())
        ]]);
        rules.insert("name.part".to_string(), vec![
            vec![
                Symbol::Nonterminal("personal.part".to_string()),
                Symbol::Nonterminal("last.name".to_string()),
                Symbol::Nonterminal("opt.suffix.part".to_string()),
                Symbol::Terminal("\\n".to_string())
            ],
            vec![
                Symbol::Nonterminal("personal.part".to_string()),
                Symbol::Nonterminal("name.part".to_string())
            ]
        ]);
        rules.insert("personal.part".to_string(), vec![
            vec![Symbol::Nonterminal("first.name".to_string())],
            vec![
                Symbol::Nonterminal("initial".to_string()),
                Symbol::Terminal(".".to_string())
            ]
        ]);
        rules.insert("street.address".to_string(), vec![vec![
            Symbol::Nonterminal("house.num".to_string()),
            Symbol::Nonterminal("street.name".to_string()),
            Symbol::Nonterminal("opt.apt.num".to_string()),
            Symbol::Terminal("\\n".to_string())
        ]]);
        rules.insert("zip.part".to_string(), vec![vec![
            Symbol::Nonterminal("town.name".to_string()),
            Symbol::Terminal(",".to_string()),
            Symbol::Nonterminal("state.code".to_string()),
            Symbol::Nonterminal("zip.code".to_string()),
            Symbol::Terminal("\\n".to_string())
        ]]);
        rules.insert("opt.suffix.part".to_string(), vec![
            vec![Symbol::Terminal("Sr.".to_string())],
            vec![Symbol::Terminal("Jr.".to_string())],
            vec![Symbol::Nonterminal("roman.numeral".to_string())],
            vec![Symbol::Terminal("".to_string())]
        ]);
        rules.insert("opt.apt.num".to_string(), vec![
            vec![
                Symbol::Terminal("Apt".to_string()),
                Symbol::Nonterminal("apt.num".to_string())
            ],
            vec![Symbol::Terminal("".to_string())]
        ]);

        assert_eq!(example_parsed, Grammar {
            start_symbol: "postal.address".to_string(),
            rules
        });
    }
}