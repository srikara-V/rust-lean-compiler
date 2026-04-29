use crate::error::{CompileError, Result};
use crate::lexer::{lex, Token, TokenKind};
use crate::surface::{Command, MatchBranch, Pattern, Term};

pub fn parse(input: &str) -> Result<Vec<Command>> {
    let tokens = lex(input)?;
    Parser { tokens, pos: 0 }.commands()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn commands(&mut self) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        while !self.at(&TokenKind::Eof) {
            commands.push(self.command()?);
        }
        Ok(commands)
    }

    fn command(&mut self) -> Result<Command> {
        match self.peek() {
            TokenKind::Def => {
                self.bump();
                let name = self.ident()?;
                self.expect(TokenKind::Colon)?;
                let ty = self.term()?;
                self.expect(TokenKind::Assign)?;
                let value = self.term()?;
                Ok(Command::Def { name, ty, value })
            }
            TokenKind::Eval => {
                self.bump();
                Ok(Command::Eval(self.term()?))
            }
            other => Err(self.err(format!("expected command, found {other:?}"))),
        }
    }

    fn term(&mut self) -> Result<Term> {
        if self.at(&TokenKind::Fun) {
            return self.lambda();
        }
        if self.at(&TokenKind::Let) {
            return self.let_term();
        }
        if self.at(&TokenKind::Match) {
            return self.match_term();
        }
        if let Some((name, ty)) = self.dependent_binder()? {
            self.expect(TokenKind::Arrow)?;
            let body = self.term()?;
            return Ok(Term::Pi {
                name: Some(name),
                ty: Box::new(ty),
                body: Box::new(body),
            });
        }
        self.arrow()
    }

    fn match_term(&mut self) -> Result<Term> {
        self.expect(TokenKind::Match)?;
        let scrutinee = self.term()?;
        self.expect(TokenKind::With)?;
        let mut branches = Vec::new();
        loop {
            if self.at(&TokenKind::Pipe) {
                self.bump();
            } else if !branches.is_empty() {
                break;
            }

            let pattern = self.pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.term()?;
            branches.push(MatchBranch { pattern, body });

            if !self.at(&TokenKind::Pipe) {
                break;
            }
        }

        if branches.is_empty() {
            return Err(self.err("match expression needs at least one branch".to_string()));
        }

        Ok(Term::Match {
            scrutinee: Box::new(scrutinee),
            branches,
        })
    }

    fn pattern(&mut self) -> Result<Pattern> {
        match self.peek().clone() {
            TokenKind::Number(value) => {
                self.bump();
                Ok(Pattern::Number(value))
            }
            TokenKind::Ident(name) if name == "_" => {
                self.bump();
                Ok(Pattern::Wildcard)
            }
            TokenKind::Ident(name) => {
                self.bump();
                if is_constructor_pattern(&name) {
                    let mut binders = Vec::new();
                    while let TokenKind::Ident(binder) = self.peek().clone() {
                        if binder == "_" || is_constructor_pattern(&binder) {
                            break;
                        }
                        self.bump();
                        binders.push(binder);
                    }
                    Ok(Pattern::Ctor { name, binders })
                } else {
                    Ok(Pattern::Var(name))
                }
            }
            TokenKind::Nat => {
                self.bump();
                Ok(Pattern::Ctor {
                    name: "Nat".to_string(),
                    binders: Vec::new(),
                })
            }
            other => Err(self.err(format!("expected pattern, found {other:?}"))),
        }
    }

    fn let_term(&mut self) -> Result<Term> {
        self.expect(TokenKind::Let)?;
        let name = self.ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.term()?;
        self.expect(TokenKind::Assign)?;
        let value = self.term()?;
        if self.at(&TokenKind::Semicolon) || self.at(&TokenKind::In) {
            self.bump();
        } else {
            return Err(self.err(format!(
                "expected {:?} or {:?}",
                TokenKind::Semicolon,
                TokenKind::In
            )));
        }
        let body = self.term()?;
        Ok(Term::Let {
            name,
            ty: Box::new(ty),
            value: Box::new(value),
            body: Box::new(body),
        })
    }

    fn lambda(&mut self) -> Result<Term> {
        self.expect(TokenKind::Fun)?;
        let name = self.ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.term()?;
        self.expect(TokenKind::FatArrow)?;
        let body = self.term()?;
        Ok(Term::Lam {
            name,
            ty: Box::new(ty),
            body: Box::new(body),
        })
    }

    fn arrow(&mut self) -> Result<Term> {
        let left = self.add()?;
        if self.at(&TokenKind::Arrow) {
            self.bump();
            let right = self.term()?;
            Ok(Term::Pi {
                name: None,
                ty: Box::new(left),
                body: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn add(&mut self) -> Result<Term> {
        let mut term = self.app()?;
        while self.at(&TokenKind::Plus) {
            self.bump();
            let rhs = self.app()?;
            term = Term::Add(Box::new(term), Box::new(rhs));
        }
        Ok(term)
    }

    fn app(&mut self) -> Result<Term> {
        let mut term = self.atom()?;
        while self.starts_atom() {
            let arg = self.atom()?;
            term = Term::App(Box::new(term), Box::new(arg));
        }
        Ok(term)
    }

    fn atom(&mut self) -> Result<Term> {
        match self.peek().clone() {
            TokenKind::Type => {
                self.bump();
                Ok(Term::Type)
            }
            TokenKind::Nat => {
                self.bump();
                Ok(Term::Nat)
            }
            TokenKind::Number(value) => {
                self.bump();
                Ok(Term::Number(value))
            }
            TokenKind::Ident(name) => {
                self.bump();
                Ok(Term::Ident(name))
            }
            TokenKind::LParen => {
                self.bump();
                let term = self.term()?;
                self.expect(TokenKind::RParen)?;
                Ok(term)
            }
            other => Err(self.err(format!("expected term, found {other:?}"))),
        }
    }

    fn dependent_binder(&mut self) -> Result<Option<(String, Term)>> {
        let checkpoint = self.pos;
        if !self.at(&TokenKind::LParen) {
            return Ok(None);
        }
        self.bump();
        let name = match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.bump();
                name
            }
            _ => {
                self.pos = checkpoint;
                return Ok(None);
            }
        };
        if !self.at(&TokenKind::Colon) {
            self.pos = checkpoint;
            return Ok(None);
        }
        self.bump();
        let ty = self.term()?;
        if !self.at(&TokenKind::RParen) {
            self.pos = checkpoint;
            return Ok(None);
        }
        self.bump();
        if !self.at(&TokenKind::Arrow) {
            self.pos = checkpoint;
            return Ok(None);
        }
        Ok(Some((name, ty)))
    }

    fn starts_atom(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Type
                | TokenKind::Nat
                | TokenKind::Number(_)
                | TokenKind::Ident(_)
                | TokenKind::LParen
        )
    }

    fn ident(&mut self) -> Result<String> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.bump();
                Ok(name)
            }
            other => Err(self.err(format!("expected identifier, found {other:?}"))),
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Result<()> {
        if self.at(&expected) {
            self.bump();
            Ok(())
        } else {
            Err(self.err(format!("expected {expected:?}, found {:?}", self.peek())))
        }
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn err(&self, message: String) -> CompileError {
        CompileError::new(format!("{message} at {}", self.tokens[self.pos].pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dependent_identity_type() {
        let commands =
            parse("def id : (A : Type) -> A -> A := fun A : Type => fun x : A => x").unwrap();
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn parses_application_left_associative() {
        let commands = parse("#eval f x y").unwrap();
        match &commands[0] {
            Command::Eval(Term::App(fun, _)) => assert!(matches!(**fun, Term::App(_, _))),
            other => panic!("unexpected parse: {other:?}"),
        }
    }

    #[test]
    fn parses_addition_left_associative() {
        let commands = parse("#eval 1 + 2 + 3").unwrap();
        match &commands[0] {
            Command::Eval(Term::Add(left, _)) => assert!(matches!(**left, Term::Add(_, _))),
            other => panic!("unexpected parse: {other:?}"),
        }
    }

    #[test]
    fn parses_let_with_semicolon_or_in() {
        assert!(parse("#eval let x : Nat := 1; x + 1").is_ok());
        assert!(parse("#eval let x : Nat := 1 in x + 1").is_ok());
    }

    #[test]
    fn parses_match_branches() {
        let commands = parse("#eval match b with | true => false | false => true").unwrap();
        match &commands[0] {
            Command::Eval(Term::Match { branches, .. }) => assert_eq!(branches.len(), 2),
            other => panic!("unexpected parse: {other:?}"),
        }
    }
}

fn is_constructor_pattern(name: &str) -> bool {
    matches!(
        name,
        "true" | "false" | "none" | "some" | "nil" | "cons" | "zero" | "succ"
    )
}
