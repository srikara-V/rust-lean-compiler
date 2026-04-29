use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Sort,
    NatType,
    NatLit(u64),
    Var(usize),
    Const(String),
    Pi {
        name: String,
        ty: Box<Term>,
        body: Box<Term>,
    },
    Lam {
        name: String,
        ty: Box<Term>,
        body: Box<Term>,
    },
    App(Box<Term>, Box<Term>),
    Add(Box<Term>, Box<Term>),
    Match {
        scrutinee: Box<Term>,
        branches: Vec<MatchBranch>,
    },
    Let {
        name: String,
        ty: Box<Term>,
        value: Box<Term>,
        body: Box<Term>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchBranch {
    pub pattern: Pattern,
    pub body: Term,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Wildcard,
    Var(String),
    NatLit(u64),
    Ctor { name: String, binders: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub ty: Term,
    pub value: Option<Term>,
}

pub type Env = HashMap<String, Definition>;

impl Term {
    pub fn pretty(&self) -> String {
        pretty_with(self, &mut Vec::new())
    }
}

fn pretty_with(term: &Term, names: &mut Vec<String>) -> String {
    match term {
        Term::Sort => "Type".to_string(),
        Term::NatType => "Nat".to_string(),
        Term::NatLit(value) => value.to_string(),
        Term::Var(index) => names
            .len()
            .checked_sub(index + 1)
            .and_then(|i| names.get(i))
            .cloned()
            .unwrap_or_else(|| format!("#{index}")),
        Term::Const(name) => name.clone(),
        Term::Pi { name, ty, body } => {
            let ty = pretty_with(ty, names);
            names.push(name.clone());
            let body = pretty_with(body, names);
            names.pop();
            if name == "_" {
                format!("{ty} -> {body}")
            } else {
                format!("({name} : {ty}) -> {body}")
            }
        }
        Term::Lam { name, ty, body } => {
            let ty = pretty_with(ty, names);
            names.push(name.clone());
            let body = pretty_with(body, names);
            names.pop();
            format!("fun {name} : {ty} => {body}")
        }
        Term::App(fun, arg) => {
            let fun = match **fun {
                Term::Lam { .. } | Term::Pi { .. } | Term::Let { .. } => {
                    format!("({})", pretty_with(fun, names))
                }
                _ => pretty_with(fun, names),
            };
            let arg = match **arg {
                Term::App(_, _)
                | Term::Add(_, _)
                | Term::Lam { .. }
                | Term::Pi { .. }
                | Term::Let { .. } => {
                    format!("({})", pretty_with(arg, names))
                }
                _ => pretty_with(arg, names),
            };
            format!("{fun} {arg}")
        }
        Term::Add(left, right) => format!(
            "{} + {}",
            pretty_with(left, names),
            pretty_with(right, names)
        ),
        Term::Match {
            scrutinee,
            branches,
        } => {
            let scrutinee = pretty_with(scrutinee, names);
            let branches = branches
                .iter()
                .map(|branch| {
                    format!(
                        "| {} => {}",
                        branch.pattern.pretty(),
                        pretty_with(&branch.body, names)
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("match {scrutinee} with {branches}")
        }
        Term::Let {
            name,
            ty,
            value,
            body,
        } => {
            let ty = pretty_with(ty, names);
            let value = pretty_with(value, names);
            names.push(name.clone());
            let body = pretty_with(body, names);
            names.pop();
            format!("let {name} : {ty} := {value}; {body}")
        }
    }
}

pub fn shift(term: &Term, amount: isize, cutoff: usize) -> Term {
    match term {
        Term::Sort => Term::Sort,
        Term::NatType => Term::NatType,
        Term::NatLit(value) => Term::NatLit(*value),
        Term::Var(index) => {
            if *index >= cutoff {
                let shifted = (*index as isize) + amount;
                assert!(shifted >= 0, "de Bruijn shift produced a negative index");
                Term::Var(shifted as usize)
            } else {
                Term::Var(*index)
            }
        }
        Term::Const(name) => Term::Const(name.clone()),
        Term::Pi { name, ty, body } => Term::Pi {
            name: name.clone(),
            ty: Box::new(shift(ty, amount, cutoff)),
            body: Box::new(shift(body, amount, cutoff + 1)),
        },
        Term::Lam { name, ty, body } => Term::Lam {
            name: name.clone(),
            ty: Box::new(shift(ty, amount, cutoff)),
            body: Box::new(shift(body, amount, cutoff + 1)),
        },
        Term::App(fun, arg) => Term::App(
            Box::new(shift(fun, amount, cutoff)),
            Box::new(shift(arg, amount, cutoff)),
        ),
        Term::Add(left, right) => Term::Add(
            Box::new(shift(left, amount, cutoff)),
            Box::new(shift(right, amount, cutoff)),
        ),
        Term::Match {
            scrutinee,
            branches,
        } => Term::Match {
            scrutinee: Box::new(shift(scrutinee, amount, cutoff)),
            branches: branches
                .iter()
                .map(|branch| MatchBranch {
                    pattern: branch.pattern.clone(),
                    body: shift(
                        &branch.body,
                        amount,
                        cutoff + branch.pattern.binding_count(),
                    ),
                })
                .collect(),
        },
        Term::Let {
            name,
            ty,
            value,
            body,
        } => Term::Let {
            name: name.clone(),
            ty: Box::new(shift(ty, amount, cutoff)),
            value: Box::new(shift(value, amount, cutoff)),
            body: Box::new(shift(body, amount, cutoff + 1)),
        },
    }
}

pub fn subst_top(replacement: &Term, body: &Term) -> Term {
    shift(&subst(body, 0, &shift(replacement, 1, 0), 0), -1, 0)
}

fn subst(term: &Term, index: usize, replacement: &Term, depth: usize) -> Term {
    match term {
        Term::Sort => Term::Sort,
        Term::NatType => Term::NatType,
        Term::NatLit(value) => Term::NatLit(*value),
        Term::Var(var) if *var == index + depth => shift(replacement, depth as isize, 0),
        Term::Var(var) => Term::Var(*var),
        Term::Const(name) => Term::Const(name.clone()),
        Term::Pi { name, ty, body } => Term::Pi {
            name: name.clone(),
            ty: Box::new(subst(ty, index, replacement, depth)),
            body: Box::new(subst(body, index, replacement, depth + 1)),
        },
        Term::Lam { name, ty, body } => Term::Lam {
            name: name.clone(),
            ty: Box::new(subst(ty, index, replacement, depth)),
            body: Box::new(subst(body, index, replacement, depth + 1)),
        },
        Term::App(fun, arg) => Term::App(
            Box::new(subst(fun, index, replacement, depth)),
            Box::new(subst(arg, index, replacement, depth)),
        ),
        Term::Add(left, right) => Term::Add(
            Box::new(subst(left, index, replacement, depth)),
            Box::new(subst(right, index, replacement, depth)),
        ),
        Term::Match {
            scrutinee,
            branches,
        } => Term::Match {
            scrutinee: Box::new(subst(scrutinee, index, replacement, depth)),
            branches: branches
                .iter()
                .map(|branch| MatchBranch {
                    pattern: branch.pattern.clone(),
                    body: subst(
                        &branch.body,
                        index,
                        replacement,
                        depth + branch.pattern.binding_count(),
                    ),
                })
                .collect(),
        },
        Term::Let {
            name,
            ty,
            value,
            body,
        } => Term::Let {
            name: name.clone(),
            ty: Box::new(subst(ty, index, replacement, depth)),
            value: Box::new(subst(value, index, replacement, depth)),
            body: Box::new(subst(body, index, replacement, depth + 1)),
        },
    }
}

impl Pattern {
    pub fn binding_count(&self) -> usize {
        match self {
            Pattern::Wildcard | Pattern::NatLit(_) => 0,
            Pattern::Var(_) => 1,
            Pattern::Ctor { binders, .. } => binders.len(),
        }
    }

    pub fn pretty(&self) -> String {
        match self {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Var(name) => name.clone(),
            Pattern::NatLit(value) => value.to_string(),
            Pattern::Ctor { name, binders } if binders.is_empty() => name.clone(),
            Pattern::Ctor { name, binders } => format!("{name} {}", binders.join(" ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_lambda_body() {
        let body = Term::Var(0);
        let replacement = Term::Sort;
        assert_eq!(subst_top(&replacement, &body), Term::Sort);
    }
}
