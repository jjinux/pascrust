//! Parsing expressions.
//!
//! Pascal has only four precedence levels, from loosest to tightest:
//!
//! | level          | operators                           | grammar rule |
//! |----------------|-------------------------------------|--------------|
//! | relational     | `= <> < <= > >= in`                 | `expression` |
//! | adding         | `+ - or xor`                        | `simple`     |
//! | multiplying    | `* / div mod and shl shr`           | `term`       |
//! | unary          | `not`, `@`, sign                    | `factor`     |
//!
//! Note that `and` binds as tightly as `*` and `or` as tightly as `+`, which
//! is why Pascal programmers write `(x > 0) and (x < 10)` with parentheses.

use super::{PResult, Parser};
use crate::ast::*;
use crate::token::{Keyword, TokenKind};

impl Parser {
    pub(crate) fn expression(&mut self) -> PResult<Expr> {
        let lhs = self.simple_expression()?;
        let op = match self.peek() {
            TokenKind::Eq => BinaryOp::Eq,
            TokenKind::NotEq => BinaryOp::NotEq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::LtEq => BinaryOp::LtEq,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::GtEq => BinaryOp::GtEq,
            TokenKind::Keyword(Keyword::In) => BinaryOp::In,
            _ => return Ok(lhs),
        };
        self.bump();
        // Relational operators don't chain: `a < b < c` is a type error in
        // Pascal, so there's no loop here.
        let rhs = self.simple_expression()?;
        Ok(self.binary(op, lhs, rhs))
    }

    /// An expression without relational operators. Subrange bounds use this,
    /// so that in `const a: integer = 5` the `=` isn't taken as a comparison.
    pub(super) fn simple_expression(&mut self) -> PResult<Expr> {
        let mut lhs = self.term()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                TokenKind::Keyword(Keyword::Or) => BinaryOp::Or,
                TokenKind::Keyword(Keyword::Xor) => BinaryOp::Xor,
                _ => return Ok(lhs),
            };
            self.bump();
            let rhs = self.term()?;
            lhs = self.binary(op, lhs, rhs);
        }
    }

    fn term(&mut self) -> PResult<Expr> {
        let mut lhs = self.factor()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::RealDiv,
                TokenKind::Keyword(Keyword::Div) => BinaryOp::Div,
                TokenKind::Keyword(Keyword::Mod) => BinaryOp::Mod,
                TokenKind::Keyword(Keyword::And) => BinaryOp::And,
                TokenKind::Keyword(Keyword::Shl) => BinaryOp::Shl,
                TokenKind::Keyword(Keyword::Shr) => BinaryOp::Shr,
                _ => return Ok(lhs),
            };
            self.bump();
            let rhs = self.factor()?;
            lhs = self.binary(op, lhs, rhs);
        }
    }

    fn binary(&mut self, op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
        let span = lhs.span.to(rhs.span);
        self.mk_expr(ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span)
    }

    fn factor(&mut self) -> PResult<Expr> {
        let start = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Integer(n) => {
                self.bump();
                ExprKind::Int(n)
            }
            TokenKind::Real(x) => {
                self.bump();
                ExprKind::Real(x)
            }
            TokenKind::Str(s) => {
                self.bump();
                ExprKind::Str(s)
            }
            TokenKind::Keyword(Keyword::Nil) => {
                self.bump();
                ExprKind::Nil
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.expression()?;
                self.expect(&TokenKind::RParen)?;
                // Parentheses only group; they don't need their own node.
                // Widen the span to include them for better error messages.
                return Ok(Expr { span: start.to(self.prev_span()), ..inner });
            }
            // Turbo Pascal allows a sign in front of any factor, so
            // `a * -b` is legal. ISO Pascal only allows it at the start of a
            // simple expression.
            TokenKind::Minus | TokenKind::Plus | TokenKind::Keyword(Keyword::Not) => {
                let op = match self.bump().kind {
                    TokenKind::Minus => UnaryOp::Neg,
                    TokenKind::Plus => UnaryOp::Plus,
                    _ => UnaryOp::Not,
                };
                let operand = self.factor()?;
                ExprKind::Unary { op, operand: Box::new(operand) }
            }
            TokenKind::At => {
                self.bump();
                ExprKind::AddrOf(Box::new(self.designator()?))
            }
            TokenKind::LBracket => self.set_constructor()?,
            TokenKind::Ident(_) => return self.designator(),
            _ => return Err(self.expected("an expression")),
        };
        Ok(self.mk_expr(kind, start.to(self.prev_span())))
    }

    /// `[1, 3..5, x]`
    fn set_constructor(&mut self) -> PResult<ExprKind> {
        self.expect(&TokenKind::LBracket)?;
        let mut elems = Vec::new();
        if !self.at(&TokenKind::RBracket) {
            loop {
                let lo = self.expression()?;
                if self.eat(&TokenKind::DotDot) {
                    elems.push(SetElem::Range(lo, self.expression()?));
                } else {
                    elems.push(SetElem::Single(lo));
                }
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RBracket)?;
        Ok(ExprKind::Set(elems))
    }

    /// A name followed by any number of selectors: `.field`, `[index]`, `^`
    /// and `(arguments)`. Examples: `grid[c, r]`, `p^.next^.value`,
    /// `Copy(s, 1, 3)`.
    pub(super) fn designator(&mut self) -> PResult<Expr> {
        let name = self.expect_ident()?;
        let span = name.span;
        let mut expr = self.mk_expr(ExprKind::Name(name), span);
        loop {
            let kind = match self.peek() {
                TokenKind::Dot => {
                    self.bump();
                    let field = self.expect_ident()?;
                    ExprKind::Field(Box::new(expr), field)
                }
                TokenKind::LBracket => {
                    self.bump();
                    let mut indexes = vec![self.expression()?];
                    while self.eat(&TokenKind::Comma) {
                        indexes.push(self.expression()?);
                    }
                    self.expect(&TokenKind::RBracket)?;
                    ExprKind::Index(Box::new(expr), indexes)
                }
                TokenKind::Caret => {
                    self.bump();
                    ExprKind::Deref(Box::new(expr))
                }
                TokenKind::LParen => {
                    let args = self.arguments()?;
                    ExprKind::Call { callee: Box::new(expr), args }
                }
                _ => return Ok(expr),
            };
            let span = span.to(self.prev_span());
            expr = self.mk_expr(kind, span);
        }
    }

    /// `(a, b:5, c:8:2)`. The `:width:decimals` suffixes are only meaningful
    /// for `Write`/`Writeln`/`Str`; sema rejects them elsewhere.
    fn arguments(&mut self) -> PResult<Vec<Expr>> {
        self.expect(&TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.eat(&TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            let value = self.expression()?;
            let arg = if self.eat(&TokenKind::Colon) {
                let width = self.expression()?;
                let decimals = if self.eat(&TokenKind::Colon) { Some(Box::new(self.expression()?)) } else { None };
                let span = value.span.to(self.prev_span());
                self.mk_expr(ExprKind::Format { value: Box::new(value), width: Box::new(width), decimals }, span)
            } else {
                value
            };
            args.push(arg);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;
        Ok(args)
    }
}
