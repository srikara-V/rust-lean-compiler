use crate::error::{CompileError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Def,
    Eval,
    Fun,
    Let,
    In,
    Match,
    With,
    Type,
    Nat,
    Number(u64),
    Ident(String),
    LParen,
    RParen,
    Colon,
    Assign,
    Arrow,
    FatArrow,
    Plus,
    Pipe,
    Semicolon,
    Eof,
}

pub fn lex(input: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '-' && chars.get(i + 1) == Some(&'-') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        let pos = i;
        let kind = match c {
            '(' => {
                i += 1;
                TokenKind::LParen
            }
            ')' => {
                i += 1;
                TokenKind::RParen
            }
            '+' => {
                i += 1;
                TokenKind::Plus
            }
            '|' => {
                i += 1;
                TokenKind::Pipe
            }
            ';' => {
                i += 1;
                TokenKind::Semicolon
            }
            ':' if chars.get(i + 1) == Some(&'=') => {
                i += 2;
                TokenKind::Assign
            }
            ':' => {
                i += 1;
                TokenKind::Colon
            }
            '-' if chars.get(i + 1) == Some(&'>') => {
                i += 2;
                TokenKind::Arrow
            }
            '=' if chars.get(i + 1) == Some(&'>') => {
                i += 2;
                TokenKind::FatArrow
            }
            '#' => {
                i += 1;
                let start = i;
                while i < chars.len() && is_ident_continue(chars[i]) {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.as_str() {
                    "eval" => TokenKind::Eval,
                    _ => {
                        return Err(CompileError::new(format!(
                            "unknown directive #{word} at {pos}"
                        )))
                    }
                }
            }
            c if c.is_ascii_digit() => {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let digits: String = chars[pos..i].iter().collect();
                let value = digits
                    .parse()
                    .map_err(|_| CompileError::new(format!("invalid number {digits} at {pos}")))?;
                TokenKind::Number(value)
            }
            c if is_ident_start(c) => {
                i += 1;
                while i < chars.len() && is_ident_continue(chars[i]) {
                    i += 1;
                }
                let word: String = chars[pos..i].iter().collect();
                match word.as_str() {
                    "def" => TokenKind::Def,
                    "fun" => TokenKind::Fun,
                    "let" => TokenKind::Let,
                    "in" => TokenKind::In,
                    "match" => TokenKind::Match,
                    "with" => TokenKind::With,
                    "Type" => TokenKind::Type,
                    "Nat" => TokenKind::Nat,
                    _ => TokenKind::Ident(word),
                }
            }
            _ => {
                return Err(CompileError::new(format!(
                    "unexpected character '{c}' at {pos}"
                )))
            }
        };

        tokens.push(Token { kind, pos });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        pos: input.len(),
    });
    Ok(tokens)
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c == '\'' || c.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_definition_keywords() {
        let tokens = lex("def id : Type := Type").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Def));
        assert_eq!(tokens[1].kind, TokenKind::Ident("id".to_string()));
        assert!(matches!(tokens[3].kind, TokenKind::Type));
        assert!(matches!(tokens[4].kind, TokenKind::Assign));
    }

    #[test]
    fn lexes_nat_literals_and_let() {
        let tokens = lex("let x : Nat := 1 + 2; x").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[3].kind, TokenKind::Nat));
        assert_eq!(tokens[5].kind, TokenKind::Number(1));
        assert!(matches!(tokens[6].kind, TokenKind::Plus));
    }
}
