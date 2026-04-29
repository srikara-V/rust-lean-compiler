#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Def { name: String, ty: Term, value: Term },
    Eval(Term),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Type,
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
}
