/*
    This module parses BNF files
*/

mod lexer;

use crate::grammar::*;
use lexer::*;

#[derive(Debug, Clone, PartialEq)]
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
}