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
