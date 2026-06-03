use rust_lean_compiler::Session;

#[test]
fn evaluates_definitions_through_the_full_pipeline() {
    let mut session = Session::new();
    let output = session
        .run_source(
            r#"
def id : (A : Type) -> A -> A := fun A : Type => fun x : A => x
def apply : (A : Type) -> (A -> A) -> A -> A := fun A : Type => fun f : A -> A => fun x : A => f x

#eval apply Type (id Type) Type
"#,
        )
        .unwrap();

    assert_eq!(output, vec!["Type"]);
}

#[test]
fn reports_unknown_constants() {
    let mut session = Session::new();
    let error = session.run_source("#eval missing").unwrap_err();

    assert!(error.message.contains("unknown constant"));
}

#[test]
fn evaluates_nat_literals_addition_and_let_bindings() {
    let mut session = Session::new();
    let output = session
        .run_source(
            r#"
def two : Nat := 1 + 1
def add_two : Nat -> Nat := fun x : Nat => x + two

#eval add_two 40
#eval let x : Nat := 5 in x + two
"#,
        )
        .unwrap();

    assert_eq!(output, vec!["42", "7"]);
}

#[test]
fn rejects_addition_outside_nat() {
    let mut session = Session::new();
    let error = session.run_source("#eval Type + 1").unwrap_err();

    assert!(error.message.contains("type mismatch"));
}

#[test]
fn evaluates_bool_option_and_list_matches() {
    let mut session = Session::new();
    let output = session
        .run_source(
            r#"
def not : Bool -> Bool := fun b : Bool => match b with | true => false | false => true
def getOrZero : Option Nat -> Nat := fun x : Option Nat => match x with | none => 0 | some n => n
def lengthNat : List Nat -> Nat := fun xs : List Nat => match xs with | nil => 0 | cons x rest => 1 + lengthNat rest

#eval not false
#eval getOrZero (some Nat 5)
#eval lengthNat (cons Nat 1 (cons Nat 2 (nil Nat)))
"#,
        )
        .unwrap();

    assert_eq!(output, vec!["true", "5", "2"]);
}

#[test]
fn evaluates_structural_nat_recursion() {
    let mut session = Session::new();
    let output = session
        .run_source(
            r#"
def sumTo : Nat -> Nat := fun n : Nat => match n with | zero => 0 | succ k => n + sumTo k

#eval sumTo 3
"#,
        )
        .unwrap();

    assert_eq!(output, vec!["6"]);
}

#[test]
fn rejects_incomplete_bool_match() {
    let mut session = Session::new();
    let error = session
        .run_source("def bad : Bool -> Bool := fun b : Bool => match b with | true => false")
        .unwrap_err();

    assert!(error.message.contains("missing false branch"));
}

#[test]
fn rejects_duplicate_definitions() {
    let mut session = Session::new();
    let error = session
        .run_source(
            r#"
def x : Nat := 1
def x : Nat := 2
"#,
        )
        .unwrap_err();

    assert!(error.message.contains("duplicate definition"));
}

#[test]
fn rejects_builtin_redefinition() {
    let mut session = Session::new();
    let error = session
        .run_source("def Bool : Type := Type")
        .unwrap_err();

    assert!(error.message.contains("cannot redefine builtin"));
}

#[test]
fn evaluates_const_combinator() {
    let mut session = Session::new();
    let output = session
        .run_source(
            r#"
def const : (A : Type) -> (B : Type) -> A -> B -> A := fun A : Type => fun B : Type => fun x : A => fun y : B => x

#eval const Type Type Type Type
"#,
        )
        .unwrap();

    assert_eq!(output, vec!["Type"]);
}

    let mut session = Session::new();
    let error = session
        .run_source("def loop : Nat -> Nat := fun n : Nat => loop n")
        .unwrap_err();

    assert!(error.message.contains("not structurally recursive"));
}
