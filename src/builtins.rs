use crate::core::{Definition, Env, Term};

pub const BOOL: &str = "Bool";
pub const TRUE: &str = "true";
pub const FALSE: &str = "false";
pub const OPTION: &str = "Option";
pub const NONE: &str = "none";
pub const SOME: &str = "some";
pub const LIST: &str = "List";
pub const NIL: &str = "nil";
pub const CONS: &str = "cons";
pub const ZERO: &str = "zero";
pub const SUCC: &str = "succ";

pub fn initial_env() -> Env {
    let mut env = Env::new();

    env.insert(BOOL.to_string(), builtin(Term::Sort));
    env.insert(TRUE.to_string(), builtin(bool_ty()));
    env.insert(FALSE.to_string(), builtin(bool_ty()));
    env.insert(
        ZERO.to_string(),
        Definition {
            ty: Term::NatType,
            value: Some(Term::NatLit(0)),
        },
    );
    env.insert(
        SUCC.to_string(),
        builtin(pi("_", Term::NatType, Term::NatType)),
    );

    env.insert(OPTION.to_string(), builtin(pi("A", Term::Sort, Term::Sort)));
    env.insert(
        NONE.to_string(),
        builtin(pi(
            "A",
            Term::Sort,
            app(Term::Const(OPTION.to_string()), Term::Var(0)),
        )),
    );
    env.insert(
        SOME.to_string(),
        builtin(pi(
            "A",
            Term::Sort,
            pi(
                "x",
                Term::Var(0),
                app(Term::Const(OPTION.to_string()), Term::Var(1)),
            ),
        )),
    );

    env.insert(LIST.to_string(), builtin(pi("A", Term::Sort, Term::Sort)));
    env.insert(
        NIL.to_string(),
        builtin(pi(
            "A",
            Term::Sort,
            app(Term::Const(LIST.to_string()), Term::Var(0)),
        )),
    );
    env.insert(
        CONS.to_string(),
        builtin(pi(
            "A",
            Term::Sort,
            pi(
                "head",
                Term::Var(0),
                pi(
                    "tail",
                    app(Term::Const(LIST.to_string()), Term::Var(1)),
                    app(Term::Const(LIST.to_string()), Term::Var(2)),
                ),
            ),
        )),
    );

    env
}

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        BOOL | TRUE | FALSE | OPTION | NONE | SOME | LIST | NIL | CONS | ZERO | SUCC
    )
}

pub fn bool_ty() -> Term {
    Term::Const(BOOL.to_string())
}

fn builtin(ty: Term) -> Definition {
    Definition { ty, value: None }
}

fn pi(name: &str, ty: Term, body: Term) -> Term {
    Term::Pi {
        name: name.to_string(),
        ty: Box::new(ty),
        body: Box::new(body),
    }
}

pub fn app(fun: Term, arg: Term) -> Term {
    Term::App(Box::new(fun), Box::new(arg))
}
