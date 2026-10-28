//! Parsing statements.

use super::{PResult, Parser};
use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::token::{Keyword, TokenKind};

impl Parser {
    /// `stmt ; stmt ; ...`. Pascal's semicolon is a *separator*, so the list
    /// ends at the first statement not followed by `;`.
    pub(super) fn statement_list(&mut self) -> PResult<Vec<Stmt>> {
        let mut stmts = vec![self.statement()?];
        while self.eat(&TokenKind::Semi) {
            stmts.push(self.statement()?);
        }
        // Drop the empty statements produced by `;` before `end`: they mean
        // nothing and would only clutter the tree.
        stmts.retain(|s| s.kind != StmtKind::Empty);
        Ok(stmts)
    }

    pub(super) fn statement(&mut self) -> PResult<Stmt> {
        let start = self.span();
        let kind = match self.peek() {
            TokenKind::Keyword(Keyword::Begin) => {
                self.bump();
                let body = self.statement_list()?;
                self.expect_keyword(Keyword::End)?;
                StmtKind::Compound(body)
            }
            TokenKind::Keyword(Keyword::If) => {
                self.bump();
                let cond = self.expression()?;
                self.expect_keyword(Keyword::Then)?;
                let then_branch = Box::new(self.statement()?);
                // The "dangling else": in `if a then if b then x else y`, the
                // `else` belongs to the nearest `if`. Greedily taking any
                // `else` we see right here gives exactly that rule.
                let else_branch =
                    if self.eat_keyword(Keyword::Else) { Some(Box::new(self.statement()?)) } else { None };
                StmtKind::If { cond, then_branch, else_branch }
            }
            TokenKind::Keyword(Keyword::While) => {
                self.bump();
                let cond = self.expression()?;
                self.expect_keyword(Keyword::Do)?;
                StmtKind::While { cond, body: Box::new(self.statement()?) }
            }
            TokenKind::Keyword(Keyword::Repeat) => {
                self.bump();
                let body = self.statement_list()?;
                self.expect_keyword(Keyword::Until)?;
                StmtKind::Repeat { body, cond: self.expression()? }
            }
            TokenKind::Keyword(Keyword::For) => {
                self.bump();
                let var = self.expect_ident()?;
                self.expect(&TokenKind::Assign)?;
                let start = self.expression()?;
                let down = if self.eat_keyword(Keyword::Downto) {
                    true
                } else {
                    self.expect_keyword(Keyword::To)?;
                    false
                };
                let end = self.expression()?;
                self.expect_keyword(Keyword::Do)?;
                StmtKind::For { var, start, end, down, body: Box::new(self.statement()?) }
            }
            TokenKind::Keyword(Keyword::Case) => self.case_statement()?,
            TokenKind::Keyword(Keyword::With) => {
                self.bump();
                let mut records = vec![self.designator()?];
                while self.eat(&TokenKind::Comma) {
                    records.push(self.designator()?);
                }
                self.expect_keyword(Keyword::Do)?;
                StmtKind::With { records, body: Box::new(self.statement()?) }
            }
            TokenKind::Keyword(Keyword::Goto) => {
                return Err(Diagnostic::new(start, "`goto` is not supported"));
            }
            TokenKind::Ident(_) | TokenKind::Keyword(Keyword::Inherited) => {
                let target = self.designator()?;
                if self.eat(&TokenKind::Assign) {
                    StmtKind::Assign { target, value: self.expression()? }
                } else {
                    StmtKind::Call(target)
                }
            }
            // Anything else (`;`, `end`, `until`, `else`) means this
            // statement is empty.
            _ => StmtKind::Empty,
        };
        let span =
            if kind == StmtKind::Empty { Span::new(start.start, start.start) } else { start.to(self.prev_span()) };
        Ok(Stmt { kind, span })
    }

    fn case_statement(&mut self) -> PResult<StmtKind> {
        self.expect_keyword(Keyword::Case)?;
        let scrutinee = self.expression()?;
        self.expect_keyword(Keyword::Of)?;
        let mut arms = Vec::new();
        let mut else_body = None;
        loop {
            if self.at_keyword(Keyword::End) {
                break;
            }
            if self.eat_keyword(Keyword::Else) {
                else_body = Some(self.statement_list()?);
                break;
            }
            let mut labels = vec![self.case_label()?];
            while self.eat(&TokenKind::Comma) {
                labels.push(self.case_label()?);
            }
            self.expect(&TokenKind::Colon)?;
            let body = self.statement()?;
            arms.push(CaseArm { labels, body });
            // The `;` after the last arm is optional.
            if !self.eat(&TokenKind::Semi) && !self.at_keyword(Keyword::Else) {
                break;
            }
        }
        self.expect_keyword(Keyword::End)?;
        Ok(StmtKind::Case { scrutinee, arms, else_body })
    }

    fn case_label(&mut self) -> PResult<CaseLabel> {
        let lo = self.expression()?;
        if self.eat(&TokenKind::DotDot) {
            Ok(CaseLabel::Range(lo, self.expression()?))
        } else {
            Ok(CaseLabel::Value(lo))
        }
    }
}
