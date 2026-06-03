use crate::builtins::{initial_env, is_builtin};
use crate::core::{
    Definition, Env, MatchBranch as CoreMatchBranch, Pattern as CorePattern, Term as CoreTerm,
};
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
    /// Create a session with builtin definitions loaded.
    pub fn new() -> Self {
        Self { env: initial_env() }
    }

    /// Parse, elaborate, typecheck, and evaluate commands from source text.
    pub fn run_source(&mut self, source: &str) -> Result<Vec<String>> {
        let commands = parse(source)?;
        let mut output = Vec::new();

        for command in commands {
            match command {
                Command::Def { name, ty, value } => {
                    if self.env.contains_key(&name) && is_builtin(&name) {
                        return Err(CompileError::new(format!(
                            "cannot redefine builtin '{name}'"
                        )));
                    }
                    if self
                        .env
                        .get(&name)
                        .and_then(|def| def.value.as_ref())
                        .is_some()
                    {
                        return Err(CompileError::new(format!("duplicate definition '{name}'")));
                    }
                    let ty = elaborate(&ty, &mut Vec::new())?;
                    check(&self.env, &mut Vec::new(), &ty, &CoreTerm::Sort)?;
                    self.env.insert(
                        name.clone(),
                        Definition {
                            ty: ty.clone(),
                            value: None,
                        },
                    );
                    let value = elaborate(&value, &mut Vec::new())?;
                    check(&self.env, &mut Vec::new(), &value, &ty)?;
                    validate_structural_recursion(&name, &value)?;
                    self.env.insert(
                        name,
                        Definition {
                            ty,
                            value: Some(value),
                        },
                    );
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
        Term::Nat => Ok(CoreTerm::NatType),
        Term::Number(value) => Ok(CoreTerm::NatLit(*value)),
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
        Term::Add(left, right) => Ok(CoreTerm::Add(
            Box::new(elaborate(left, locals)?),
            Box::new(elaborate(right, locals)?),
        )),
        Term::Match {
            scrutinee,
            branches,
        } => {
            let scrutinee = elaborate(scrutinee, locals)?;
            let branches = branches
                .iter()
                .map(|branch| {
                    let pattern = elaborate_pattern(&branch.pattern);
                    for binder in pattern.binders() {
                        locals.push(binder);
                    }
                    let body = elaborate(&branch.body, locals);
                    for _ in 0..pattern.binding_count() {
                        locals.pop();
                    }
                    body.map(|body| CoreMatchBranch { pattern, body })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(CoreTerm::Match {
                scrutinee: Box::new(scrutinee),
                branches,
            })
        }
        Term::Let {
            name,
            ty,
            value,
            body,
        } => {
            let ty = elaborate(ty, locals)?;
            let value = elaborate(value, locals)?;
            locals.push(name.clone());
            let body = elaborate(body, locals);
            locals.pop();
            Ok(CoreTerm::Let {
                name: name.clone(),
                ty: Box::new(ty),
                value: Box::new(value),
                body: Box::new(body?),
            })
        }
    }
}

fn elaborate_pattern(pattern: &crate::surface::Pattern) -> CorePattern {
    match pattern {
        crate::surface::Pattern::Wildcard => CorePattern::Wildcard,
        crate::surface::Pattern::Var(name) => CorePattern::Var(name.clone()),
        crate::surface::Pattern::Number(value) => CorePattern::NatLit(*value),
        crate::surface::Pattern::Ctor { name, binders } => CorePattern::Ctor {
            name: name.clone(),
            binders: binders.clone(),
        },
    }
}

trait PatternBinders {
    fn binders(&self) -> Vec<String>;
}

impl PatternBinders for CorePattern {
    fn binders(&self) -> Vec<String> {
        match self {
            CorePattern::Wildcard | CorePattern::NatLit(_) => Vec::new(),
            CorePattern::Var(name) => vec![name.clone()],
            CorePattern::Ctor { binders, .. } => binders.clone(),
        }
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

fn validate_structural_recursion(name: &str, value: &CoreTerm) -> Result<()> {
    if validates_recursion(name, value, &mut Vec::new(), false) {
        Ok(())
    } else {
        Err(CompileError::new(format!(
            "recursive definition '{name}' is not structurally recursive"
        )))
    }
}

fn validates_recursion(
    name: &str,
    term: &CoreTerm,
    smaller_locals: &mut Vec<bool>,
    under_match: bool,
) -> bool {
    if is_recursive_call(name, term) {
        return under_match && recursive_call_has_smaller_arg(name, term, smaller_locals);
    }

    match term {
        CoreTerm::Sort
        | CoreTerm::NatType
        | CoreTerm::NatLit(_)
        | CoreTerm::Var(_)
        | CoreTerm::Const(_) => true,
        CoreTerm::Pi { ty, body, .. } | CoreTerm::Lam { ty, body, .. } => {
            validates_recursion(name, ty, smaller_locals, under_match) && {
                smaller_locals.push(false);
                let valid = validates_recursion(name, body, smaller_locals, under_match);
                smaller_locals.pop();
                valid
            }
        }
        CoreTerm::App(fun, arg) | CoreTerm::Add(fun, arg) => {
            validates_recursion(name, fun, smaller_locals, under_match)
                && validates_recursion(name, arg, smaller_locals, under_match)
        }
        CoreTerm::Match {
            scrutinee,
            branches,
        } => {
            if !validates_recursion(name, scrutinee, smaller_locals, under_match) {
                return false;
            }
            branches.iter().all(|branch| {
                let flags = smaller_flags(&branch.pattern);
                smaller_locals.extend(flags.iter().copied());
                let valid = validates_recursion(name, &branch.body, smaller_locals, true);
                for _ in flags {
                    smaller_locals.pop();
                }
                valid
            })
        }
        CoreTerm::Let {
            ty, value, body, ..
        } => {
            validates_recursion(name, ty, smaller_locals, under_match)
                && validates_recursion(name, value, smaller_locals, under_match)
                && {
                    smaller_locals.push(false);
                    let valid = validates_recursion(name, body, smaller_locals, under_match);
                    smaller_locals.pop();
                    valid
                }
        }
    }
}

fn smaller_flags(pattern: &CorePattern) -> Vec<bool> {
    match pattern {
        CorePattern::Var(_) => vec![false],
        CorePattern::Ctor { name, binders } if name == "succ" && binders.len() == 1 => vec![true],
        CorePattern::Ctor { name, binders } if name == "some" && binders.len() == 1 => vec![true],
        CorePattern::Ctor { name, binders } if name == "cons" && binders.len() == 2 => {
            vec![false, true]
        }
        CorePattern::Ctor { binders, .. } => vec![false; binders.len()],
        CorePattern::Wildcard | CorePattern::NatLit(_) => Vec::new(),
    }
}

fn is_recursive_call(name: &str, term: &CoreTerm) -> bool {
    let (head, _) = collect_app(term);
    matches!(head, CoreTerm::Const(const_name) if const_name == name)
}

fn recursive_call_has_smaller_arg(name: &str, term: &CoreTerm, smaller_locals: &[bool]) -> bool {
    let (head, args) = collect_app(term);
    matches!(head, CoreTerm::Const(const_name) if const_name == name)
        && args.iter().any(|arg| is_smaller_local(arg, smaller_locals))
}

fn is_smaller_local(term: &CoreTerm, smaller_locals: &[bool]) -> bool {
    match term {
        CoreTerm::Var(index) => smaller_locals
            .len()
            .checked_sub(index + 1)
            .and_then(|pos| smaller_locals.get(pos))
            .copied()
            .unwrap_or(false),
        _ => false,
    }
}

fn collect_app(term: &CoreTerm) -> (&CoreTerm, Vec<&CoreTerm>) {
    let mut args = Vec::new();
    let mut current = term;
    while let CoreTerm::App(fun, arg) = current {
        args.push(arg.as_ref());
        current = fun;
    }
    args.reverse();
    (current, args)
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

    #[test]
    fn evaluates_nat_arithmetic_and_let() {
        let mut session = Session::new();
        let output = session
            .run_source("def two : Nat := 1 + 1\n#eval let x : Nat := two; x + 40")
            .unwrap();
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn rejects_non_nat_addition() {
        let mut session = Session::new();
        let err = session.run_source("#eval Type + 1").unwrap_err();
        assert!(err.message.contains("type mismatch"));
    }
}
