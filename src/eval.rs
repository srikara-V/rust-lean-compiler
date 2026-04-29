use crate::core::{subst_top, Env, Term};

pub fn normalize(env: &Env, term: &Term) -> Term {
    match term {
        Term::Sort => Term::Sort,
        Term::Var(index) => Term::Var(*index),
        Term::Const(name) => env
            .get(name)
            .map(|def| normalize(env, &def.value))
            .unwrap_or_else(|| Term::Const(name.clone())),
        Term::Pi { name, ty, body } => Term::Pi {
            name: name.clone(),
            ty: Box::new(normalize(env, ty)),
            body: Box::new(normalize(env, body)),
        },
        Term::Lam { name, ty, body } => Term::Lam {
            name: name.clone(),
            ty: Box::new(normalize(env, ty)),
            body: Box::new(normalize(env, body)),
        },
        Term::App(fun, arg) => {
            let fun = normalize(env, fun);
            let arg = normalize(env, arg);
            match fun {
                Term::Lam { body, .. } => normalize(env, &subst_top(&arg, &body)),
                other => Term::App(Box::new(other), Box::new(arg)),
            }
        }
    }
}

pub fn equivalent(env: &Env, left: &Term, right: &Term) -> bool {
    alpha_eq(&normalize(env, left), &normalize(env, right))
}

fn alpha_eq(left: &Term, right: &Term) -> bool {
    match (left, right) {
        (Term::Sort, Term::Sort) => true,
        (Term::Var(left), Term::Var(right)) => left == right,
        (Term::Const(left), Term::Const(right)) => left == right,
        (
            Term::Pi {
                ty: left_ty,
                body: left_body,
                ..
            },
            Term::Pi {
                ty: right_ty,
                body: right_body,
                ..
            },
        )
        | (
            Term::Lam {
                ty: left_ty,
                body: left_body,
                ..
            },
            Term::Lam {
                ty: right_ty,
                body: right_body,
                ..
            },
        ) => alpha_eq(left_ty, right_ty) && alpha_eq(left_body, right_body),
        (Term::App(left_fun, left_arg), Term::App(right_fun, right_arg)) => {
            alpha_eq(left_fun, right_fun) && alpha_eq(left_arg, right_arg)
        }
        _ => false,
    }
}
