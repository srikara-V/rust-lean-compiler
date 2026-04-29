use crate::builtins::{CONS, FALSE, NIL, NONE, SOME, SUCC, TRUE};
use crate::core::{subst_top, Env, Pattern, Term};

const NORMALIZE_FUEL: usize = 10_000;

pub fn normalize(env: &Env, term: &Term) -> Term {
    normalize_with_fuel(env, term, NORMALIZE_FUEL)
}

fn normalize_with_fuel(env: &Env, term: &Term, fuel: usize) -> Term {
    if fuel == 0 {
        return term.clone();
    }

    match term {
        Term::Sort => Term::Sort,
        Term::NatType => Term::NatType,
        Term::NatLit(value) => Term::NatLit(*value),
        Term::Var(index) => Term::Var(*index),
        Term::Const(name) => env
            .get(name)
            .and_then(|def| def.value.as_ref())
            .map(|value| normalize_with_fuel(env, value, fuel - 1))
            .unwrap_or_else(|| Term::Const(name.clone())),
        Term::Pi { name, ty, body } => Term::Pi {
            name: name.clone(),
            ty: Box::new(normalize_with_fuel(env, ty, fuel - 1)),
            body: Box::new(normalize_with_fuel(env, body, fuel - 1)),
        },
        Term::Lam { name, ty, body } => Term::Lam {
            name: name.clone(),
            ty: Box::new(normalize_with_fuel(env, ty, fuel - 1)),
            body: Box::new(normalize_with_fuel(env, body, fuel - 1)),
        },
        Term::App(fun, arg) => {
            let fun = normalize_with_fuel(env, fun, fuel - 1);
            let arg = normalize_with_fuel(env, arg, fuel - 1);
            match fun {
                Term::Lam { body, .. } => {
                    normalize_with_fuel(env, &subst_top(&arg, &body), fuel - 1)
                }
                Term::Const(name) if name == SUCC => match arg {
                    Term::NatLit(value) => Term::NatLit(value + 1),
                    other => Term::App(Box::new(Term::Const(name)), Box::new(other)),
                },
                other => Term::App(Box::new(other), Box::new(arg)),
            }
        }
        Term::Add(left, right) => {
            let left = normalize_with_fuel(env, left, fuel - 1);
            let right = normalize_with_fuel(env, right, fuel - 1);
            match (&left, &right) {
                (Term::NatLit(left), Term::NatLit(right)) => Term::NatLit(left + right),
                _ => Term::Add(Box::new(left), Box::new(right)),
            }
        }
        Term::Match {
            scrutinee,
            branches,
        } => {
            let scrutinee = normalize_with_fuel(env, scrutinee, fuel - 1);
            for branch in branches {
                if let Some(values) = match_pattern(&branch.pattern, &scrutinee) {
                    let mut body = branch.body.clone();
                    for value in values.iter().rev() {
                        body = subst_top(value, &body);
                    }
                    return normalize_with_fuel(env, &body, fuel - 1);
                }
            }
            Term::Match {
                scrutinee: Box::new(scrutinee),
                branches: branches.clone(),
            }
        }
        Term::Let { value, body, .. } => {
            let value = normalize_with_fuel(env, value, fuel - 1);
            normalize_with_fuel(env, &subst_top(&value, body), fuel - 1)
        }
    }
}

fn match_pattern(pattern: &Pattern, value: &Term) -> Option<Vec<Term>> {
    match pattern {
        Pattern::Wildcard => Some(Vec::new()),
        Pattern::Var(_) => Some(vec![value.clone()]),
        Pattern::NatLit(expected) => match value {
            Term::NatLit(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        Pattern::Ctor { name, .. } if name == "zero" => match value {
            Term::NatLit(0) => Some(Vec::new()),
            _ => None,
        },
        Pattern::Ctor { name, binders } if name == "succ" && binders.len() == 1 => match value {
            Term::NatLit(actual) if *actual > 0 => Some(vec![Term::NatLit(actual - 1)]),
            _ => None,
        },
        Pattern::Ctor { name, binders } => {
            let (head, args) = collect_app(value);
            match head {
                Term::Const(head_name) if head_name == name => {
                    constructor_bindings(name, binders, &args)
                }
                _ => None,
            }
        }
    }
}

fn constructor_bindings(name: &str, binders: &[String], args: &[Term]) -> Option<Vec<Term>> {
    match name {
        TRUE | FALSE if binders.is_empty() && args.is_empty() => Some(Vec::new()),
        NONE | NIL if binders.is_empty() && args.len() == 1 => Some(Vec::new()),
        SOME if binders.len() == 1 && args.len() == 2 => Some(vec![args[1].clone()]),
        CONS if binders.len() == 2 && args.len() == 3 => {
            Some(vec![args[1].clone(), args[2].clone()])
        }
        _ => None,
    }
}

fn collect_app(term: &Term) -> (&Term, Vec<Term>) {
    let mut args = Vec::new();
    let mut current = term;
    while let Term::App(fun, arg) = current {
        args.push((**arg).clone());
        current = fun;
    }
    args.reverse();
    (current, args)
}

pub fn equivalent(env: &Env, left: &Term, right: &Term) -> bool {
    alpha_eq(&normalize(env, left), &normalize(env, right))
}

fn alpha_eq(left: &Term, right: &Term) -> bool {
    match (left, right) {
        (Term::Sort, Term::Sort) => true,
        (Term::NatType, Term::NatType) => true,
        (Term::NatLit(left), Term::NatLit(right)) => left == right,
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
        (Term::Add(left_lhs, left_rhs), Term::Add(right_lhs, right_rhs)) => {
            alpha_eq(left_lhs, right_lhs) && alpha_eq(left_rhs, right_rhs)
        }
        (
            Term::Match {
                scrutinee: left_scrutinee,
                branches: left_branches,
            },
            Term::Match {
                scrutinee: right_scrutinee,
                branches: right_branches,
            },
        ) => {
            alpha_eq(left_scrutinee, right_scrutinee)
                && left_branches.len() == right_branches.len()
                && left_branches
                    .iter()
                    .zip(right_branches)
                    .all(|(left, right)| {
                        left.pattern == right.pattern && alpha_eq(&left.body, &right.body)
                    })
        }
        (
            Term::Let {
                ty: left_ty,
                value: left_value,
                body: left_body,
                ..
            },
            Term::Let {
                ty: right_ty,
                value: right_value,
                body: right_body,
                ..
            },
        ) => {
            alpha_eq(left_ty, right_ty)
                && alpha_eq(left_value, right_value)
                && alpha_eq(left_body, right_body)
        }
        _ => false,
    }
}
