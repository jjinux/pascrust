//! Parsing type expressions: `integer`, `array[1..10] of char`,
//! `record ... end`, `^Node`, `(red, green)`, `1..maxcol`, ...

use super::{PResult, Parser};
use crate::ast::*;
use crate::diag::Diagnostic;
use crate::token::{Keyword, TokenKind};

impl Parser {
    pub(super) fn type_expr(&mut self) -> PResult<TypeExpr> {
        let start = self.span();
        // `packed` asked old compilers to squeeze data into fewer bytes. It
        // doesn't change the meaning of a program, so we ignore it.
        self.eat_keyword(Keyword::Packed);
        match self.peek().clone() {
            TokenKind::Keyword(Keyword::Array) => {
                self.bump();
                self.expect(&TokenKind::LBracket)?;
                let mut indexes = vec![self.type_expr()?];
                while self.eat(&TokenKind::Comma) {
                    indexes.push(self.type_expr()?);
                }
                self.expect(&TokenKind::RBracket)?;
                self.expect_keyword(Keyword::Of)?;
                let elem = Box::new(self.type_expr()?);
                Ok(TypeExpr::Array { indexes, elem, span: start.to(self.prev_span()) })
            }
            TokenKind::Keyword(Keyword::Record) => {
                self.bump();
                let (fields, variant) = self.field_list()?;
                let end = self.expect_keyword(Keyword::End)?;
                Ok(TypeExpr::Record(fields, variant, start.to(end)))
            }
            TokenKind::Keyword(Keyword::String) => {
                self.bump();
                let len = if self.eat(&TokenKind::LBracket) {
                    let len = self.expression()?;
                    self.expect(&TokenKind::RBracket)?;
                    Some(Box::new(len))
                } else {
                    None
                };
                Ok(TypeExpr::String(len, start.to(self.prev_span())))
            }
            TokenKind::Caret => {
                self.bump();
                let target = self.expect_ident()?;
                Ok(TypeExpr::Pointer(target, start.to(self.prev_span())))
            }
            TokenKind::Keyword(Keyword::Set) => {
                self.bump();
                self.expect_keyword(Keyword::Of)?;
                let elem = self.type_expr()?;
                Ok(TypeExpr::Set(Box::new(elem), start.to(self.prev_span())))
            }
            TokenKind::Keyword(Keyword::File) => {
                self.bump();
                let elem = if self.eat_keyword(Keyword::Of) { Some(Box::new(self.type_expr()?)) } else { None };
                Ok(TypeExpr::File(elem, start.to(self.prev_span())))
            }
            TokenKind::LParen => {
                self.bump();
                let names = self.ident_list()?;
                let end = self.expect(&TokenKind::RParen)?;
                Ok(TypeExpr::Enum(names, start.to(end)))
            }
            TokenKind::Keyword(Keyword::Object) => {
                self.bump();
                let parent = if self.eat(&TokenKind::LParen) {
                    let parent = self.expect_ident()?;
                    self.expect(&TokenKind::RParen)?;
                    Some(parent)
                } else {
                    None
                };
                let mut fields = Vec::new();
                let mut methods = Vec::new();
                loop {
                    match self.peek() {
                        TokenKind::Ident(name)
                            if name.eq_ignore_ascii_case("private") || name.eq_ignore_ascii_case("public") =>
                        {
                            // Visibility sections don't change what a program
                            // does, so they're ignored.
                            self.bump();
                        }
                        TokenKind::Ident(_) => {
                            let names = self.ident_list()?;
                            self.expect(&TokenKind::Colon)?;
                            let ty = self.type_expr()?;
                            self.expect(&TokenKind::Semi)?;
                            fields.push(FieldDecl { names, ty });
                        }
                        TokenKind::Keyword(
                            Keyword::Procedure | Keyword::Function | Keyword::Constructor | Keyword::Destructor,
                        ) => methods.push(self.proc_decl(true)?),
                        _ => break,
                    }
                }
                let end = self.expect_keyword(Keyword::End)?;
                Ok(TypeExpr::Object { parent, fields, methods, span: start.to(end) })
            }

            _ => {
                // Either a type name or a subrange `lo..hi`. Both ends of a
                // subrange are constant expressions (`1..maxcol - 1`,
                // `'a'..'z'`), so parse an expression and see what follows.
                let lo = self.simple_expression()?;
                if self.eat(&TokenKind::DotDot) {
                    let hi = self.simple_expression()?;
                    return Ok(TypeExpr::Subrange {
                        lo: Box::new(lo),
                        hi: Box::new(hi),
                        span: start.to(self.prev_span()),
                    });
                }
                match lo.kind {
                    ExprKind::Name(ident) => Ok(TypeExpr::Named(ident)),
                    _ => Err(Diagnostic::new(lo.span, "expected a type")),
                }
            }
        }
    }

    /// The fields of a record, up to (not including) the closing `end` (or
    /// a variant's `)`), and the variant part at the end, if any.
    fn field_list(&mut self) -> PResult<(Vec<FieldDecl>, Option<Box<VariantDecl>>)> {
        let mut fields = Vec::new();
        while matches!(self.peek(), TokenKind::Ident(_)) {
            let names = self.ident_list()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.type_expr()?;
            fields.push(FieldDecl { names, ty });
            // The `;` after the last field is optional.
            if !self.eat(&TokenKind::Semi) {
                break;
            }
        }
        let variant = if self.at_keyword(Keyword::Case) { Some(Box::new(self.variant_part()?)) } else { None };
        Ok((fields, variant))
    }

    /// `case [Tag:] Type of Label, ...: (Fields); ...`, the variant part of
    /// a record.
    fn variant_part(&mut self) -> PResult<VariantDecl> {
        let start = self.expect_keyword(Keyword::Case)?;
        let first = self.expect_ident()?;
        let (tag, tag_type) =
            if self.eat(&TokenKind::Colon) { (Some(first), self.expect_ident()?) } else { (None, first) };
        self.expect_keyword(Keyword::Of)?;
        let mut variants = Vec::new();
        // Variants run to the record's `end` (or, nested, the `)` of the
        // variant this part is in); the `;` after the last one is optional.
        while !self.at_keyword(Keyword::End) && !self.at(&TokenKind::RParen) {
            let mut labels = vec![self.expression()?];
            while self.eat(&TokenKind::Comma) {
                labels.push(self.expression()?);
            }
            self.expect(&TokenKind::Colon)?;
            self.expect(&TokenKind::LParen)?;
            let (fields, variant) = self.field_list()?;
            self.expect(&TokenKind::RParen)?;
            variants.push(VariantArm { labels, fields, variant });
            if !self.eat(&TokenKind::Semi) {
                break;
            }
        }
        Ok(VariantDecl { tag, tag_type, variants, span: start.to(self.prev_span()) })
    }
}
