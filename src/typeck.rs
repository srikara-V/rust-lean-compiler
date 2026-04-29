use crate::core::{shift, subst_top, Env, Term};
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
}
