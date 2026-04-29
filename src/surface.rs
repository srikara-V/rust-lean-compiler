#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Def { name: String, ty: Term, value: Term },
    Eval(Term),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Type,
    Nat,
    Number(u64),
    Ident(String),
    Lam {
        name: String,
        ty: Box<Term>,
        body: Box<Term>,
    },
    Pi {
        name: Option<String>,
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
    Number(u64),
    Ctor { name: String, binders: Vec<String> },
}
