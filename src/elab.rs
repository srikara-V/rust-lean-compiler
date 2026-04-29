use crate::core::{Definition, Env, Term as CoreTerm};
use crate::error::{CompileError, Result};
use crate::eval::normalize;
use crate::parser::parse;
use crate::surface::{Command, Term};
use crate::typeck::{check, infer};

#[derive(Debug, Default)]
pub struct Session {
    env: Env,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_source(&mut self, source: &str) -> Result<Vec<String>> {
        let commands = parse(source)?;
        let mut output = Vec::new();

        for command in commands {
            match command {
                Command::Def { name, ty, value } => {
                    if self.env.contains_key(&name) {
                        return Err(CompileError::new(format!("duplicate definition '{name}'")));
                    }
                    let ty = elaborate(&ty, &mut Vec::new())?;
                    check(&self.env, &mut Vec::new(), &ty, &CoreTerm::Sort)?;
                    let value = elaborate(&value, &mut Vec::new())?;
                    check(&self.env, &mut Vec::new(), &value, &ty)?;
                    self.env.insert(name, Definition { ty, value });
                }
                Command::Eval(term) => {
                    let term = elaborate(&term, &mut Vec::new())?;
                    infer(&self.env, &mut Vec::new(), &term)?;
                    output.push(normalize(&self.env, &term).pretty());
                }
            }
        }

        Ok(output)
    }
}

fn elaborate(term: &Term, locals: &mut Vec<String>) -> Result<CoreTerm> {
    match term {
        Term::Type => Ok(CoreTerm::Sort),
        Term::Ident(name) => resolve(name, locals),
        Term::Lam { name, ty, body } => {
            let ty = elaborate(ty, locals)?;
            locals.push(name.clone());
            let body = elaborate(body, locals);
            locals.pop();
            Ok(CoreTerm::Lam {
                name: name.clone(),
                ty: Box::new(ty),
                body: Box::new(body?),
            })
        }
        Term::Pi { name, ty, body } => {
            let ty = elaborate(ty, locals)?;
            let binder = name.clone().unwrap_or_else(|| "_".to_string());
            locals.push(binder.clone());
            let body = elaborate(body, locals);
            locals.pop();
            Ok(CoreTerm::Pi {
                name: binder,
                ty: Box::new(ty),
                body: Box::new(body?),
            })
        }
        Term::App(fun, arg) => Ok(CoreTerm::App(
            Box::new(elaborate(fun, locals)?),
            Box::new(elaborate(arg, locals)?),
        )),
    }
}

fn resolve(name: &str, locals: &[String]) -> Result<CoreTerm> {
    for (index, local) in locals.iter().rev().enumerate() {
        if local == name {
            return Ok(CoreTerm::Var(index));
        }
    }
    Ok(CoreTerm::Const(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_identity_eval() {
        let mut session = Session::new();
        let output = session
            .run_source("def id : (A : Type) -> A -> A := fun A : Type => fun x : A => x\n#eval id Type Type")
            .unwrap();
        assert_eq!(output, vec!["Type"]);
    }

    #[test]
    fn rejects_bad_application() {
        let mut session = Session::new();
        let err = session.run_source("#eval Type Type").unwrap_err();
        assert!(err.message.contains("cannot apply non-function"));
    }
}
