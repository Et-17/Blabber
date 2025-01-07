use itertools::{Itertools, PeekingNext};

use super::{CompileError, Result};

#[derive(PartialEq, Debug)]
pub enum Token {
    Equals,
    Or,
    Nonterminal(String),
    Terminal(String)
}

pub fn lex_terminal(line: &mut impl PeekingNext<Item = char>) -> Result<Token> {
    line.next(); // Consume open quote
    let token_text = line.peeking_take_while(|&c| c != '\"').collect();

    // Check if there is a close quote and consume it if there is
    if line.next() != Some('\"') {
        return Err(CompileError::UnmatchedQuote);
    }

    Ok(Token::Terminal(token_text))
}

pub fn lex_nonterminal(line: &mut impl Iterator<Item = char>) -> Result<Token> {
    Ok(Token::Nonterminal(line.take_while(|c| !c.is_whitespace()).collect()))
}

pub fn lex_line(line: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();

    let mut line_chars = line.chars().peekable();

    while let Some(c) = line_chars.peek() {
        if *c == '=' {
            line_chars.next();
            tokens.push(Token::Equals);
        } else if *c == '|' {
            line_chars.next();
            tokens.push(Token::Or);
        } else if *c == '\"' {
            tokens.push(lex_terminal(&mut line_chars)?);
        } else if !c.is_whitespace() {
            tokens.push(lex_nonterminal(&mut line_chars)?);
        } else {
            line_chars.next();
        }
    }

    return Ok(tokens);
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use super::*;

    #[test]
    fn lex_normal_terminal() {
        let lines = vec![
            "\"alpha\" bravo charlie",
            "\"delta\"",
            "\"january\"\"february\"\"march\""
        ];
        // (result from the function, rest of the iterator)
        let answers = vec![
            (Token::Terminal("alpha".to_string()), " bravo charlie"),
            (Token::Terminal("delta".to_string()), ""),
            (Token::Terminal("january".to_string()), "\"february\"\"march\"")
        ];

        for (line, (answer_token, answer_rest)) in zip(lines, answers) {
            let mut chars = line.chars().peekable();
            assert_eq!(lex_terminal(&mut chars).unwrap(), answer_token);
            assert_eq!(chars.collect::<String>(), answer_rest);
        }
    }

    #[test]
    fn lex_mismatched_terminal() {
        let lines = vec![
            "\"welcome",
            "\"alpha bravo charlie"
        ];

        for line in lines {
            let mut chars = line.chars().peekable();
            chars.next();

            assert_eq!(lex_terminal(&mut chars).unwrap_err(), CompileError::UnmatchedQuote);
        }
    }

    #[test]
    fn lex_normal_nonterminal() {
        let lines = vec![
            "alpha bravo charlie",
            "delta",
            "january february march"
        ];
        // (result from the function, rest of the iterator)
        let answers = vec![
            (Token::Nonterminal("alpha".to_string()), "bravo charlie"),
            (Token::Nonterminal("delta".to_string()), ""),
            (Token::Nonterminal("january".to_string()), "february march")
        ];

        for (line, (answer_token, answer_rest)) in zip(lines, answers) {
            let mut chars = line.chars();
            assert_eq!(lex_nonterminal(&mut chars).unwrap(), answer_token);
            assert_eq!(chars.collect::<String>(), answer_rest);
        }
    }

    #[test]
    fn lex_normal_line() {
        let lines = vec![
            "personal.part = first.name | initial \".\"",
            "opt.apt.num = \"Apt\" apt.num | \"\""
        ];
        let answers = vec![
            vec![
                Token::Nonterminal("personal.part".to_string()),
                Token::Equals,
                Token::Nonterminal("first.name".to_string()),
                Token::Or,
                Token::Nonterminal("initial".to_string()),
                Token::Terminal(".".to_string())
            ],
            vec![
                Token::Nonterminal("opt.apt.num".to_string()),
                Token::Equals,
                Token::Terminal("Apt".to_string()),
                Token::Nonterminal("apt.num".to_string()),
                Token::Or,
                Token::Terminal("".to_string())
            ]
        ];

        for (line, answer) in zip(lines, answers) {
            assert_eq!(lex_line(line).unwrap(), answer)
        }
    }
}