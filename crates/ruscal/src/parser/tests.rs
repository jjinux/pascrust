//! Parser unit tests.

use super::*;

fn parse(src: &str) -> Program {
    match parse_program(src.as_bytes()) {
        Ok(parsed) => parsed.program,
        Err(d) => panic!("{}", d.render("test.pas", src.as_bytes())),
    }
}

fn parse_err(src: &str) -> String {
    let d = parse_program(src.as_bytes()).expect_err("expected a parse error");
    d.render("test.pas", src.as_bytes())
}

#[test]
fn hello_world() {
    let p = parse("program Hello;\nbegin\n  writeln('Hello, world!');\nend.\n");
    assert_eq!(p.name.name, "Hello");
    assert_eq!(p.block.body.len(), 1);
    let StmtKind::Call(call) = &p.block.body[0].kind else { panic!() };
    let ExprKind::Call { args, .. } = &call.kind else { panic!() };
    assert_eq!(args[0].kind, ExprKind::Str(b"Hello, world!".to_vec()));
}

#[test]
fn program_header_is_optional() {
    let p = parse("begin end.");
    assert!(p.block.body.is_empty());
}

#[test]
fn precedence_and_binds_like_multiplication() {
    // `a and b = c` parses as `(a and b) = c` in Pascal.
    let p = parse("begin x := a and b = c end.");
    let StmtKind::Assign { value, .. } = &p.block.body[0].kind else { panic!() };
    let ExprKind::Binary { op: BinaryOp::Eq, lhs, .. } = &value.kind else { panic!("{value:?}") };
    assert!(matches!(lhs.kind, ExprKind::Binary { op: BinaryOp::And, .. }));
}

#[test]
fn dangling_else_binds_to_nearest_if() {
    let p = parse("begin if a then if b then x := 1 else x := 2 end.");
    let StmtKind::If { then_branch, else_branch, .. } = &p.block.body[0].kind else { panic!() };
    assert!(else_branch.is_none());
    assert!(matches!(then_branch.kind, StmtKind::If { else_branch: Some(_), .. }));
}

#[test]
fn write_formats() {
    let p = parse("begin writeln(x:5, y:8:2) end.");
    let StmtKind::Call(call) = &p.block.body[0].kind else { panic!() };
    let ExprKind::Call { args, .. } = &call.kind else { panic!() };
    assert!(matches!(&args[0].kind, ExprKind::Format { decimals: None, .. }));
    assert!(matches!(&args[1].kind, ExprKind::Format { decimals: Some(_), .. }));
}

#[test]
fn declarations() {
    let p = parse(
        "const max = 10; greeting = 'hi';
         type Grid = array[0..max, 1..5] of boolean;
         var a, b: integer; g: Grid;
         procedure P(var x: integer; y: char); forward;
         function F(n: integer): integer; begin F := n end;
         procedure P(var x: integer; y: char); begin end;
         begin end.",
    );
    assert_eq!(p.block.decls.len(), 8);
}

#[test]
fn typed_constants() {
    let p = parse("const a: array[1..3] of integer = (1, 2, 3); b: integer = (1 + 2) * 3; begin end.");
    let Decl::Const(a) = &p.block.decls[0] else { panic!() };
    assert!(matches!(&a.value, ConstValue::List(items, _) if items.len() == 3));
    let Decl::Const(b) = &p.block.decls[1] else { panic!() };
    assert!(matches!(&b.value, ConstValue::Expr(_)));
}

#[test]
fn case_with_else() {
    let p = parse("begin case c of 'a', 'b': x := 1; '0'..'9': x := 2 else x := 3 end end.");
    let StmtKind::Case { arms, else_body, .. } = &p.block.body[0].kind else { panic!() };
    assert_eq!(arms.len(), 2);
    assert!(else_body.is_some());
}

#[test]
fn error_messages_point_at_the_problem() {
    let err = parse_err("program X;\nbegin\n  writeln('hi')\n  writeln('there')\nend.\n");
    assert_eq!(err, "test.pas:4:3: error: expected `end`, found identifier `writeln`\n  writeln('there')\n  ^^^^^^^");
}
