extern crate env_logger;
#[macro_use]
extern crate pretty_assertions;

extern crate gluon_base as base;
extern crate gluon_parser as parser;

mod support;

use base::ast::{Pattern, PatternField, TypedIdent};
use base::pos::{self, BytePos, Spanned};
use base::types::Type;

use parser::{Error, ParseErrors, TokenizeError};

use support::*;

// The expected tokens aren't very interesting since they may change fairly often
fn remove_expected(errors: ParseErrors) -> ParseErrors {
    let f = |mut err: Spanned<Error, _>| {
        match err.value {
            Error::UnexpectedToken(_, ref mut expected) |
            Error::UnexpectedEof(ref mut expected) => expected.clear(),
            _ => (),
        }
        err
    };
    ParseErrors::from(errors.into_iter().map(f).collect::<Vec<_>>())
}

#[test]
fn empty_input() {
    let _ = ::env_logger::init();

    let result = parse("");
    assert!(result.is_err());
    let (expr, err) = result.unwrap_err();
    assert_eq!(expr, Some(error()));

    let error = Error::UnexpectedEof(vec![]);
    let span = pos::span(BytePos::from(0), BytePos::from(0));
    assert_eq!(
        remove_expected(err),
        ParseErrors::from(vec![pos::spanned(span, error)])
    );
}

#[test]
fn missing_match_expr() {
    let _ = ::env_logger::init();

    let expr = r#"
    match with
    | x -> x
    "#;
    let result = parse(expr);
    assert!(result.is_err());
    let (expr, err) = result.unwrap_err();
    assert_eq!(
        expr,
        Some(case(
            error(),
            vec![(Pattern::Ident(TypedIdent::new(intern("x"))), id("x"))],
        ),)
    );

    let error = Error::UnexpectedToken("With".into(), vec![]);
    let span = pos::span(BytePos::from(11), BytePos::from(15));
    assert_eq!(
        remove_expected(err),
        ParseErrors::from(vec![pos::spanned(span, error)])
    );
}

#[test]
fn wrong_indent_expression() {
    let _ = ::env_logger::init();

    let result = parse(
        r#"
let y =
    let x = 1
    x
   2
y
"#,
    );
    let error = Error::UnexpectedToken("IntLiteral".into(), vec![]);
    let span = pos::span(BytePos::from(32), BytePos::from(32));
    let errors = ParseErrors::from(vec![pos::spanned(span, error)]);

    assert_eq!(result.map_err(|(_, err)| remove_expected(err)), Err(errors));
}

#[test]
fn unclosed_string() {
    let _ = ::env_logger::init();

    let result = parse(
        r#"
"abc
"#,
    );
    assert!(result.is_err());
}

#[test]
fn tokenizer_error_is_returned() {
    let _ = ::env_logger::init();

    let result = parse(
        r#"
12345678901234567890 test
"#,
    );

    let error = Error::Token(TokenizeError::NonParseableInt);
    let span = pos::span(BytePos::from(0), BytePos::from(0));
    let errors = ParseErrors::from(vec![pos::spanned(span, error)]);

    assert_eq!(result.map_err(|(_, err)| err), Err(errors));
}

#[test]
fn tokenizer_error_at_eof_is_returned() {
    let _ = ::env_logger::init();

    let result = parse(
        r#"
12345678901234567890
"#,
    );

    let error = Error::Token(TokenizeError::NonParseableInt);
    let span = pos::span(BytePos::from(0), BytePos::from(0));
    let errors = ParseErrors::from(vec![pos::spanned(span, error)]);

    assert_eq!(result.map_err(|(_, err)| err), Err(errors));
}

#[test]
fn no_infinite_loop_from_default_block() {
    let _ = ::env_logger::init();

    let result = parse(
        r#"
let x = 1

    x,
    y = 1
}
"#,
    );
    assert!(result.is_err());
}

#[test]
fn missing_pattern() {
    let _ = ::env_logger::init();

    let expr = r#"
    match 1 with
    | -> x
    "#;
    let result = parse(expr);
    assert!(result.is_err());
    let (expr, err) = result.unwrap_err();
    assert_eq!(expr, Some(case(int(1), vec![(Pattern::Error, id("x"))])));

    let error = Error::UnexpectedToken("RArrow".into(), vec![]);
    let span = pos::span(BytePos::from(24), BytePos::from(26));
    assert_eq!(
        remove_expected(err),
        ParseErrors::from(vec![pos::spanned(span, error)])
    );
}

#[test]
fn incomplete_alternative() {
    let _ = ::env_logger::init();

    let expr = r#"
    match 1 with
    | //
    "#;
    let result = parse(expr);
    assert!(result.is_err());
    let (expr, err) = result.unwrap_err();
    assert_eq!(expr, Some(case(int(1), vec![(Pattern::Error, error())])));

    let error = Error::UnexpectedToken("CloseBlock".into(), vec![]);
    let span = pos::span(BytePos::from(24), BytePos::from(26));
    assert_eq!(
        remove_expected(err),
        ParseErrors::from(vec![pos::spanned(span, error)])
    );
}

#[test]
fn incomplete_alternative_before_complete_alternative() {
    let _ = ::env_logger::init();

    let expr = r#"
    match 1 with
    | //
    | x -> x
    "#;
    let result = parse(expr);
    assert!(result.is_err());
    let (expr, err) = result.unwrap_err();
    assert_eq!(
        expr,
        Some(case(
            int(1),
            vec![
                (Pattern::Error, error()),
                (Pattern::Ident(TypedIdent::new(intern("x"))), id("x")),
            ],
        ),)
    );

    let error = Error::UnexpectedToken("Pipe".into(), vec![]);
    let span = pos::span(BytePos::from(24), BytePos::from(24));
    assert_eq!(
        remove_expected(err),
        ParseErrors::from(vec![pos::spanned(span, error)])
    );
}

#[test]
fn incomplete_alternative_with_partial_pattern() {
    let _ = ::env_logger::init();

    let expr = r#"
    match 1 with
    | { x = }
    "#;
    let result = parse(expr);
    assert!(result.is_err());
    let (expr, err) = result.unwrap_err();
    assert_eq!(
        expr,
        Some(case(
            int(1),
            vec![
                (
                    Pattern::Record {
                        typ: Type::hole(),
                        types: vec![],
                        fields: vec![
                            PatternField {
                                name: no_loc(intern("x")),
                                value: Some(no_loc(Pattern::Error)),
                            },
                        ],
                    },
                    error(),
                ),
            ],
        ),)
    );

    let errors = vec![
        no_loc(Error::UnexpectedToken("RBrace".into(), vec![])),
        no_loc(Error::UnexpectedToken("CloseBlock".into(), vec![])),
    ];
    assert_eq!(remove_expected(err), ParseErrors::from(errors));
}
