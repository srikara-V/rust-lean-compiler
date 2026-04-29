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
    Type,
    Ident(String),
    LParen,
    RParen,
    Colon,
    Assign,
    Arrow,
    FatArrow,
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
            c if is_ident_start(c) => {
                i += 1;
                while i < chars.len() && is_ident_continue(chars[i]) {
                    i += 1;
                }
                let word: String = chars[pos..i].iter().collect();
                match word.as_str() {
                    "def" => TokenKind::Def,
                    "fun" => TokenKind::Fun,
                    "Type" => TokenKind::Type,
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
}
