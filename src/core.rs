use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Sort,
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
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub ty: Term,
    pub value: Term,
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
                Term::Lam { .. } | Term::Pi { .. } => format!("({})", pretty_with(fun, names)),
                _ => pretty_with(fun, names),
            };
            let arg = match **arg {
                Term::App(_, _) | Term::Lam { .. } | Term::Pi { .. } => {
                    format!("({})", pretty_with(arg, names))
                }
                _ => pretty_with(arg, names),
            };
            format!("{fun} {arg}")
        }
    }
}

pub fn shift(term: &Term, amount: isize, cutoff: usize) -> Term {
    match term {
        Term::Sort => Term::Sort,
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
    }
}

pub fn subst_top(replacement: &Term, body: &Term) -> Term {
    shift(&subst(body, 0, &shift(replacement, 1, 0), 0), -1, 0)
}

fn subst(term: &Term, index: usize, replacement: &Term, depth: usize) -> Term {
    match term {
        Term::Sort => Term::Sort,
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
