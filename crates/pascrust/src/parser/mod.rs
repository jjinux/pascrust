//! A handwritten recursive-descent parser.
//!
//! "Recursive descent" means there is one function per grammar rule, and
//! rules that contain other rules call each other's functions: `statement`
//! calls `expression`, which calls `term`, which calls `factor`, which may
//! call `expression` again for a parenthesized sub-expression. Niklaus Wirth
//! designed Pascal so that one token of lookahead is almost always enough to
//! decide which rule applies, which is what makes this style so pleasant for
//! Pascal.
//!
//! The parser is split across files by grammar area:
//! - `mod.rs` (this file): token helpers, programs and declarations
//! - `types.rs`: type expressions
//! - `stmt.rs`: statements
//! - `expr.rs`: expressions

mod expr;
mod stmt;
mod types;

use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::lexer;
use crate::token::{Keyword, Token, TokenKind};

/// Result type for parser functions. A type alias saves writing the error
/// type in every signature.
type PResult<T> = Result<T, Diagnostic>;

/// A compiler directive such as `{$I-}` and where it appeared.
#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub text: String,
    pub span: Span,
}

/// What the parser produces for one source file.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProgram {
    pub program: Program,
    pub directives: Vec<Directive>,
}

/// Lex and parse a complete program.
pub fn parse_program(source: &[u8]) -> PResult<ParsedProgram> {
    match parse_file(source, 0)? {
        (SourceFile::Program(program), directives, _) => Ok(ParsedProgram { program, directives }),
        (SourceFile::Unit(unit), ..) => Err(Diagnostic::new(unit.name.span, "expected a program, found a unit")),
    }
}

/// A source file is either a program or a unit.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceFile {
    Program(Program),
    Unit(Unit),
}

/// Lex and parse a program or unit. `base` is added to every span, so that
/// spans from different files don't overlap (see [`crate::diag::SourceMap`]).
/// Node ids start at `base` too, which keeps them unique across files.
pub fn parse_file(source: &[u8], base: usize) -> PResult<(SourceFile, Vec<Directive>, Vec<lexer::Comment>)> {
    let shift = |span: Span| Span::new(span.start + base, span.end + base);
    let (tokens, comments) =
        lexer::tokenize_with_comments(source).map_err(|d| Diagnostic::new(shift(d.span), d.message))?;
    let tokens = tokens.into_iter().map(|t| Token { span: shift(t.span), ..t }).collect();
    let comments = comments.into_iter().map(|c| lexer::Comment { span: shift(c.span), ..c }).collect();
    let mut parser = Parser::new(tokens);
    parser.next_id = base as NodeId;
    let file = if parser.at_keyword(Keyword::Unit) {
        SourceFile::Unit(parser.unit()?)
    } else {
        SourceFile::Program(parser.program()?)
    };
    Ok((file, parser.directives, comments))
}

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    next_id: NodeId,
    directives: Vec<Directive>,
}

impl Parser {
    fn new(all_tokens: Vec<Token>) -> Parser {
        // Directives can appear anywhere, even in the middle of an
        // expression, so pull them out of the token stream up front and keep
        // them on the side. Sema can still tell where each one was by span.
        let mut tokens = Vec::new();
        let mut directives = Vec::new();
        for token in all_tokens {
            match token.kind {
                TokenKind::Directive(text) => directives.push(Directive { text, span: token.span }),
                _ => tokens.push(token),
            }
        }
        Parser { tokens, pos: 0, next_id: 0, directives }
    }

    // ----- Token helpers -------------------------------------------------

    fn peek(&self) -> &TokenKind {
        self.peek_at(0)
    }

    fn peek_at(&self, offset: usize) -> &TokenKind {
        // The token list always ends with `Eof`, so clamping to the last
        // index means "peeking past the end" just keeps seeing `Eof`.
        let index = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[index].kind
    }

    fn span(&self) -> Span {
        self.tokens[self.pos.min(self.tokens.len() - 1)].span
    }

    /// The span of the most recently consumed token.
    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn at_keyword(&self, kw: Keyword) -> bool {
        self.peek() == &TokenKind::Keyword(kw)
    }

    /// Consume the token if it matches; report whether it did.
    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_keyword(&mut self, kw: Keyword) -> bool {
        self.eat(&TokenKind::Keyword(kw))
    }

    fn expected(&self, what: &str) -> Diagnostic {
        Diagnostic::new(self.span(), format!("expected {what}, found {}", self.peek().describe()))
    }

    fn expect(&mut self, kind: &TokenKind) -> PResult<Span> {
        if self.at(kind) { Ok(self.bump().span) } else { Err(self.expected(&kind.describe())) }
    }

    fn expect_keyword(&mut self, kw: Keyword) -> PResult<Span> {
        self.expect(&TokenKind::Keyword(kw))
    }

    fn expect_ident(&mut self) -> PResult<Ident> {
        // `name` borrows from `self.tokens`, so clone it before calling
        // `bump`, which needs `&mut self`. The borrow checker won't let us
        // hold a shared borrow across a mutable one.
        if let TokenKind::Ident(name) = self.peek() {
            let name = name.clone();
            let span = self.bump().span;
            Ok(Ident { name, span })
        } else {
            Err(self.expected("an identifier"))
        }
    }

    fn ident_list(&mut self) -> PResult<Vec<Ident>> {
        let mut names = vec![self.expect_ident()?];
        while self.eat(&TokenKind::Comma) {
            names.push(self.expect_ident()?);
        }
        Ok(names)
    }

    fn new_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn mk_expr(&mut self, kind: ExprKind, span: Span) -> Expr {
        Expr { id: self.new_id(), kind, span }
    }

    // ----- Programs and blocks -------------------------------------------

    fn program(&mut self) -> PResult<Program> {
        // The `program Name;` header is optional in Turbo Pascal.
        let name = if self.eat_keyword(Keyword::Program) {
            let name = self.expect_ident()?;
            // Standard Pascal lists files here: `program Copy(input, output);`
            if self.eat(&TokenKind::LParen) {
                self.ident_list()?;
                self.expect(&TokenKind::RParen)?;
            }
            self.expect(&TokenKind::Semi)?;
            name
        } else {
            Ident { name: "Main".to_string(), span: Span::default() }
        };
        let uses = self.uses_clause()?;
        let block = self.block()?;
        self.expect(&TokenKind::Dot)?;
        Ok(Program { name, uses, block })
    }

    fn unit(&mut self) -> PResult<Unit> {
        let start = self.expect_keyword(Keyword::Unit)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Semi)?;
        let interface_keyword = self.expect_keyword(Keyword::Interface)?;
        let interface_uses = self.uses_clause()?;
        let interface = self.declarations(true)?;
        self.expect_keyword(Keyword::Implementation)?;
        let implementation_uses = self.uses_clause()?;
        let implementation = self.declarations(false)?;
        let init = if self.eat_keyword(Keyword::Begin) { self.statement_list()? } else { Vec::new() };
        let end = self.expect_keyword(Keyword::End)?;
        self.expect(&TokenKind::Dot)?;
        Ok(Unit {
            name,
            interface_keyword,
            interface_uses,
            interface,
            implementation_uses,
            implementation,
            init,
            end,
            span: start.to(self.prev_span()),
        })
    }

    fn uses_clause(&mut self) -> PResult<Vec<Ident>> {
        if !self.eat_keyword(Keyword::Uses) {
            return Ok(Vec::new());
        }
        let names = self.ident_list()?;
        self.expect(&TokenKind::Semi)?;
        Ok(names)
    }

    fn block(&mut self) -> PResult<Block> {
        let decls = self.declarations(false)?;
        let start = self.expect_keyword(Keyword::Begin)?;
        let body = self.statement_list()?;
        let end = self.expect_keyword(Keyword::End)?;
        Ok(Block { decls, body, body_span: start.to(end) })
    }

    // ----- Declarations --------------------------------------------------

    /// Declaration sections, in any order. In a unit's interface
    /// (`headings_only`), procedures are just headings.
    fn declarations(&mut self, headings_only: bool) -> PResult<Vec<Decl>> {
        let mut decls = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Keyword(Keyword::Const) => self.const_section(&mut decls)?,
                TokenKind::Keyword(Keyword::Type) => self.type_section(&mut decls)?,
                TokenKind::Keyword(Keyword::Var) => self.var_section(&mut decls)?,
                TokenKind::Keyword(
                    Keyword::Procedure | Keyword::Function | Keyword::Constructor | Keyword::Destructor,
                ) => {
                    decls.push(Decl::Proc(self.proc_decl(headings_only)?));
                }
                TokenKind::Keyword(Keyword::Label) => {
                    return Err(Diagnostic::new(self.span(), "labels and `goto` are not supported"));
                }
                _ => return Ok(decls),
            }
        }
    }

    fn const_section(&mut self, decls: &mut Vec<Decl>) -> PResult<()> {
        self.expect_keyword(Keyword::Const)?;
        // Keep going while the next token is an identifier: a const section
        // ends when another section (or `begin`) starts.
        while matches!(self.peek(), TokenKind::Ident(_)) {
            let name = self.expect_ident()?;
            let ty = if self.eat(&TokenKind::Colon) { Some(self.type_expr()?) } else { None };
            self.expect(&TokenKind::Eq)?;
            let value = if ty.is_some() { self.typed_const_value()? } else { ConstValue::Expr(self.expression()?) };
            self.expect(&TokenKind::Semi)?;
            decls.push(Decl::Const(ConstDecl { name, ty, value }));
        }
        Ok(())
    }

    /// The initializer of a typed constant. `(1, 2, 3)` is an array,
    /// `(x: 1; y: 2)` is a record, anything else is an expression. A
    /// parenthesized expression like `(3 + 4) * 2` also starts with `(`, so
    /// for lists we parse optimistically and back up if it doesn't pan out.
    fn typed_const_value(&mut self) -> PResult<ConstValue> {
        if !self.at(&TokenKind::LParen) {
            return Ok(ConstValue::Expr(self.expression()?));
        }
        let start = self.span();
        // `(name: value; ...)` is a record initializer.
        if matches!(self.peek_at(1), TokenKind::Ident(_)) && self.peek_at(2) == &TokenKind::Colon {
            self.bump();
            let mut fields = Vec::new();
            while !self.at(&TokenKind::RParen) {
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                fields.push((name, self.typed_const_value()?));
                if !self.eat(&TokenKind::Semi) {
                    break;
                }
            }
            let end = self.expect(&TokenKind::RParen)?;
            return Ok(ConstValue::Record(fields, start.to(end)));
        }
        let saved_pos = self.pos;
        let saved_id = self.next_id;
        if let Ok(list) = self.const_list(start) {
            let ConstValue::List(items, _) = &list else { unreachable!() };
            // A one-element list followed by an operator was really a
            // parenthesized expression, e.g. `(1 + 2) * 3`.
            let ends_value = matches!(self.peek(), TokenKind::Semi | TokenKind::Comma | TokenKind::RParen);
            if ends_value && (items.len() > 1 || matches!(items[0], ConstValue::List(..) | ConstValue::Record(..))) {
                return Ok(list);
            }
        }
        self.pos = saved_pos;
        self.next_id = saved_id;
        Ok(ConstValue::Expr(self.expression()?))
    }

    fn const_list(&mut self, start: Span) -> PResult<ConstValue> {
        self.expect(&TokenKind::LParen)?;
        let mut items = vec![self.typed_const_value()?];
        while self.eat(&TokenKind::Comma) {
            items.push(self.typed_const_value()?);
        }
        let end = self.expect(&TokenKind::RParen)?;
        Ok(ConstValue::List(items, start.to(end)))
    }

    fn type_section(&mut self, decls: &mut Vec<Decl>) -> PResult<()> {
        self.expect_keyword(Keyword::Type)?;
        while matches!(self.peek(), TokenKind::Ident(_)) {
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Eq)?;
            let ty = self.type_expr()?;
            self.expect(&TokenKind::Semi)?;
            decls.push(Decl::Type(TypeDecl { name, ty }));
        }
        Ok(())
    }

    fn var_section(&mut self, decls: &mut Vec<Decl>) -> PResult<()> {
        self.expect_keyword(Keyword::Var)?;
        while matches!(self.peek(), TokenKind::Ident(_)) {
            let names = self.ident_list()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.type_expr()?;
            if self.peek().is_ident("absolute") {
                return Err(Diagnostic::new(self.span(), "`absolute` variables are not supported"));
            }
            self.expect(&TokenKind::Semi)?;
            decls.push(Decl::Var(VarDecl { names, ty }));
        }
        Ok(())
    }

    /// A procedure or function. With `heading_only`, stop after the heading
    /// (as in a unit interface or an `object` type).
    pub(super) fn proc_decl(&mut self, heading_only: bool) -> PResult<ProcDecl> {
        let start = self.span();
        // In Turbo Pascal a constructor also sets up the object's table of
        // virtual methods. PascRust's objects carry no such table (see
        // `notes/virtual_methods.md`), so constructors and destructors are
        // ordinary procedures.
        let role = match self.peek() {
            TokenKind::Keyword(Keyword::Constructor) => MethodRole::Constructor,
            TokenKind::Keyword(Keyword::Destructor) => MethodRole::Destructor,
            _ => MethodRole::Plain,
        };
        let kind = match self.bump().kind {
            TokenKind::Keyword(Keyword::Function) => ProcKind::Function,
            TokenKind::Keyword(Keyword::Procedure | Keyword::Constructor | Keyword::Destructor) => ProcKind::Procedure,
            _ => return Err(Diagnostic::new(start, "expected `procedure` or `function`")),
        };
        let mut name = self.expect_ident()?;
        // `procedure Queue.AddQ`: the body of a method of object type `Queue`.
        let mut owner = None;
        if self.eat(&TokenKind::Dot) {
            owner = Some(name);
            name = self.expect_ident()?;
        }
        let params = if self.at(&TokenKind::LParen) { self.param_list()? } else { Vec::new() };
        // The result type may be left out when repeating the header of a
        // function that was declared `forward`.
        let ret =
            if kind == ProcKind::Function && self.eat(&TokenKind::Colon) { Some(self.type_expr()?) } else { None };
        let header_span = start.to(self.expect(&TokenKind::Semi)?);

        // Directives after the header. `far` and `near` were about 16-bit
        // x86 memory models and mean nothing to us, so they're skipped.
        let mut body = None;
        let mut is_virtual = false;
        loop {
            if self.peek().is_ident("forward") {
                self.bump();
                body = Some(ProcBody::Forward);
            } else if self.peek().is_ident("external") {
                self.bump();
                body = Some(ProcBody::External);
            } else if self.peek().is_ident("far") || self.peek().is_ident("near") {
                self.bump();
            } else if self.peek().is_ident("virtual") {
                self.bump();
                is_virtual = true;
                // TP 5.5 allows a dynamic method index after `virtual`
                // (`virtual 100;`), for Windows message handlers.
                if let TokenKind::Integer(_) = self.peek() {
                    self.bump();
                }
            } else {
                break;
            }
            self.expect(&TokenKind::Semi)?;
        }
        let body = match body {
            Some(body) => body,
            None if heading_only => ProcBody::Heading,
            None => {
                let block = self.block()?;
                self.expect(&TokenKind::Semi)?;
                ProcBody::Block(Box::new(block))
            }
        };
        Ok(ProcDecl {
            kind,
            owner,
            name,
            params,
            ret,
            body,
            span: start.to(self.prev_span()),
            is_virtual,
            role,
            header_span,
        })
    }

    fn param_list(&mut self) -> PResult<Vec<ParamGroup>> {
        self.expect(&TokenKind::LParen)?;
        let mut groups = Vec::new();
        loop {
            let mode = if self.eat_keyword(Keyword::Var) {
                ParamMode::Var
            } else if self.eat_keyword(Keyword::Const) {
                ParamMode::Const
            } else {
                ParamMode::Value
            };
            let names = self.ident_list()?;
            let ty = if self.eat(&TokenKind::Colon) { Some(self.type_expr()?) } else { None };
            groups.push(ParamGroup { mode, names, ty });
            if !self.eat(&TokenKind::Semi) {
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;
        Ok(groups)
    }
}

#[cfg(test)]
mod tests;
