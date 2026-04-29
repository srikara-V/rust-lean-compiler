use crate::builtins::{bool_ty, CONS, FALSE, LIST, NIL, NONE, OPTION, SOME, TRUE};
use crate::core::{shift, subst_top, Env, Pattern, Term};
use crate::error::{CompileError, Result};
use crate::eval::{equivalent, normalize};

#[derive(Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub ty: Term,
}

pub type Context = Vec<Binding>;

pub fn infer(env: &Env, ctx: &mut Context, term: &Term) -> Result<Term> {
    match term {
        Term::Sort => Ok(Term::Sort),
        Term::NatType => Ok(Term::Sort),
        Term::NatLit(_) => Ok(Term::NatType),
        Term::Var(index) => {
            let binding = ctx
                .get(ctx.len().checked_sub(index + 1).ok_or_else(|| {
                    CompileError::new(format!("unbound de Bruijn variable #{index}"))
                })?)
                .ok_or_else(|| CompileError::new(format!("unbound de Bruijn variable #{index}")))?;
            Ok(shift(&binding.ty, *index as isize, 0))
        }
        Term::Const(name) => env
            .get(name)
            .map(|def| def.ty.clone())
            .ok_or_else(|| CompileError::new(format!("unknown constant '{name}'"))),
        Term::Pi { name, ty, body } => {
            check(env, ctx, ty, &Term::Sort)?;
            with_local(ctx, name.clone(), ty, |ctx| {
                check(env, ctx, body, &Term::Sort)
            })?;
            Ok(Term::Sort)
        }
        Term::Lam { name, ty, body } => {
            check(env, ctx, ty, &Term::Sort)?;
            let body_ty = with_local(ctx, name.clone(), ty, |ctx| infer(env, ctx, body))?;
            Ok(Term::Pi {
                name: name.clone(),
                ty: ty.clone(),
                body: Box::new(body_ty),
            })
        }
        Term::App(fun, arg) => {
            let fun_ty = normalize(env, &infer(env, ctx, fun)?);
            match fun_ty {
                Term::Pi { ty, body, .. } => {
                    check(env, ctx, arg, &ty)?;
                    Ok(subst_top(arg, &body))
                }
                other => Err(CompileError::new(format!(
                    "cannot apply non-function of type {}",
                    other.pretty()
                ))),
            }
        }
        Term::Add(left, right) => {
            check(env, ctx, left, &Term::NatType)?;
            check(env, ctx, right, &Term::NatType)?;
            Ok(Term::NatType)
        }
        Term::Match {
            scrutinee,
            branches,
        } => {
            let scrutinee_ty = normalize(env, &infer(env, ctx, scrutinee)?);
            let mut result_ty = None;
            let mut seen = Vec::new();

            for branch in branches {
                let bindings = pattern_bindings(&branch.pattern, &scrutinee_ty)?;
                seen.push(branch.pattern.clone());
                for binding in &bindings {
                    ctx.push(Binding {
                        name: binding.name.clone(),
                        ty: shift(&binding.ty, 1, 0),
                    });
                }
                let branch_ty = infer(env, ctx, &branch.body)?;
                for _ in &bindings {
                    ctx.pop();
                }

                if let Some(expected) = &result_ty {
                    if !equivalent(env, &branch_ty, expected) {
                        return Err(CompileError::new(format!(
                            "match branch type mismatch: expected {}, got {}",
                            expected.pretty(),
                            branch_ty.pretty()
                        )));
                    }
                } else {
                    result_ty = Some(branch_ty);
                }
            }

            ensure_covered(&scrutinee_ty, &seen)?;
            result_ty.ok_or_else(|| CompileError::new("match needs at least one branch"))
        }
        Term::Let {
            name,
            ty,
            value,
            body,
        } => {
            check(env, ctx, ty, &Term::Sort)?;
            check(env, ctx, value, ty)?;
            let body_ty = with_local(ctx, name.clone(), ty, |ctx| infer(env, ctx, body))?;
            Ok(subst_top(value, &body_ty))
        }
    }
}

fn pattern_bindings(pattern: &Pattern, scrutinee_ty: &Term) -> Result<Vec<Binding>> {
    match pattern {
        Pattern::Wildcard => Ok(Vec::new()),
        Pattern::Var(name) => Ok(vec![Binding {
            name: name.clone(),
            ty: scrutinee_ty.clone(),
        }]),
        Pattern::NatLit(_) => {
            expect_type(scrutinee_ty, &Term::NatType, "Nat literal pattern")?;
            Ok(Vec::new())
        }
        Pattern::Ctor { name, binders } if name == "zero" => {
            expect_type(scrutinee_ty, &Term::NatType, "zero pattern")?;
            if binders.is_empty() {
                Ok(Vec::new())
            } else {
                Err(CompileError::new("zero pattern does not bind values"))
            }
        }
        Pattern::Ctor { name, binders } if name == "succ" => {
            expect_type(scrutinee_ty, &Term::NatType, "succ pattern")?;
            if binders.len() == 1 {
                Ok(vec![Binding {
                    name: binders[0].clone(),
                    ty: Term::NatType,
                }])
            } else {
                Err(CompileError::new("succ pattern binds exactly one value"))
            }
        }
        Pattern::Ctor { name, binders } if name == TRUE || name == FALSE => {
            expect_type(scrutinee_ty, &bool_ty(), "Bool constructor pattern")?;
            if binders.is_empty() {
                Ok(Vec::new())
            } else {
                Err(CompileError::new(format!(
                    "{name} pattern does not bind values"
                )))
            }
        }
        Pattern::Ctor { name, binders } if name == NONE => {
            option_arg(scrutinee_ty)?;
            if binders.is_empty() {
                Ok(Vec::new())
            } else {
                Err(CompileError::new("none pattern does not bind values"))
            }
        }
        Pattern::Ctor { name, binders } if name == SOME => {
            let inner = option_arg(scrutinee_ty)?;
            if binders.len() == 1 {
                Ok(vec![Binding {
                    name: binders[0].clone(),
                    ty: inner,
                }])
            } else {
                Err(CompileError::new("some pattern binds exactly one value"))
            }
        }
        Pattern::Ctor { name, binders } if name == NIL => {
            list_arg(scrutinee_ty)?;
            if binders.is_empty() {
                Ok(Vec::new())
            } else {
                Err(CompileError::new("nil pattern does not bind values"))
            }
        }
        Pattern::Ctor { name, binders } if name == CONS => {
            let inner = list_arg(scrutinee_ty)?;
            if binders.len() == 2 {
                Ok(vec![
                    Binding {
                        name: binders[0].clone(),
                        ty: inner.clone(),
                    },
                    Binding {
                        name: binders[1].clone(),
                        ty: Term::App(Box::new(Term::Const(LIST.to_string())), Box::new(inner)),
                    },
                ])
            } else {
                Err(CompileError::new("cons pattern binds exactly two values"))
            }
        }
        Pattern::Ctor { name, .. } => Err(CompileError::new(format!(
            "unknown pattern constructor '{name}'"
        ))),
    }
}

fn ensure_covered(scrutinee_ty: &Term, seen: &[Pattern]) -> Result<()> {
    if seen
        .iter()
        .any(|p| matches!(p, Pattern::Wildcard | Pattern::Var(_)))
    {
        return Ok(());
    }
    if *scrutinee_ty == bool_ty() {
        has_ctor(seen, TRUE)
            .then_some(())
            .ok_or_else(|| CompileError::new("missing true branch"))?;
        has_ctor(seen, FALSE)
            .then_some(())
            .ok_or_else(|| CompileError::new("missing false branch"))?;
    } else if *scrutinee_ty == Term::NatType {
        has_ctor(seen, "zero")
            .then_some(())
            .ok_or_else(|| CompileError::new("missing zero branch"))?;
        has_ctor(seen, "succ")
            .then_some(())
            .ok_or_else(|| CompileError::new("missing succ branch"))?;
    } else if option_arg(scrutinee_ty).is_ok() {
        has_ctor(seen, NONE)
            .then_some(())
            .ok_or_else(|| CompileError::new("missing none branch"))?;
        has_ctor(seen, SOME)
            .then_some(())
            .ok_or_else(|| CompileError::new("missing some branch"))?;
    } else if list_arg(scrutinee_ty).is_ok() {
        has_ctor(seen, NIL)
            .then_some(())
            .ok_or_else(|| CompileError::new("missing nil branch"))?;
        has_ctor(seen, CONS)
            .then_some(())
            .ok_or_else(|| CompileError::new("missing cons branch"))?;
    }
    Ok(())
}

fn has_ctor(patterns: &[Pattern], ctor: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| matches!(pattern, Pattern::Ctor { name, .. } if name == ctor))
}

fn expect_type(actual: &Term, expected: &Term, label: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(CompileError::new(format!(
            "{label} expected scrutinee type {}, got {}",
            expected.pretty(),
            actual.pretty()
        )))
    }
}

fn option_arg(term: &Term) -> Result<Term> {
    unary_type_arg(term, OPTION)
}

fn list_arg(term: &Term) -> Result<Term> {
    unary_type_arg(term, LIST)
}

fn unary_type_arg(term: &Term, name: &str) -> Result<Term> {
    match term {
        Term::App(fun, arg) if matches!(**fun, Term::Const(ref const_name) if const_name == name) => {
            Ok((**arg).clone())
        }
        _ => Err(CompileError::new(format!(
            "expected {name} application, got {}",
            term.pretty()
        ))),
    }
}

pub fn check(env: &Env, ctx: &mut Context, term: &Term, expected: &Term) -> Result<()> {
    let actual = infer(env, ctx, term)?;
    if equivalent(env, &actual, expected) {
        Ok(())
    } else {
        Err(CompileError::new(format!(
            "type mismatch: expected {}, got {}",
            expected.pretty(),
            actual.pretty()
        )))
    }
}

fn with_local<T>(
    ctx: &mut Context,
    name: String,
    ty: &Term,
    f: impl FnOnce(&mut Context) -> Result<T>,
) -> Result<T> {
    ctx.push(Binding {
        name,
        ty: shift(ty, 1, 0),
    });
    let result = f(ctx);
    ctx.pop();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Term;

    #[test]
    fn infers_identity_type() {
        let env = Env::new();
        let term = Term::Lam {
            name: "A".to_string(),
            ty: Box::new(Term::Sort),
            body: Box::new(Term::Lam {
                name: "x".to_string(),
                ty: Box::new(Term::Var(0)),
                body: Box::new(Term::Var(0)),
            }),
        };
        let ty = infer(&env, &mut Vec::new(), &term).unwrap();
        assert_eq!(ty.pretty(), "(A : Type) -> (x : A) -> A");
    }

    #[test]
    fn infers_nat_addition() {
        let env = Env::new();
        let term = Term::Add(Box::new(Term::NatLit(1)), Box::new(Term::NatLit(2)));
        let ty = infer(&env, &mut Vec::new(), &term).unwrap();
        assert_eq!(ty, Term::NatType);
    }
}
