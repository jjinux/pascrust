//! Code generation: walk the AST, consult sema's [`Analysis`], and write
//! Rust source text.
//!
//! The output only needs to be valid Rust; the driver runs it through
//! `rustfmt` afterwards, so this module doesn't fuss over line lengths. It
//! does care about *readability*, though: it avoids redundant parentheses,
//! casts and `mut`s, because the generated code is meant to be read.
//!
//! Split by area:
//! - `mod.rs` (this file): the file layout, functions and statements
//! - `expr.rs`: expressions, type conversions and literals
//! - `builtins.rs`: calls to Turbo Pascal's built-in procedures

mod builtins;
mod expr;

use std::rc::Rc;

use crate::ast::*;
use crate::diag::{SourceMap, Span};
use crate::sema::types::ObjectType;
use crate::sema::{Access, Analysis, Builtin, CaptureMode, MAIN, ProcId, SymId, SymKind, Type, VarRole};

/// Generate the Rust source for a program and its units: one file per
/// Pascal source file, `main.rs` first.
/// `io_unchecked` lists the parts of the source compiled with `{$I-}`.
pub fn generate(
    program: &Program,
    units: &[Unit],
    analysis: &Analysis,
    map: &SourceMap,
    io_unchecked: &[Span],
) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut g = Codegen {
        a: analysis,
        map,
        out: String::new(),
        indent: 0,
        proc: MAIN,
        explained: Default::default(),
        file: 0,
        vis: "",
        mutable_place: false,
        pending: Vec::new(),
        loops: Vec::new(),
        io_unchecked,
        self_expr: None,
    };
    g.program(program);
    files.push(("main.rs".to_string(), std::mem::take(&mut g.out)));
    for (unit, info) in units.iter().zip(&analysis.units) {
        g.unit(unit);
        files.push((format!("{}.rs", info.module), std::mem::take(&mut g.out)));
    }
    files
}

struct Codegen<'a> {
    a: &'a Analysis,
    map: &'a SourceMap,
    out: String,
    indent: usize,
    /// The Pascal procedure whose Rust function is being generated. It
    /// decides how variables are reached (`x` or `*x`, see [`Access`]).
    proc: ProcId,
    /// Teaching comments already written. Each explanation appears in full
    /// only the first time, so the output doesn't drown in them.
    explained: std::collections::HashSet<&'static str>,
    /// The source file (index in the source map) being translated.
    file: usize,
    /// Visibility for items: `pub ` in a unit's module, nothing in `main.rs`.
    vis: &'static str,
    /// True while generating a place that will be written, so that `p^`
    /// borrows the record mutably (`p.get_mut()`).
    mutable_place: bool,
    /// `let` statements to emit before the next line (see
    /// `expr::call_args`).
    pending: Vec<String>,
    /// The loops we're inside, innermost last: the labels `Break` and
    /// `Continue` must use for a `repeat` that has a `Continue` in it (see
    /// `repeat_loop`), or `None` for a loop where plain `break` and
    /// `continue` do the right thing.
    loops: Vec<Option<LoopLabels>>,
    /// The parts of the source compiled with `{$I-}`, where I/O errors go
    /// to `IOResult` instead of stopping the program.
    io_unchecked: &'a [Span],
    /// While writing a trait's default method (see `object_traits`): how
    /// the body reaches its own object's fields, `self.t_shape()`, since
    /// `self` there is any object descended from it.
    self_expr: Option<String>,
}

/// Labels for a `repeat ... until` loop whose body uses `Continue`.
struct LoopLabels {
    /// `Break` jumps out of the loop: `break 'repeat_loop;`. (Only if the
    /// body uses `Break`: Rust warns about unused labels.)
    loop_label: Option<String>,
    /// `Continue` jumps to the `until` test: `break 'repeat_body;`.
    body_label: String,
}

impl<'a> Codegen<'a> {
    /// True the first time it's called for `topic`.
    fn first_time(&mut self, topic: &'static str) -> bool {
        self.explained.insert(topic)
    }

    /// Write one line at the current indentation.
    fn line(&mut self, text: impl AsRef<str>) {
        // Temporaries that the code on this line needs come first.
        for pending in std::mem::take(&mut self.pending) {
            for _ in 0..self.indent {
                self.out.push_str("    ");
            }
            self.out.push_str(&pending);
            self.out.push('\n');
        }
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(text.as_ref());
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    /// Source text for a span, with runs of whitespace collapsed, for doc
    /// comments that quote the Pascal.
    fn pascal_text(&self, span: crate::diag::Span) -> String {
        let text = String::from_utf8_lossy(self.map.slice(span));
        text.split_whitespace().collect::<Vec<_>>().join(" ").replace('`', "'")
    }

    fn file_name(&self) -> String {
        self.map.file(self.file).name.clone()
    }

    /// `use` lines shared by every generated file.
    fn imports(&mut self, current_module: Option<&str>) {
        self.blank();
        self.line("use pascrust_runtime::prelude::*;");
        let others: Vec<&str> =
            self.a.units.iter().map(|u| u.module.as_str()).filter(|&m| Some(m) != current_module).collect();
        if !others.is_empty() {
            // Every module can see every unit. Pascal is stricter, but sema
            // already checked visibility, and lifted procedures may need to
            // name types from units their Pascal source never mentions.
            for module in others {
                self.line("#[allow(unused_imports)]");
                self.line(format!("use crate::{module}::*;"));
            }
        }
    }

    fn program(&mut self, program: &Program) {
        self.file = self.map.file_index(program.block.body_span.start);
        let file_name = self.file_name();
        self.line(format!("//! Generated by pascrust from `{file_name}`. Do not edit by hand."));
        self.line(format!("//! Pascal: `program {};`", program.name.name));
        if !program.block.decls.is_empty() || !self.a.units.is_empty() {
            self.blank();
            self.line("// Pascal programs often declare things they never use, or assign a");
            self.line("// variable before reading it; that's fine, so don't warn about it.");
            self.line("#![allow(unused_variables, unused_assignments, dead_code)]");
        }
        if !self.a.units.is_empty() {
            self.blank();
            self.line("// Each Pascal unit becomes a Rust module.");
            for unit in &self.a.units {
                self.line(format!("mod {};", unit.module));
            }
        }
        self.imports(None);

        self.consts(&program.block.decls);
        self.type_defs();
        self.procs(&program.block.decls);
        self.impls(&program.block.decls);

        self.blank();
        self.line(format!("/// The main program: the `begin ... end.` block of `{}`.", program.name.name));
        self.line("/// Its local variables are the Pascal program's global variables.");
        if !self.a.units.is_empty() {
            self.line("/// (Including the global variables of every unit.)");
        }
        self.line("fn pascal_main() {");
        self.indent += 1;
        self.proc = MAIN;
        self.locals(MAIN);
        for unit in &self.a.units {
            if let Some(init) = unit.init {
                self.line(format!("// The initialization part of unit `{}`.", unit.name));
                let call = self.user_call(init, &[]);
                self.line(format!("{call};"));
            }
        }
        self.stmts(&program.block.body);
        self.indent -= 1;
        self.line("}");

        self.blank();
        self.line("fn main() {");
        self.line("    // `run` turns Pascal runtime errors into Turbo Pascal style messages and");
        self.line("    // exit codes.");
        self.line("    pascrust_runtime::run(pascal_main);");
        self.line("}");
    }

    /// A unit's Rust module.
    fn unit(&mut self, unit: &Unit) {
        self.file = self.map.file_index(unit.span.start);
        let info = self.a.units.iter().find(|u| u.span == unit.span).expect("sema saw every unit");
        let module = info.module.clone();
        let file_name = self.file_name();
        self.line(format!("//! Generated by pascrust from `{file_name}`. Do not edit by hand."));
        self.line(format!("//! Pascal: `unit {};`", unit.name.name));
        self.imports(Some(&module));
        self.vis = "pub ";
        let decls: Vec<Decl> = unit.interface.iter().chain(&unit.implementation).cloned().collect();
        self.consts(&decls);
        self.type_defs();
        self.procs(&decls);
        self.impls(&decls);
        if let Some(init) = info.init {
            self.proc = init;
            self.blank();
            self.line(format!(
                "/// The initialization part of unit `{}`, run before the main program.",
                unit.name.name
            ));
            let params = self.signature_params(init);
            self.line(format!("pub fn {}({params}) {{", self.a.procs[init].rust_name));
            self.indent += 1;
            self.stmts(&unit.init);
            self.indent -= 1;
            self.line("}");
        }
        self.vis = "";
    }

    /// `const` declarations become Rust `const` items.
    fn consts(&mut self, decls: &[Decl]) {
        // Constants declared inside procedures go here too, at module level:
        // a procedure's nested procedures become separate functions, and
        // they need to see the constants. Their names are already unique.
        fn collect<'d>(decls: &'d [Decl], out: &mut Vec<&'d Decl>) {
            for decl in decls {
                match decl {
                    Decl::Const(_) => out.push(decl),
                    Decl::Proc(ProcDecl { body: ProcBody::Block(body), .. }) => collect(&body.decls, out),
                    _ => {}
                }
            }
        }
        let mut all = Vec::new();
        collect(decls, &mut all);
        let mut first = true;
        for decl in all {
            let Decl::Const(c) = decl else { continue };
            if c.ty.is_some() {
                continue; // typed constants are variables
            }
            let sym = self.a.decls[&c.name.span.start];
            let SymKind::Const { value, ty } = &self.a.sym(sym).kind else {
                continue;
            };
            if first {
                self.blank();
                first = false;
            }
            let rust_ty = match ty {
                Type::StrLit(_) => {
                    if value_is_ascii(value) {
                        "&str".to_string()
                    } else {
                        "&[u8]".to_string()
                    }
                }
                other => other.rust(),
            };
            let value = self.const_value(value, ty);
            let name = &self.a.sym(sym).rust_name;
            self.line(format!("{}const {name}: {rust_ty} = {value};", self.vis));
        }
    }

    /// Rust items for the program's enum, record and array types.
    fn type_defs(&mut self) {
        for (ty, span) in &self.a.type_defs {
            if self.map.file_index(span.start) != self.file {
                continue;
            }
            self.blank();
            // Quote short declarations whole; for records and objects, just
            // the opening.
            let text = self.pascal_text(*span);
            let text = match ty {
                Type::Record(r) if text.len() > 60 => format!("{} = record ... end", r.rust_name),
                Type::Object(o) if text.len() > 60 => match &o.parent {
                    Some(parent) => format!("{} = object({}) ... end", o.rust_name, parent.rust_name),
                    None => format!("{} = object ... end", o.rust_name),
                },
                _ => text,
            };
            self.line(format!("/// Pascal: `{text}`"));
            match ty {
                Type::Enum(e) => {
                    self.line("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]");
                    self.line(format!("{}enum {} {{", self.vis, e.rust_name));
                    for (i, variant) in e.variants.iter().enumerate() {
                        if i == 0 {
                            self.line("    #[default]");
                        }
                        self.line(format!("    {variant},"));
                    }
                    self.line("}");
                    self.blank();
                    self.line("// Pascal enumerations are ordinal types: they can index arrays, drive");
                    self.line("// `for` loops and go through `Succ`/`Pred`. These impls connect the enum");
                    self.line("// to the runtime's support for that.");
                    self.line(format!("impl Ordinal for {} {{", e.rust_name));
                    self.line("    fn ord(self) -> i64 {");
                    self.line("        self as i64");
                    self.line("    }");
                    self.line("}");
                    self.blank();
                    self.line(format!("impl Enumerated for {} {{", e.rust_name));
                    let values: Vec<String> = e.variants.iter().map(|v| format!("{}::{v}", e.rust_name)).collect();
                    self.line(format!("    const VALUES: &'static [Self] = &[{}];", values.join(", ")));
                    self.line("}");
                }
                Type::Record(r) => {
                    let derives = if ty.is_copy() { "Clone, Copy, Debug, Default" } else { "Clone, Debug, Default" };
                    self.line(format!("#[derive({derives})]"));
                    self.line(format!("{}struct {} {{", self.vis, r.rust_name));
                    for field in &r.fields {
                        self.line(format!("    {}{}: {},", self.vis, field.rust_name, field.ty.rust()));
                    }
                    self.line("}");
                }
                Type::Object(o) => {
                    let derives = if ty.is_copy() { "Clone, Copy, Debug, Default" } else { "Clone, Debug, Default" };
                    self.line(format!("#[derive({derives})]"));
                    self.line(format!("{}struct {} {{", self.vis, o.rust_name));
                    if let Some(parent) = &o.parent {
                        if self.first_time("inheritance") {
                            self.line(format!(
                                "    /// The inherited part: `{}` descends from `{}`. Rust has no struct",
                                o.rust_name, parent.rust_name
                            ));
                            self.line(
                                "    /// inheritance, so the parent object is embedded as a field, and inherited",
                            );
                            self.line("    /// fields and methods are reached through it (`self.base.x`).");
                        }
                        self.line(format!("    {}base: {},", self.vis, parent.rust_name));
                    }
                    for field in &o.fields {
                        self.line(format!("    {}{}: {},", self.vis, field.rust_name, field.ty.rust()));
                    }
                    self.line("}");
                }
                Type::Pointer(p) => {
                    let alias = p.alias.as_deref().expect("only named pointer types are emitted");
                    if self.first_time("pointer") {
                        self.line("///");
                        self.line("/// A Pascal pointer becomes a `Ptr`, the runtime's safe handle to a shared");
                        self.line("/// heap record (an `Rc<RefCell<...>>` inside): using `nil` or a disposed");
                        self.line("/// record is a runtime error instead of undefined behaviour.");
                    }
                    self.line(format!("{}type {alias} = Ptr<{}>;", self.vis, p.target().rust()));
                }
                Type::Array(a) => {
                    let alias = a.alias.as_deref().expect("only named array types are emitted");
                    self.line(format!("{}type {alias} = {};", self.vis, a.rust_unaliased()));
                }
                _ => {}
            }
            if self.stored_in_files().contains(&ty.rust()) {
                self.file_record(ty);
            }
        }
    }

    /// The Rust names of the record, object and enum types that some
    /// `file of T` stores, directly or as part of `T`.
    fn stored_in_files(&self) -> std::collections::HashSet<String> {
        fn visit(ty: &Type, found: &mut std::collections::HashSet<String>) {
            match ty {
                Type::Record(r) => {
                    if found.insert(r.rust_name.clone()) {
                        r.fields.iter().for_each(|f| visit(&f.ty, found));
                    }
                }
                Type::Object(o) => {
                    if found.insert(o.rust_name.clone()) {
                        o.fields.iter().for_each(|f| visit(&f.ty, found));
                        if let Some(parent) = &o.parent {
                            visit(&Type::Object(parent.clone()), found);
                        }
                    }
                }
                Type::Enum(e) => {
                    found.insert(e.rust_name.clone());
                }
                Type::Array(a) => visit(&a.elem, found),
                _ => {}
            }
        }
        let mut found = std::collections::HashSet::new();
        for ty in &self.a.file_types {
            visit(ty, &mut found);
        }
        found
    }

    /// `file_record!(...)`: let a `file of T` store this type.
    fn file_record(&mut self, ty: &Type) {
        self.blank();
        if self.first_time("file_record") {
            self.line("// `file of` needs to turn records into bytes and back. This runtime macro");
            self.line("// implements that (the `FileRecord` trait) field by field.");
        }
        match ty {
            Type::Enum(e) => self.line(format!("file_record!(enum {});", e.rust_name)),
            Type::Record(r) => {
                let fields: Vec<&str> = r.fields.iter().map(|f| f.rust_name.as_str()).collect();
                self.line(format!("file_record!({} {{ {} }});", r.rust_name, fields.join(", ")));
            }
            Type::Object(o) => {
                let mut fields: Vec<&str> = o.parent.iter().map(|_| "base").collect();
                fields.extend(o.fields.iter().map(|f| f.rust_name.as_str()));
                self.line(format!("file_record!({} {{ {} }});", o.rust_name, fields.join(", ")));
            }
            _ => {}
        }
    }

    /// Every procedure in `decls`, then (recursively) the procedures nested
    /// inside each one. Rust has no nested functions with access to their
    /// parent's variables, so they all become top-level functions.
    fn procs(&mut self, decls: &[Decl]) {
        for decl in decls {
            if let Decl::Proc(p) = decl
                && let ProcBody::Block(body) = &p.body
                && p.owner.is_none()
            {
                self.proc_fn(p, body);
                self.procs(&body.decls);
            }
        }
    }

    /// An `impl` block for each object type, holding its methods. Procedures
    /// nested inside a method follow the block as free functions.
    fn impls(&mut self, decls: &[Decl]) {
        let methods: Vec<&ProcDecl> = decls
            .iter()
            .filter_map(|d| match d {
                Decl::Proc(p) if p.owner.is_some() => Some(p),
                _ => None,
            })
            .collect();
        let mut owners: Vec<String> = Vec::new();
        for m in &methods {
            let key = m.owner.as_ref().unwrap().key();
            if !owners.contains(&key) {
                owners.push(key);
            }
        }
        let mut with_traits = Vec::new();
        for owner in owners {
            let group: Vec<&&ProcDecl> = methods.iter().filter(|m| m.owner.as_ref().unwrap().key() == owner).collect();
            let pid = self.a.proc_decls[&group[0].name.span.start];
            let object = self.a.procs[pid].self_param.as_ref().unwrap().0.clone();
            // Methods that must see the object's real type go in its trait.
            let (dispatched, inherent): (Vec<&ProcDecl>, Vec<&ProcDecl>) = group
                .iter()
                .map(|m| **m)
                .partition(|m| self.a.dispatched.contains(&self.a.proc_decls[&m.name.span.start]));
            if !inherent.is_empty() {
                self.blank();
                if self.first_time("impl") {
                    self.line("// A Pascal object's methods go in an `impl` block. Each takes `&mut self`,");
                    self.line("// which is Pascal's implicit `Self` parameter.");
                }
                self.line(format!("impl {} {{", object.rust_name));
                self.indent += 1;
                for m in &inherent {
                    if let ProcBody::Block(body) = &m.body {
                        self.proc_fn(m, body);
                    }
                }
                self.indent -= 1;
                self.line("}");
            }
            if self.in_dispatch(&object) {
                self.object_traits(&object, &dispatched);
                with_traits.push(object.rust_name.clone());
            }
            for m in &group {
                if let ProcBody::Block(body) = &m.body {
                    self.procs(&body.decls);
                }
            }
        }
        // Objects in a hierarchy with virtual methods that have no method
        // bodies of their own still need their trait and impls.
        let others: Vec<_> = self
            .a
            .type_defs
            .iter()
            .filter(|(_, span)| self.map.file_index(span.start) == self.file)
            .filter_map(|(ty, _)| match ty {
                Type::Object(o) if self.in_dispatch(o) && !with_traits.contains(&o.rust_name) => Some(o.clone()),
                _ => None,
            })
            .collect();
        for object in others {
            self.object_traits(&object, &[]);
        }
    }

    /// Is `object` part of a hierarchy with virtual methods?
    pub(super) fn in_dispatch(&self, object: &Rc<ObjectType>) -> bool {
        self.a.dispatch_roots.contains(&object.lineage().last().unwrap().0.rust_name)
    }

    /// The trait for an object in a hierarchy with virtual methods, and its
    /// impls (see `notes/virtual_methods.md`). `bodies` are the object's
    /// methods that must see the real object; they become default methods,
    /// so in them `self` is whatever object descended from this one the
    /// method was called on, and `self.draw()` runs that object's `Draw`.
    fn object_traits(&mut self, object: &Rc<ObjectType>, bodies: &[&ProcDecl]) {
        let name = &object.rust_name;
        let supertrait = match &object.parent {
            Some(parent) => format!(": {}", trait_name(parent)),
            None => String::new(),
        };
        self.blank();
        if self.first_time("virtual") {
            self.line("// Turbo Pascal finds a virtual method's version at run time, from the");
            self.line("// object's real type. Here each object type in the hierarchy has a trait");
            self.line("// like this one: its default methods are written once but run with `self`");
            self.line("// being the real object, so a call to a virtual method inside them goes to");
            self.line("// the right version. Each object implements the traits of its ancestors.");
        }
        self.line(format!("{}trait {}{supertrait} {{", self.vis, trait_name(object)));
        self.indent += 1;
        self.line(format!("/// The `{name}` part of the object, for its fields."));
        self.line(format!("fn {}(&mut self) -> &mut {name};", accessor(object)));
        for slot in self.slots_introduced(object) {
            let (params, ret) = self.method_signature(slot);
            self.line(format!("fn {}({params}){ret};", self.a.procs[slot].rust_name));
        }
        let saved = self.self_expr.replace(format!("self.{}()", accessor(object)));
        for m in bodies {
            if let ProcBody::Block(body) = &m.body {
                self.proc_fn(m, body);
            }
        }
        self.self_expr = saved;
        self.indent -= 1;
        self.line("}");
        // `impl AncestorMethods for Object` for this object and each ancestor.
        for (ancestor, path) in object.lineage() {
            self.blank();
            self.line(format!("impl {} for {name} {{", trait_name(&ancestor)));
            self.indent += 1;
            let part = if path.is_empty() { "self".to_string() } else { format!("&mut self.{}", path.join(".")) };
            self.line(format!("fn {}(&mut self) -> &mut {} {{", accessor(&ancestor), ancestor.rust_name));
            self.line(format!("    {part}"));
            self.line("}");
            for slot in self.slots_introduced(&ancestor) {
                // The nearest version of the method, from this object up.
                let key = self.a.procs[slot].name.rsplit('.').next().unwrap().to_ascii_lowercase();
                let version = object
                    .lineage()
                    .into_iter()
                    .find_map(|(o, _)| self.a.methods.get(&(o.rust_name.clone(), key.clone())).copied())
                    .unwrap_or(slot);
                let (params, ret) = self.method_signature(slot);
                let info = &self.a.procs[slot];
                let mut args: Vec<String> =
                    info.captures.iter().map(|c| self.a.var_name(slot, c.sym).to_string()).collect();
                args.extend(info.params.iter().map(|&p| self.a.sym(p).rust_name.clone()));
                self.line(format!("fn {}({params}){ret} {{", info.rust_name));
                self.line(format!("    self.{}({})", self.body_name(version), args.join(", ")));
                self.line("}");
            }
            self.indent -= 1;
            self.line("}");
        }
    }

    /// The virtual methods `object` declares that no ancestor has: the
    /// trait methods every object in the hierarchy must provide.
    fn slots_introduced(&self, object: &Rc<ObjectType>) -> Vec<ProcId> {
        let mut slots: Vec<ProcId> = self
            .a
            .virtuals
            .iter()
            .copied()
            .filter(|&p| {
                let (owner, _) = self.a.procs[p].self_param.as_ref().unwrap();
                let key = self.a.procs[p].name.rsplit('.').next().unwrap().to_ascii_lowercase();
                owner == object
                    && object.parent.as_ref().is_none_or(|parent| {
                        !parent
                            .lineage()
                            .iter()
                            .any(|(o, _)| self.a.methods.contains_key(&(o.rust_name.clone(), key.clone())))
                    })
            })
            .collect();
        slots.sort();
        slots
    }

    /// The Rust name of a method's body: a virtual method's versions are
    /// named after their object (`t_circle_draw`), since the trait method
    /// `draw` picks one of them.
    pub(super) fn body_name(&self, pid: ProcId) -> String {
        let info = &self.a.procs[pid];
        match &info.self_param {
            Some((object, _)) if self.a.virtuals.contains(&pid) => format!("{}_{}", accessor(object), info.rust_name),
            _ => info.rust_name.clone(),
        }
    }

    /// A method's parameter list (with `&mut self`) and return type.
    fn method_signature(&self, pid: ProcId) -> (String, String) {
        let params = self.signature_params(pid);
        let params = if params.is_empty() { "&mut self".to_string() } else { format!("&mut self, {params}") };
        let ret = match &self.a.procs[pid].ret {
            Some(ty) => format!(" -> {}", ty.rust()),
            None => String::new(),
        };
        (params, ret)
    }

    fn proc_fn(&mut self, decl: &ProcDecl, body: &Block) {
        let pid = self.a.proc_decls[&decl.name.span.start];
        let info = &self.a.procs[pid];
        self.proc = pid;

        self.blank();
        self.line(format!("/// Pascal: `{}`", self.pascal_text(decl.header_span)));
        if let Some(parent) = info.parent.filter(|&p| p != MAIN) {
            self.line("///");
            let parent = &self.a.procs[parent].name;
            if self.first_time("nested") {
                self.line(format!("/// Nested inside `{parent}` in the Pascal source. Rust functions can't see"));
                self.line("/// their caller's local variables, so this one was \"lifted\" to the top level");
                self.line("/// and any outer variables it uses are passed in as extra parameters.");
            } else {
                self.line(format!("/// Nested inside `{parent}`; lifted, with outer variables passed in."));
            }
        } else if !info.captures.is_empty() {
            self.line("///");
            if self.first_time("globals") {
                self.line("/// The global variables this procedure uses are passed in as parameters,");
                self.line("/// because the globals are local variables of `pascal_main`.");
            } else {
                self.line("/// Uses globals, which are passed in as parameters.");
            }
        }

        let mut aliases: Vec<_> = self
            .a
            .capture_aliases
            .iter()
            .filter(|((p, _), param)| *p == pid && self.a.used_aliases.contains(&(pid, **param)))
            .collect();
        aliases.sort();
        for ((_, outer), param) in aliases {
            self.line("///");
            let (outer, param) = (&self.a.sym(*outer).name, &self.a.sym(*param).name);
            self.line(format!("/// Every call passes `{outer}` itself as `{param}`, so where the Pascal code"));
            self.line(format!("/// uses `{outer}` directly, this function uses `{param}`: they are the same"));
            self.line("/// variable, and Rust doesn't allow two references to one variable.");
        }

        let mut params = self.signature_params(pid);
        if info.self_param.is_some() {
            params = if params.is_empty() { "&mut self".to_string() } else { format!("&mut self, {params}") };
        }
        let ret = match &info.ret {
            Some(ty) => format!(" -> {}", ty.rust()),
            None => String::new(),
        };
        // Trait methods take no `pub`.
        let vis = if self.self_expr.is_some() { "" } else { self.vis };
        self.line(format!("{vis}fn {}({params}){ret} {{", self.body_name(pid)));
        self.indent += 1;
        self.locals(pid);
        self.stmts(&body.body);
        if info.result.is_some() {
            self.line("result");
        }
        self.indent -= 1;
        self.line("}");
    }

    /// The parameter list of a procedure's Rust function: captured outer
    /// variables first, then the Pascal parameters.
    fn signature_params(&self, pid: ProcId) -> String {
        let info = &self.a.procs[pid];
        let mut params = Vec::new();
        for cap in &info.captures {
            let name = self.a.var_name(pid, cap.sym);
            let ty = self.a.var(cap.sym).ty.rust();
            params.push(match cap.mode {
                CaptureMode::RefMut => format!("{name}: &mut {ty}"),
                CaptureMode::Ref => format!("{name}: &{ty}"),
                CaptureMode::Value => format!("{name}: {ty}"),
            });
        }
        for &param in &info.params {
            let name = &self.a.sym(param).rust_name;
            let var = self.a.var(param);
            let ty = var.ty.rust();
            params.push(match (&var.role, &var.ty) {
                // A `var` parameter of an object type with virtual methods
                // takes any object descended from it.
                (VarRole::Param(ParamMode::Var), Type::Object(o)) if self.in_dispatch(o) => {
                    format!("{name}: &mut dyn {}", trait_name(o))
                }
                (VarRole::Param(ParamMode::Var), _) => format!("{name}: &mut {ty}"),
                _ if self.a.mutated.contains(&param) => format!("mut {name}: {ty}"),
                _ => format!("{name}: {ty}"),
            });
        }
        params.join(", ")
    }

    /// `let` statements for a procedure's local variables.
    fn locals(&mut self, pid: ProcId) {
        for &sym in &self.a.procs[pid].locals {
            if self.a.elided_vars.contains(&sym) {
                continue;
            }
            let var = self.a.var(sym);
            let name = &self.a.sym(sym).rust_name;
            let mutable = if self.a.mutated.contains(&sym) { "mut " } else { "" };
            let init = match self.a.typed_consts.get(&sym) {
                Some(ConstDecl { value, .. }) => self.const_init(value, &var.ty),
                _ => default_value(&var.ty),
            };
            let line = format!("let {mutable}{name}: {} = {init};", var.ty.rust());
            self.line(line);
        }
    }

    // ----- Statements ---------------------------------------------------------

    fn stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    /// A statement as the body of `if`/`while`/...: always a `{ }` block in
    /// Rust, and a Pascal `begin ... end` doesn't need a second pair.
    fn block_body(&mut self, s: &Stmt) {
        self.indent += 1;
        match &s.kind {
            StmtKind::Compound(body) => self.stmts(body),
            _ => self.stmt(s),
        }
        self.indent -= 1;
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Empty => {}
            StmtKind::Assign { target, value } => {
                let line = self.assignment(target, value);
                self.line(line);
            }
            StmtKind::Call(e) => {
                let (callee, args): (&Expr, &[Expr]) = match &e.kind {
                    ExprKind::Call { callee, args } => (callee, args),
                    _ => (e, &[]),
                };
                let code = self.call(callee, args);
                // A function called as a statement: discard its result.
                let is_function = matches!(self.a.type_of(e.id), t if *t != Type::Void);
                if is_function && !code.ends_with(';') && !code.ends_with('}') {
                    self.line(format!("let _ = {code};"));
                } else if code.ends_with(';') || code.ends_with('}') {
                    self.line(code);
                } else {
                    self.line(format!("{code};"));
                }
            }
            StmtKind::Compound(body) => {
                // A nested `begin ... end` just groups statements; Rust
                // doesn't need a block for that.
                self.stmts(body);
            }
            StmtKind::If { .. } => self.if_chain(s, ""),
            StmtKind::While { cond, body } => {
                let cond = self.condition(cond);
                self.line(format!("while {cond} {{"));
                self.loops.push(None);
                self.block_body(body);
                self.loops.pop();
                self.line("}");
            }
            StmtKind::Repeat { body, cond } => self.repeat_loop(body, cond),
            StmtKind::For { var, start, end, down, body } => {
                self.loops.push(None);
                self.for_loop(s, var, start, end, *down, body);
                self.loops.pop();
            }
            StmtKind::Case { scrutinee, arms, else_body } => self.case(scrutinee, arms, else_body.as_deref()),
            StmtKind::With { .. } => {
                // `self.a` is a plain shared reference, so copying it out lets
                // us borrow the rewritten body while `self` is borrowed mutably.
                let a = self.a;
                if self.first_time("with") {
                    self.line("// Pascal `with`: Rust has no equivalent, so each field is written out in full.");
                }
                self.stmt(&a.with_bodies[&s.span.start]);
            }
        }
    }

    /// `repeat ... until cond` becomes `loop { ...; if cond { break; } }`.
    fn repeat_loop(&mut self, body: &[Stmt], cond: &Expr) {
        self.line("// Pascal `repeat ... until`: the body always runs at least once.");
        // Rust's `continue` would skip the `if cond { break; }` at the end of
        // the loop, but Pascal's `Continue` goes on to the `until` test. So a
        // body that uses `Continue` goes in a labeled block, and `Continue`
        // leaves the block with `break 'repeat_body;`. Inside that block a
        // plain `break` isn't allowed, so `Break` needs a label for the loop.
        let labels = self.uses(body, Builtin::Continue).then(|| {
            let n = self.loops.iter().flatten().count();
            let suffix = if n == 0 { String::new() } else { (n + 1).to_string() };
            LoopLabels {
                loop_label: self.uses(body, Builtin::Break).then(|| format!("'repeat_loop{suffix}")),
                body_label: format!("'repeat_body{suffix}"),
            }
        });
        match &labels {
            Some(l) => {
                if self.first_time("repeat-continue") {
                    self.line("// `Continue` must still test the `until` condition, so it jumps out of");
                    self.line("// the labeled block below rather than back to the top of the loop.");
                }
                match &l.loop_label {
                    Some(label) => self.line(format!("{label}: loop {{")),
                    None => self.line("loop {"),
                }
                self.indent += 1;
                self.line(format!("{}: {{", l.body_label));
            }
            None => self.line("loop {"),
        }
        self.indent += 1;
        let labeled = labels.is_some();
        self.loops.push(labels);
        self.stmts(body);
        self.loops.pop();
        if labeled {
            self.indent -= 1;
            self.line("}");
        }
        let cond = self.condition(cond);
        self.line(format!("if {cond} {{"));
        self.line("    break;");
        self.line("}");
        self.indent -= 1;
        self.line("}");
    }

    /// Does `body` call `Break` or `Continue` (`b`) for the loop around it,
    /// rather than for a loop nested inside?
    fn uses(&self, body: &[Stmt], b: Builtin) -> bool {
        body.iter().any(|s| match &s.kind {
            StmtKind::Call(e) => {
                let callee = match &e.kind {
                    ExprKind::Call { callee, .. } => callee,
                    _ => e,
                };
                self.a
                    .refs
                    .get(&callee.id)
                    .is_some_and(|sym| matches!(self.a.sym(*sym).kind, SymKind::Builtin(found) if found == b))
            }
            // `std::slice::from_ref` views one statement as a one-element
            // slice, without copying it.
            StmtKind::Compound(stmts) => self.uses(stmts, b),
            StmtKind::If { then_branch, else_branch, .. } => {
                self.uses(std::slice::from_ref(then_branch), b)
                    || else_branch.as_ref().is_some_and(|e| self.uses(std::slice::from_ref(e), b))
            }
            StmtKind::Case { arms, else_body, .. } => {
                arms.iter().any(|arm| self.uses(std::slice::from_ref(&arm.body), b))
                    || else_body.as_deref().is_some_and(|body| self.uses(body, b))
            }
            StmtKind::With { .. } => self.uses(std::slice::from_ref(&self.a.with_bodies[&s.span.start]), b),
            // A `Break` or `Continue` inside a nested loop belongs to that loop.
            _ => false,
        })
    }

    /// `case` becomes `match`.
    fn case(&mut self, scrutinee: &Expr, arms: &[CaseArm], else_body: Option<&[Stmt]>) {
        let scrutinee_ty = self.a.type_of(scrutinee.id).concrete();
        let scrutinee_code = self.expr(scrutinee).0;
        self.line(format!("match {scrutinee_code} {{"));
        self.indent += 1;
        // Turbo Pascal doesn't object to a label appearing twice; the first
        // arm wins. Rust warns about the unreachable second one, so drop it.
        let mut seen = std::collections::HashSet::new();
        for arm in arms {
            let mut labels = Vec::new();
            let mut dropped = Vec::new();
            for label in &arm.labels {
                match label {
                    CaseLabel::Value(e) => {
                        let code = self.case_label(e);
                        if seen.insert(self.label_ord(e)) {
                            labels.push(code);
                        } else {
                            dropped.push(code);
                        }
                    }
                    CaseLabel::Range(lo, hi) => {
                        seen.extend(self.label_ord(lo)..=self.label_ord(hi));
                        labels.push(format!("{}..={}", self.case_label(lo), self.case_label(hi)));
                    }
                }
            }
            if !dropped.is_empty() {
                self.line(format!("// {} also appeared in an earlier arm, which wins.", dropped.join(", ")));
            }
            if labels.is_empty() {
                continue;
            }
            self.line(format!("{} => {{", labels.join(" | ")));
            self.block_body(&arm.body);
            self.line("}");
        }
        let domain = match &scrutinee_ty {
            Type::Enum(e) => Some(e.variants.len()),
            Type::Bool => Some(2),
            Type::Char | Type::Int(crate::sema::IntKind::Byte) => Some(256),
            _ => None,
        };
        let exhaustive = domain == Some(seen.len());
        match else_body {
            Some(body) if !exhaustive => {
                self.line("_ => {");
                self.indent += 1;
                self.stmts(body);
                self.indent -= 1;
                self.line("}");
            }
            // Pascal's `case` does nothing when no label matches; Rust's
            // `match` must cover every value, so say so explicitly. (Unless
            // the labels already cover every value, as they can for an enum.)
            None if !exhaustive => self.line("_ => {}"),
            _ => {}
        }
        self.indent -= 1;
        self.line("}");
    }

    /// A case label's ordinal value, for spotting duplicates.
    fn label_ord(&self, e: &Expr) -> i64 {
        match &self.a.case_labels[&e.id] {
            crate::sema::ConstVal::Int(n) => *n,
            crate::sema::ConstVal::Str(s) => s.first().copied().unwrap_or(0) as i64,
            crate::sema::ConstVal::Bool(b) => *b as i64,
            crate::sema::ConstVal::Real(_) => 0,
        }
    }

    /// A loop condition. If it needed temporaries, they go in a block
    /// expression (`{ let n = ...; f(&mut x, n) }`): a loop condition is
    /// evaluated every time round, so they can't be hoisted above the loop.
    fn condition(&mut self, cond: &Expr) -> String {
        let code = self.expr(cond).0;
        if self.pending.is_empty() {
            return code;
        }
        let lets = std::mem::take(&mut self.pending).join(" ");
        format!("{{ {lets} {code} }}")
    }

    /// `if`, with `else if` chains flattened the way a person would write
    /// them.
    fn if_chain(&mut self, s: &Stmt, prefix: &str) {
        let StmtKind::If { cond, then_branch, else_branch } = &s.kind else { unreachable!() };
        // Unlike a loop's condition, an `if` condition is evaluated once, so
        // `line` can put its temporaries on the lines before it. (Except in
        // an `else if`, where there's no line before it to put them on.)
        let cond = if prefix.is_empty() { self.expr(cond).0 } else { self.condition(cond) };
        self.line(format!("{prefix}if {cond} {{"));
        self.block_body(then_branch);
        match else_branch.as_deref() {
            None => self.line("}"),
            Some(e @ Stmt { kind: StmtKind::If { .. }, .. }) => self.if_chain(e, "} else "),
            Some(e) => {
                self.line("} else {");
                self.block_body(e);
                self.line("}");
            }
        }
    }

    fn for_loop(&mut self, s: &Stmt, var: &Ident, start: &Expr, end: &Expr, down: bool, body: &Stmt) {
        let sym = self.a.decls[&var.span.start];
        let ty = self.a.var(sym).ty.clone();
        // With two literal bounds (`for i := 10 to 12`), nothing would tell
        // Rust that a fresh loop variable is 16 bits, and it would pick
        // `i32`; then `i * 3000` wouldn't wrap as it does in TP. A suffix on
        // the first bound (`10_i16..=12`) settles it.
        // (A loop variable that's used outside the loop is declared with its
        // type already.)
        let scoped = self.a.scoped_for_loops.contains(&s.span.start);
        let literal = |e: &Expr| matches!(self.a.type_of(e.id), Type::IntLit(_));
        let suffix = match &ty {
            Type::Int(kind) if scoped && literal(start) && literal(end) => format!("_{}", kind.rust()),
            _ => String::new(),
        };
        let lo_hi = |g: &mut Self| {
            let (start, end) = (g.coerce(start, &ty).0, g.coerce(end, &ty).0);
            // The suffix goes on whichever bound comes first in the range.
            let (start, end) = if down { (start, format!("{end}{suffix}")) } else { (format!("{start}{suffix}"), end) };
            match (&ty, down) {
                // Rust ranges only work for numbers (and `u8` chars); the
                // runtime's `enum_range` steps through enum values instead.
                (Type::Enum(_), false) => format!("enum_range({start}, {end})"),
                (Type::Enum(_), true) => format!("enum_range({end}, {start}).rev()"),
                (_, true) => format!("({end}..={start}).rev()"),
                (_, false) => format!("{start}..={end}"),
            }
        };
        let range = lo_hi(self);
        let name = self.a.sym(sym).rust_name.clone();
        if scoped {
            // Pascal's `for i := 1 to n` evaluates both bounds once and does
            // nothing if `n < 1`, exactly like Rust's inclusive range `1..=n`.
            self.line(format!("for {name} in {range} {{"));
            self.block_body(body);
            self.line("}");
        } else {
            // The loop variable is used outside the loop too (Pascal leaves it
            // holding the last value), so keep the variable and update it on
            // every iteration.
            let place = self.var_place(sym);
            if self.first_time("for-variable") {
                self.line(format!("// `{name}` is also used outside this loop (Pascal leaves it holding the"));
                self.line("// last value), so the loop updates it on every pass instead of declaring");
                self.line("// a fresh loop variable.");
            }
            self.line(format!("for {name}_value in {range} {{"));
            self.line(format!("    {place} = {name}_value;"));
            self.block_body(body);
            self.line("}");
        }
    }

    fn assignment(&mut self, target: &Expr, value: &Expr) -> String {
        let target_ty = self.a.type_of(target.id).clone();
        // `p := p^.next` would borrow `p` (through `p.get()`) while assigning
        // to it, which the borrow checker rejects. Computing the value into
        // a temporary first ends that borrow before the assignment.
        if contains_deref(value) && (contains_deref(target) || deref_roots(value).contains(&root_name(target))) {
            let temp = self.temp_name(&value_temp_base(value));
            let value_code = self.coerce(value, &target_ty).0;
            let place = self.place(target);
            return format!("let {temp} = {value_code};\n{}{place} = {temp};", "    ".repeat(self.indent));
        }
        let place = self.place(target);
        // `x := x + n` reads better as Rust's compound assignment `x += n`.
        if let ExprKind::Binary { op: op @ (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul), lhs, rhs } = &value.kind
            && *self.a.type_of(value.id) == target_ty
            && *self.a.type_of(lhs.id) == target_ty
            && !matches!(target_ty, Type::Str { .. })
            && self.same_as_place(lhs, &place)
        {
            let rust_op = match op {
                BinaryOp::Add => "+=",
                BinaryOp::Sub => "-=",
                _ => "*=",
            };
            return format!("{place} {rust_op} {};", self.coerce(rhs, &target_ty).0);
        }
        let mut value_code = self.coerce(value, &target_ty).0;
        // A `string[n]` variable silently truncates longer values in Turbo
        // Pascal.
        if let Type::Str { cap } = target_ty
            && cap < 255
        {
            let short_literal = matches!(self.a.type_of(value.id), Type::StrLit(n) if *n <= cap as usize)
                || *self.a.type_of(value.id) == Type::Char;
            if !short_literal {
                value_code = format!("{}.truncated({cap})", self.postfix(value, &target_ty));
            }
        }
        format!("{place} = {value_code};")
    }

    /// Is `e` the place `place` (so `place := e + n` can be `place += n`)?
    /// Generating `e` just to compare may make temporaries for its calls;
    /// they're thrown away, as the code that would use them is.
    fn same_as_place(&mut self, e: &Expr, place: &str) -> bool {
        let pending = self.pending.len();
        let same = self.expr(e).0 == place;
        self.pending.truncate(pending);
        same
    }

    /// The Rust place expression for writing variable `sym` in the current
    /// function: `x` for a variable it owns, `*x` for one it holds by
    /// reference.
    fn var_place(&self, sym: SymId) -> String {
        let name = self.a.var_name(self.proc, sym);
        match self.a.access(self.proc, sym) {
            Access::Direct => name.to_string(),
            Access::Reference => format!("*{name}"),
        }
    }

    /// The initial value of a typed constant: `PArray::new([1, 2, 3])`,
    /// `Point { x: 1, y: 2, ..Default::default() }` or a plain expression.
    fn const_init(&mut self, value: &ConstValue, ty: &Type) -> String {
        match (value, ty) {
            (ConstValue::Expr(e), _) => self.coerce(e, ty).0,
            (ConstValue::List(items, _), Type::Array(a)) => {
                let items: Vec<String> = items.iter().map(|item| self.const_init(item, &a.elem)).collect();
                format!("PArray::new([{}])", items.join(", "))
            }
            (ConstValue::Record(fields, _), Type::Record(r)) => {
                let mut parts = Vec::new();
                for (name, value) in fields {
                    let field = r.field(&name.name).expect("sema checked the field names");
                    parts.push(format!("{}: {}", field.rust_name, self.const_init(value, &field.ty)));
                }
                if parts.len() < r.fields.len() {
                    // Struct update syntax: take the remaining fields from
                    // the default value.
                    parts.push("..Default::default()".to_string());
                }
                format!("{} {{ {} }}", r.rust_name, parts.join(", "))
            }
            _ => unreachable!("sema checked the initializer's shape"),
        }
    }

    fn case_label(&self, e: &Expr) -> String {
        let value = &self.a.case_labels[&e.id];
        let ty = self.a.type_of(e.id).concrete();
        self.const_value(value, &ty)
    }
}

/// Does the expression dereference a pointer anywhere?
fn contains_deref(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Deref(_) => true,
        ExprKind::Field(base, _) => contains_deref(base),
        ExprKind::Index(base, indexes) => contains_deref(base) || indexes.iter().any(contains_deref),
        ExprKind::Unary { operand, .. } => contains_deref(operand),
        ExprKind::Binary { lhs, rhs, .. } => contains_deref(lhs) || contains_deref(rhs),
        ExprKind::Call { args, .. } => args.iter().any(contains_deref),
        _ => false,
    }
}

/// The variable names (lowercase) at the root of each pointer dereference.
fn deref_roots(e: &Expr) -> Vec<String> {
    let mut roots = Vec::new();
    fn walk(e: &Expr, roots: &mut Vec<String>) {
        match &e.kind {
            ExprKind::Deref(base) => {
                roots.push(root_name(base));
                walk(base, roots);
            }
            ExprKind::Field(base, _) => walk(base, roots),
            ExprKind::Index(base, indexes) => {
                walk(base, roots);
                indexes.iter().for_each(|i| walk(i, roots));
            }
            ExprKind::Unary { operand, .. } => walk(operand, roots),
            ExprKind::Binary { lhs, rhs, .. } => {
                walk(lhs, roots);
                walk(rhs, roots);
            }
            ExprKind::Call { args, .. } => args.iter().for_each(|a| walk(a, roots)),
            _ => {}
        }
    }
    walk(e, &mut roots);
    roots
}

/// The name of the variable a place expression starts from.
fn root_name(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(id) => id.key(),
        ExprKind::Field(base, _) | ExprKind::Index(base, _) | ExprKind::Deref(base) => root_name(base),
        _ => String::new(),
    }
}

/// A name for a hoisted temporary: the last field name (`next` for
/// `p^.next`), or `value`.
fn value_temp_base(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Field(_, field) => crate::sema::names::snake_case(&field.name),
        _ => "value".to_string(),
    }
}

/// The value a Pascal variable of type `ty` starts with. Turbo Pascal
/// leaves local variables uninitialized (whatever was on the stack); Rust
/// insists on initialization, and zero is the safe choice.
fn default_value(ty: &Type) -> String {
    match ty {
        Type::Int(_) | Type::IntLit(_) | Type::Char => "0".to_string(),
        Type::Real => "0.0".to_string(),
        Type::Bool => "false".to_string(),
        Type::Str { .. } | Type::StrLit(_) => "PString::new()".to_string(),
        // An enum starts at its first value, which is what Pascal's
        // zero-filled memory would hold too.
        Type::Enum(e) => format!("{}::{}", e.rust_name, e.variants[0]),
        Type::Array(a) if a.alias.is_none() => "PArray::default()".to_string(),
        Type::Array(_) | Type::Record(_) | Type::Object(_) => format!("{}::default()", ty.rust()),
        Type::Pointer(_) => "Ptr::nil()".to_string(),
        Type::Text => "PText::default()".to_string(),
        Type::File(_) => "PFile::default()".to_string(),
        Type::Nil | Type::Void | Type::Error => "()".to_string(),
    }
}

fn value_is_ascii(value: &crate::sema::ConstVal) -> bool {
    match value {
        crate::sema::ConstVal::Str(s) => s.iter().all(|&b| b.is_ascii()),
        _ => true,
    }
}

/// The trait of an object in a hierarchy with virtual methods: `TShapeMethods`.
pub(super) fn trait_name(object: &ObjectType) -> String {
    format!("{}Methods", object.rust_name)
}

/// The trait method that reaches an object's part of `self`: `t_shape`.
pub(super) fn accessor(object: &ObjectType) -> String {
    crate::sema::names::snake_case(&object.rust_name)
}
