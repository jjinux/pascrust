//! A Rust program using a Turbo Pascal unit from 1997: `BSTUnit`, a binary
//! search tree of people's IDs (examples/database/bstunit.pas). `build.rs`
//! translates it; `include_library!` brings it in as the module `bstunit`.

ruscal_runtime::include_library!(bstunit);

use bstunit::{Bst, BstUnitContext, PeopleKey};
use ruscal_runtime::{RuntimeError, call};

fn main() -> Result<(), RuntimeError> {
    // Every library starts with its context, which runs the units'
    // initialization parts. Pascal code goes through `call`, which turns a
    // Pascal runtime error into an `Err`.
    let _unit = call(BstUnitContext::new)?;

    // A Pascal object is a Rust struct, and its methods are methods.
    let mut tree = Bst::default();
    call(|| tree.create())?;
    for (id, location) in [(1997, 1), (1984, 2), (2026, 3), (1990, 4)] {
        let mut person = PeopleKey { id, location, ..Default::default() };
        let mut ok = false;
        call(|| tree.insert(&mut person, &mut ok))?;
        println!("insert {id}: {ok}");
    }

    let mut wanted = PeopleKey { id: 1990, ..Default::default() };
    let mut found = false;
    call(|| tree.search(&mut wanted, &mut found))?;
    println!("search 1990: {found}, at record {}", wanted.location);

    let mut first = PeopleKey::default();
    call(|| tree.first_item(&mut first))?;
    println!("smallest: {}", first.id);

    let mut ok = false;
    call(|| tree.delete(PeopleKey { id: 1984, ..Default::default() }, &mut ok))?;
    call(|| tree.first_item(&mut first))?;
    println!("after deleting 1984, smallest: {}", first.id);
    Ok(())
}
