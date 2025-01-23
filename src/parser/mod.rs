/*
    This module parses BNF files
*/

mod lexer;

use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;

use crate::grammar::*;
use crate::error_handling::*;
use itertools::Itertools;
use lexer::*;

#[derive(Debug)]
pub enum CompileErrorType {
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

impl ErrorType for CompileErrorType {}

impl PartialEq for CompileErrorType {
    fn eq(&self, other: &Self) -> bool {
        if let CompileErrorType::FileError(a) = self {
            if let CompileErrorType::FileError(b) = other {
                return a.kind() == b.kind();
            }
        }
        return std::mem::discriminant(self) == std::mem::discriminant(other);
    }
}

impl Display for CompileErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileErrorType::MissingEquals => write!(f, "Expected `=` after nonterminal"),
            CompileErrorType::UnexpectedEquals => write!(f, "Unexpected `=` encountered"),
            CompileErrorType::MissingNonterminal => write!(f, "Tried to define something other than a nonterminal"),
            CompileErrorType::UnmatchedQuote => write!(f, "Unmatched quotes"),
            CompileErrorType::UnsplitRewrite => write!(f, "Rewrite was not fully split (this is a problem with blabber, not the grammar)"),
            CompileErrorType::UnexpectedBlankLine => write!(f, "Blank line encountered in rule parser (this is a problem with blabber, not the grammar)"),
            CompileErrorType::FileError(e) => write!(f, "File error: {}", e),
        }
    }
}

pub type CompileError = Error<CompileErrorType>;
pub type CompileErrors = Errors<CompileErrorType>;

fn io_error(error: std::io::Error, file: PathBuf) -> CompileError {
    CompileError {
        location: Location {
            file,
            line: 0
        },
        error: CompileErrorType::FileError(error)
    }
}

pub type Result<T> = std::result::Result<T, CompileErrorType>;
pub type LineResult<T> = std::result::Result<T, CompileError>;
pub type FileResult<T> = std::result::Result<T, CompileErrors>;

#[derive(PartialEq, Debug)]
struct Rule {
    symbol: String,
    rewrite: Rewrite,
    location: Location
}

fn parse_alternative(tokens: &[Token]) -> Result<Alternative> {
    tokens.iter().map(|t| match t {
        Token::Equals => Err(CompileErrorType::UnexpectedEquals),
        Token::Or => Err(CompileErrorType::UnsplitRewrite),
        Token::Nonterminal(s) => Ok(Symbol::Nonterminal(s.clone())),
        Token::Terminal(s) => Ok(Symbol::Terminal(s.clone()))
    }).collect()
}

fn parse_rewrite(tokens: &[Token]) -> Result<Rewrite> {
    tokens.split(|t| *t == Token::Or).map(parse_alternative).collect()
}

fn parse_line(tokens: &[Token], location: Location) -> Result<Rule> {
    // Try to get the token the rule is for. The match returns a result which
    // is then unwrapped with the ? operator
    let symbol = match tokens.get(0) {
        Some(Token::Nonterminal(s)) => Ok(s.clone()),
        Some(_) => Err(CompileErrorType::MissingNonterminal),
        None => Err(CompileErrorType::UnexpectedBlankLine)
    }?;

    if tokens.get(1) != Some(&Token::Equals) {
        return Err(CompileErrorType::MissingEquals)
    }

    let rewrite = parse_rewrite(&tokens[2..])?;

    return Ok(Rule {
        symbol,
        rewrite,
        location
    });
}

fn parse_lex_line(line: &str, location: Location) -> LineResult<Rule> {
    lexer::lex_line(line)
        .and_then(|lexed_line| parse_line(&lexed_line, location.clone()))
        .map_err(|error| CompileError { location: location, error })
}

fn is_rule_line(line: &String) -> bool {
    !line.is_empty() && !line.starts_with(';')
}

// Returns an iterator over the lines of a file, with the io errors wrapped
// in CompileError and enumerated
fn file_line_nums<'a>(file: File, path: &'a PathBuf) -> impl Iterator<Item = (usize, LineResult<String>)> + 'a {
    std::io::BufReader::new(file)
        .lines()
        .map(move |line| line.map_err(|e| io_error(e, path.clone())))
        .enumerate()
        .filter(|(_, line)| line.as_ref().is_ok_and(is_rule_line) || line.is_err())
        .map(|(num, line)| (num + 1, line))
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
            rules.insert(rule.symbol, (rule.rewrite, rule.location));
        }

        return Grammar {
            start_symbol,
            rules
        }
    }
}

pub fn parse_file(path: &PathBuf) -> FileResult<Grammar> {
    let file = File::open(path).map_err(|e| vec![io_error(e, path.clone())])?;
    let lines = file_line_nums(file, path);

    // If the buffer read successfully, process it; if not, keep the io error
    let parsed_lines = lines.map(|(num, line_res)| match line_res {
        Ok(line) => parse_lex_line(&line, Location {
            file: path.clone(),
            line: num
        }),
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

    impl Location {
        pub fn new() -> Self {
            Location {
                file: PathBuf::new(),
                line: 0
            }
        }
    }

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
        assert_eq!(parse_alternative(&[Token::Equals]), Err(CompileErrorType::UnexpectedEquals));
        assert_eq!(parse_alternative(&[Token::Or]), Err(CompileErrorType::UnsplitRewrite));
    }

    #[test]
    fn parse_normal_line() {
        let text = "personal.part = first.name | initial \".\"";
        let lexed = lexer::lex_line(text).unwrap();
        let location = Location {
            file: PathBuf::new(),
            line: 0
        };

        let answer = Rule {
            symbol: "personal.part".to_string(),
            rewrite: vec![
                vec![Symbol::Nonterminal("first.name".to_string())],
                vec![
                    Symbol::Nonterminal("initial".to_string()),
                    Symbol::Terminal(".".to_string())
                ]
            ],
            location: location.clone()
        };

        assert_eq!(parse_line(&lexed[..], location), Ok(answer));
    }

    #[test]
    fn parse_malformed_line() {
        // Blank
        assert_eq!(parse_line(&[], Location::new()), Err(CompileErrorType::UnexpectedBlankLine));

        // Missing equals
        assert_eq!(parse_line(
            &lexer::lex_line("alpha bravo charlie").unwrap()[..],
            Location::new()
        ), Err(CompileErrorType::MissingEquals));

        // Improper definition
        assert_eq!(parse_line(
            &lexer::lex_line("\"alpha\" = bravo charlie").unwrap()[..],
            Location::new()
        ), Err(CompileErrorType::MissingNonterminal));
        assert_eq!(parse_line(
            &lexer::lex_line("| = alpha bravo charlie").unwrap()[..],
            Location::new()
        ), Err(CompileErrorType::MissingNonterminal));
        assert_eq!(parse_line(
            &lexer::lex_line("= alpha bravo charlie").unwrap()[..],
            Location::new()
        ), Err(CompileErrorType::MissingNonterminal));
    }

    #[test]
    fn parse_normal_file() {
        let example_path = PathBuf::from("example_data/postal_address.bnf");
        let example_parsed = parse_file(&example_path).unwrap();
        let mut rules = HashMap::new();

        rules.insert("postal.address".to_string(), (vec![vec![
            Symbol::Nonterminal("name.part".to_string()),
            Symbol::Nonterminal("street.address".to_string()),
            Symbol::Nonterminal("zip.part".to_string())
        ]], Location { file: example_path.clone(), line: 4}));
        rules.insert("name.part".to_string(), (vec![
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
        ], Location { file: example_path.clone(), line: 7}));
        rules.insert("personal.part".to_string(), (vec![
            vec![Symbol::Nonterminal("first.name".to_string())],
            vec![
                Symbol::Nonterminal("initial".to_string()),
                Symbol::Terminal(".".to_string())
            ]
        ], Location { file: example_path.clone(), line: 8}));
        rules.insert("street.address".to_string(), (vec![vec![
            Symbol::Nonterminal("house.num".to_string()),
            Symbol::Nonterminal("street.name".to_string()),
            Symbol::Nonterminal("opt.apt.num".to_string()),
            Symbol::Terminal("\n".to_string())
        ]], Location { file: example_path.clone(), line: 9}));
        rules.insert("zip.part".to_string(), (vec![vec![
            Symbol::Nonterminal("town.name".to_string()),
            Symbol::Terminal(",".to_string()),
            Symbol::Nonterminal("state.code".to_string()),
            Symbol::Nonterminal("zip.code".to_string()),
            Symbol::Terminal("\n".to_string())
        ]], Location { file: example_path.clone(), line: 12}));
        rules.insert("opt.suffix.part".to_string(), (vec![
            vec![Symbol::Terminal("Sr.".to_string())],
            vec![Symbol::Terminal("Jr.".to_string())],
            vec![Symbol::Nonterminal("roman.numeral".to_string())],
            vec![Symbol::Terminal("".to_string())]
        ], Location { file: example_path.clone(), line: 15}));
        rules.insert("opt.apt.num".to_string(), (vec![
            vec![
                Symbol::Terminal("Apt".to_string()),
                Symbol::Nonterminal("apt.num".to_string())
            ],
            vec![Symbol::Terminal("".to_string())]
        ], Location { file: example_path.clone(), line: 16}));

        assert_eq!(example_parsed, Grammar {
            start_symbol: "postal.address".to_string(),
            rules
        });
    }

    #[test]
    fn parse_malformed_file() {
        let example_path = PathBuf::from("example_data/malformed.bnf");
        let example_parsed = parse_file(&example_path).unwrap_err();

        assert_eq!(example_parsed, vec![
            CompileError {
                location: Location {
                    file: example_path.clone(),
                    line: 3
                },
                error: CompileErrorType::MissingNonterminal
            },
            CompileError {
                location: Location {
                    file: example_path,
                    line: 7
                },
                error: CompileErrorType::UnexpectedEquals
            }
        ]);
    }
}