/*
    This module parses BNF files
*/

mod lexer;

use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::path::Path;

use crate::grammar::*;
use itertools::Itertools;
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

impl Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let CompileError::FileError(e) = self {
            write!(f, "FileError({:#?})", e.kind())
        } else {
            write!(f, "{:#?}", self)
        }
    }
}

// This allows the parser to specify what line an error occured on.
// If it is not a line-specific error, such as an io error,
// then the line field will be zero.
#[derive(Debug, PartialEq)]
pub struct LineCompileError {
    line: usize,
    error: CompileError
}

impl From<std::io::Error> for LineCompileError {
    fn from(value: std::io::Error) -> Self {
        LineCompileError {
            line: 0,
            error: CompileError::FileError(value)
        }
    }
}

impl Display for LineCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error {
            CompileError::FileError(_) => write!(f, "Encountered error {}", self.error),
            _ => write!(f, "Encountered error {} at line {}", self.error, self.line)
        }
    }
}

pub type Result<T> = std::result::Result<T, CompileError>;
pub type LineResult<T> = std::result::Result<T, LineCompileError>;
pub type FileResult<T> = std::result::Result<T, Vec<LineCompileError>>;

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

fn parse_line(tokens: &[Token]) -> Result<Rule> {
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

fn parse_lex_line(line_num: usize, line: &str) -> LineResult<Rule> {
    let lexed_line = lexer::lex_line(line).map_err(|error| LineCompileError {
        line: line_num,
        error
    })?;
    let parsed_line = parse_line(&lexed_line).map_err(|error| LineCompileError {
        line: line_num,
        error
    })?;
    
    return Ok(parsed_line);
}

fn is_rule_line(line: &String) -> bool {
    !line.is_empty() && !line.starts_with(';')
}

// Returns an iterator over the lines of a file, with the io errors wrapped
// in LineCompileError and enumerated
fn file_line_nums<'a>(file: File) -> impl Iterator<Item = (usize, LineResult<String>)> + 'a {
    std::io::BufReader::new(file)
        .lines()
        .map(|line| line.map_err(LineCompileError::from))
        .enumerate()
        .map(|(num, line)| (num + 1, line))
        .filter(|(_, line)| line.as_ref().is_ok_and(is_rule_line))
}

impl From<Vec<Rule>> for Grammar {
    fn from(value: Vec<Rule>) -> Self {
        let start_symbol = if value.len() > 0 {
            value[0].symbol.clone()
        } else {
            String::new()
        };

        let mut rules = HashMap::with_capacity(value.len());
        for rule in value {
            rules.insert(rule.symbol, rule.rewrite);
        }

        return Grammar {
            start_symbol,
            rules
        }
    }
}

pub fn parse_file(path: &impl AsRef<Path>) -> FileResult<Grammar> {
    let file = File::open(path).map_err(|e| vec![LineCompileError::from(e)])?;
    let lines = file_line_nums(file);

    // If the buffer read successfully, process it; if not, keep the io error
    let parsed_lines = lines.map(|(num, line_res)| match line_res {
        Ok(line) => parse_lex_line(num, &line),
        Err(e) => Err(e)
    });

    let (rules, errors): (Vec<_>, Vec<_>) = parsed_lines.partition(LineResult::is_ok);
    if errors.len() > 0 {
        return Err(errors.into_iter().map(LineResult::unwrap_err).collect_vec());
    }

    let rules_unwrapped = rules.into_iter().map(LineResult::unwrap).collect_vec();
    return Ok(rules_unwrapped.into());
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
    fn parse_normal_line() {
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

        assert_eq!(parse_line(&lexed[..]), Ok(answer));
    }

    #[test]
    fn parse_malformed_line() {
        // Blank
        assert_eq!(parse_line(&[]), Err(CompileError::UnexpectedBlankLine));

        // Missing equals
        assert_eq!(parse_line(
            &lexer::lex_line("alpha bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingEquals));

        // Improper definition
        assert_eq!(parse_line(
            &lexer::lex_line("\"alpha\" = bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingNonterminal));
        assert_eq!(parse_line(
            &lexer::lex_line("| = alpha bravo charlie").unwrap()[..]
        ), Err(CompileError::MissingNonterminal));
        assert_eq!(parse_line(
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
                Symbol::Terminal("\n".to_string())
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
            Symbol::Terminal("\n".to_string())
        ]]);
        rules.insert("zip.part".to_string(), vec![vec![
            Symbol::Nonterminal("town.name".to_string()),
            Symbol::Terminal(",".to_string()),
            Symbol::Nonterminal("state.code".to_string()),
            Symbol::Nonterminal("zip.code".to_string()),
            Symbol::Terminal("\n".to_string())
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

    #[test]
    fn parse_malformed_file() {
        let example_path = "example_data/malformed.bnf";
        let example_parsed = parse_file(&example_path).unwrap_err();

        assert_eq!(example_parsed, vec![
            LineCompileError {
                line: 3,
                error: CompileError::MissingNonterminal
            },
            LineCompileError {
                line: 7,
                error: CompileError::UnexpectedEquals
            }
        ]);
    }
}