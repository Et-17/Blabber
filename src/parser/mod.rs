/*
    This module parses BNF files
*/

mod lexer;
mod verifier;

use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;

use crate::grammar::*;
use crate::error_handling::*;
use itertools::Itertools;
use lexer::*;
use verifier::verify_rules;
use verifier::IntermediateRuleset;

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
    // An undefined token was used
    UndefinedNonterminal(String),
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
            CompileErrorType::UndefinedNonterminal(nonterminal) => write!(f, "Could not find definition for `{}`", nonterminal),
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

// Generates a rule hashmap from a vector of rules
fn ruleset_from_rules(rules: Vec<Rule>) -> FileResult<HashMap<String, Rewrite>> {
    let rule_count = rules.len();

    // Construct test hashmap
    let mut test_ruleset = IntermediateRuleset::with_capacity(rule_count);
    for rule in rules {
        test_ruleset.insert(rule.symbol, (rule.rewrite, rule.location));
    }

    verify_rules(&test_ruleset)?;

    let mut ruleset = HashMap::<String, Rewrite>::with_capacity(rule_count);
    for (symbol, (rewrite, _)) in test_ruleset.drain() {
        ruleset.insert(symbol, rewrite);
    }

    return Ok(ruleset);
}

fn grammar_from_rules(rule_list: Vec<Rule>) -> FileResult<Grammar> {
    let start_symbol = if rule_list.len() > 0 {
        rule_list[0].symbol.clone()
    } else {
        String::new()
    };

    let rules = ruleset_from_rules(rule_list)?;

    return Ok(Grammar {
        start_symbol,
        rules
    })
}

pub fn parse_file(path: &PathBuf) -> FileResult<Grammar> {
    let file = File::open(path).map_err(|e| vec![io_error(e, path.clone())])?;
    let lines = file_line_nums(file, path);

    let parsed_lines = lines.map(|(num, line_res)| {
        line_res.and_then(|line| parse_lex_line(&line, Location {
            file: path.clone(),
            line: num
        }))
    });

    let (rules, errors): (Vec<_>, Vec<_>) = parsed_lines.partition(LineResult::is_ok);
    if errors.len() > 0 {
        return Err(errors.into_iter().map(LineResult::unwrap_err).collect_vec());
    }
    let rules_unwrapped = rules.into_iter().map(LineResult::unwrap).collect_vec();

    return grammar_from_rules(rules_unwrapped);
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

    fn s_nonterminal(text: &str) -> Symbol {
        Symbol::Nonterminal(text.to_string())
    }

    fn s_terminal(text: &str) -> Symbol {
        Symbol::Terminal(text.to_string())
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
                s_nonterminal("personal.part"),
                s_nonterminal("last.name"),
                s_nonterminal("opt.suffix.name"),
                s_terminal("\\n")
            ],
            vec![
                s_nonterminal("town.name"),
                s_terminal(",")
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
                vec![s_nonterminal("first.name")],
                vec![
                    s_nonterminal("initial"),
                    s_terminal(".")
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
        let example_path = PathBuf::from("example_data/english.bnf");
        let example_parsed = parse_file(&example_path).unwrap();
        
        let mut rules = HashMap::new();
        rules.insert("sentence".to_string(), vec![vec![
            s_nonterminal("noun.phrase"),
            s_terminal(" "),
            s_nonterminal("verb.phrase")
        ]]);
        rules.insert("noun.phrase".to_string(), vec![
            vec![
                s_nonterminal("adjective.phrase"),
                s_terminal(" "),
                s_nonterminal("noun")
            ],
            vec![s_nonterminal("noun")]
        ]);
        rules.insert("noun".to_string(), vec![vec![s_terminal("ideas")]]);
        rules.insert("adjective.phrase".to_string(), vec![
            vec![
                s_nonterminal("adjective"),
                s_terminal(", "),
                s_nonterminal("adjective.phrase")
            ],
            vec![s_nonterminal("adjective")]
        ]);
        rules.insert("adjective".to_string(), vec![
            vec![s_terminal("colorless")],
            vec![s_terminal("green")]
        ]);
        rules.insert("verb.phrase".to_string(), vec![
            vec![
                s_nonterminal("verb"),
                s_terminal(" "),
                s_nonterminal("adverb")
            ],
            vec![
                s_nonterminal("adverb"),
                s_terminal(" "),
                s_nonterminal("verb"),
                s_terminal(" "),
                s_nonterminal("noun.phrase")
            ]
        ]);
        rules.insert("verb".to_string(), vec![vec![s_terminal("hug")]]);
        rules.insert("adverb.phrase".to_string(), vec![
            vec![
                s_nonterminal("adverb"),
                s_terminal(", "),
                s_nonterminal("adverb.phrase")
            ],
            vec![s_nonterminal("adverb")]
        ]);
        rules.insert("adverb".to_string(), vec![vec![s_terminal("furiously")]]);

        assert_eq!(example_parsed, Grammar {
            start_symbol: "sentence".to_string(),
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