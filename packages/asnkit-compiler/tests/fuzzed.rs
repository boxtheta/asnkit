// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! What the fuzzing of the compiler found (`fuzz/`): specifications on
//! which it panicked, ran out of stack or had no end, which are to be an
//! error or compiled.

use asnkit_compiler::{Input, compile, compile_to_asni};

fn module(types: &str) -> Input {
    Input::String(format!(
        "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n{types}\nEND\n"
    ))
}

#[test]
fn numbers_with_zeros_before_them_are_the_numbers() {
    // A number of a tag, of an object identifier, of a named number or
    // bit and of an enumeration panicked with zeros before it, which
    // values have always taken as int() of Python does.
    for (zeros, number) in [
        ("A ::= [00] INTEGER", "A ::= [0] INTEGER"),
        (
            "A ::= [APPLICATION 007] INTEGER",
            "A ::= [APPLICATION 7] INTEGER",
        ),
        (
            "id OBJECT IDENTIFIER ::= { 1 3 6 1 04 1 }",
            "id OBJECT IDENTIFIER ::= { 1 3 6 1 4 1 }",
        ),
        (
            "A ::= INTEGER { a(01), b(-02) }",
            "A ::= INTEGER { a(1), b(-2) }",
        ),
        (
            "A ::= BIT STRING { a(00), b(03) }",
            "A ::= BIT STRING { a(0), b(3) }",
        ),
        ("A ::= ENUMERATED { a(010) }", "A ::= ENUMERATED { a(10) }"),
    ] {
        assert_eq!(
            compile(&module(zeros)).unwrap(),
            compile(&module(number)).unwrap(),
            "{zeros}"
        );
    }
}

#[test]
fn minus_zero_is_zero() {
    // A named number of -0 was written as -1 - 0, which panicked.
    assert_eq!(
        compile(&module("A ::= INTEGER { m(-0) }")).unwrap(),
        compile(&module("A ::= INTEGER { m(0) }")).unwrap()
    );
    assert!(compile_to_asni(&module("A ::= INTEGER { m(-0), n(-00) }")).is_ok());
}

#[test]
fn a_name_imported_from_where_it_is_imported_is_not_found() {
    // These were followed until the stack ran out.
    let error = compile(&Input::String(
        "X DEFINITIONS ::= BEGIN IMPORTS T FROM X; A ::= SEQUENCE { a T } END\n".to_string(),
    ))
    .unwrap_err();

    assert_eq!(error.kind, "CompileError");
    assert_eq!(
        error.message,
        "Type 'T' imported by module 'X' not found in module 'X'."
    );

    let error = compile(&Input::String(
        "X DEFINITIONS ::= BEGIN IMPORTS T FROM Y; A ::= T END\n\
         Y DEFINITIONS ::= BEGIN IMPORTS T FROM X; B ::= INTEGER END\n"
            .to_string(),
    ))
    .unwrap_err();

    assert_eq!(error.kind, "CompileError");
    assert_eq!(
        error.message,
        "Type 'T' imported by module 'X' not found in module 'Y'."
    );
}

#[test]
fn a_real_too_large_for_a_double_is_an_error() {
    // It was infinite, which is no number of JSON, and panicked.
    for types in [
        "A ::= REAL (0..1.e320)",
        "r REAL ::= -1.5e400",
        "r REAL ::= 1.0e309",
    ] {
        let error = compile(&module(types)).unwrap_err();

        assert_eq!(error.kind, "ParseError", "{types}");
    }

    assert!(compile(&module("r REAL ::= 1.0e308")).is_ok());
}

/// `f`, on a stack of the size of that of the main thread, which
/// `asnkitcc` compiles on, and not of the 2 MiB of a test.
fn on_main_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn types_in_constraints_are_parsed_once() {
    // Each type in a constraint was parsed as a value, as a bound and
    // as a type, and those in it three times as many, which took more
    // than ten seconds at 12 levels and doubled the depth for each level
    // or so. They are errors at once.
    let started = std::time::Instant::now();

    for types in [
        "E ::= A(A(A(A(A(A(A(A(A(A(A(A(A(A(A(A(A(A(A(A(",
        "E ::= INTEGER (A(X(A(X(A(X(A(X(A(X(A(X(A(X(A(X(A(X(A(X(",
        "E ::= SESYTY(NT(YNPTY(TE-I-IDEXTERNAT(A(X(L(PORA(X((NULL(PjT(zR",
    ] {
        assert_eq!(compile(&module(types)).unwrap_err().kind, "ParseError");
    }

    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn what_is_nested_too_deeply_is_an_error() {
    let shapes: [fn(usize) -> String; 4] = [
        |n| format!("A ::= INTEGER {}1{}", "(".repeat(n), ")".repeat(n)),
        |n| {
            format!(
                "A ::= {}INTEGER{}",
                "SEQUENCE { a ".repeat(n),
                " }".repeat(n)
            )
        },
        |n| format!("v A ::= {}1", "a : ".repeat(n)),
        |n| format!("E ::= {}", "A(X(".repeat(n)),
    ];

    on_main_stack(move || {
        for shape in shapes {
            // These ran out of stack, which aborted the program.
            for depth in [65, 1000, 100_000] {
                let error = compile(&module(&shape(depth))).unwrap_err();

                assert_eq!(error.kind, "ParseError");
                assert!(
                    error.message.starts_with(
                        "Types, constraints and values are nested more than 64 levels deep, \
                         at line 2, column "
                    ),
                    "{}",
                    error.message
                );
            }
        }

        // As deep as that is not.
        for shape in &shapes[..3] {
            assert!(compile(&module(&shape(63))).is_ok());
        }
    });
}

fn error_of(types: &str) -> asnkit_compiler::Error {
    compile(&module(types)).unwrap_err()
}

#[test]
fn set_with_no_braces_is_an_error() {
    // It was a reference to a type named SET, which is nowhere, and the
    // runtimes refused the file.
    for types in ["P ::= SET", "P ::= SEQUENCE { a SET }"] {
        let error = error_of(types);

        assert_eq!(error.kind, "ParseError", "{types}");
        assert!(
            error.message.ends_with("Expected SIZE or ( or OF or {."),
            "{}",
            error.message
        );
    }

    // A reference that starts with it is one still, as in OMA ULP.
    assert!(compile(&module("SET-Id ::= INTEGER\nP ::= SEQUENCE { a SET-Id }")).is_ok());
}

#[test]
fn a_field_of_an_object_class_that_is_not_found_is_an_error() {
    // The runtimes refused the file, with every codec.
    let error = error_of("I ::= C.&i");

    assert_eq!(error.kind, "CompileError");
    assert_eq!(error.message, "Object class 'C' not found in module 'Foo'.");

    let error = error_of("C ::= CLASS { &id INTEGER }\nI ::= C.&nope");

    assert_eq!(error.kind, "CompileError");
    assert_eq!(error.message, "Field 'C.&nope' not found in module 'Foo'.");

    assert!(
        compile(&module(
            "C ::= CLASS { &id INTEGER }\nI ::= SEQUENCE { a C.&id }"
        ))
        .is_ok()
    );
}
