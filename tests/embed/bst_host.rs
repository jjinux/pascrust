//! A Rust program that uses BSTUnit, the binary search tree in Turbo Pascal
//! from `examples/database/bstunit.pas`, built as a library with
//! `ruscal build --lib examples/database/bstunit.pas`.

use bstunit::ruscal_runtime::{RuntimeError, call};
use bstunit::{Bst, BstUnitContext, PeopleKey};

fn main() -> Result<(), RuntimeError> {
    // Every library starts with its context, which runs the units'
    // initialization parts (BSTUnit has neither variables nor any).
    let _unit = call(BstUnitContext::new)?;

    // A Pascal object is a Rust struct; `Create` is a method like any other.
    let mut tree = Bst::default();
    call(|| tree.create())?;

    for (id, location) in [(50, 1), (30, 2), (70, 3), (60, 4)] {
        let mut item = PeopleKey { id, location, ..Default::default() };
        let mut ok = false;
        // `call` runs Pascal code; a Pascal runtime error comes back as `Err`.
        call(|| tree.insert(&mut item, &mut ok))?;
        println!("insert {id}: {ok}");
    }

    let mut item = PeopleKey { id: 60, ..Default::default() };
    let mut found = false;
    call(|| tree.search(&mut item, &mut found))?;
    println!("search 60: {found}, location {}", item.location);

    let mut first = PeopleKey::default();
    call(|| tree.first_item(&mut first))?;
    println!("first: {}", first.id);

    // Pascal's `Traverse` writes the tree itself (and waits for Enter).
    call(|| tree.traverse())?;

    // A second tree whose root is the record holding 30. Deleting 30 from
    // the first tree disposes that record, so searching the second one
    // uses a disposed record: a Pascal runtime error, which `call` returns
    // as an `Err` instead of stopping the program.
    let mut other = Bst { root: tree.root.get().before.clone() };
    let mut ok = false;
    call(|| tree.delete(PeopleKey { id: 30, ..Default::default() }, &mut ok))?;
    println!("delete 30: {ok}");
    let mut item = PeopleKey { id: 30, ..Default::default() };
    match call(|| other.search(&mut item, &mut found)) {
        Ok(()) => println!("search the other tree: {found}"),
        Err(error) => println!("search the other tree: {error}"),
    }
    Ok(())
}
