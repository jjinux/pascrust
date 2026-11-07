//! A Rust program that uses `tests/embed/counter.pas`, a unit with
//! variables, built with `ruscal build --lib`. The unit's variables, and
//! those of `helper.pas` that it uses, are the fields of `CounterContext`,
//! and its interface routines are its methods.

use counter::ruscal_runtime::{RuntimeError, call};
use counter::{CounterContext, TTally};

fn main() -> Result<(), RuntimeError> {
    // `new` runs the units' initialization parts, so it goes through `call`.
    let mut c = call(CounterContext::new)?;
    println!("after init: helper_count {}, internal {}", c.helper_count, c.internal);

    call(|| c.bump(5))?;
    println!("bump 5: current {}, helper_count {}", c.current(), c.helper_count);

    let (mut a, mut b) = (1, 2);
    call(|| c.swap(&mut a, &mut b))?;
    println!("swap: {a} {b}, log {}", c.log);

    // Pascal's `New` is `new_2`: `new` makes the struct.
    call(|| c.new_2(40))?;
    println!("new: {}", c.current());
    println!("pure: {}", c.pure(21));

    // `Hook` holds `Bump`, set by the unit's initialization part.
    call(|| c.call_hook(2))?;
    println!("hook: {}", c.current());

    // Helper's own `Count` is `count`; Counter's is `count_2`.
    call(|| c.tick_twice())?;
    println!("tick twice: helper_count {}, helper's count {}", c.helper_count, c.count);

    // An object's method that uses a unit variable takes it as a parameter.
    let mut tally = TTally::default();
    call(|| tally.add(&mut c.count_2, 3))?;
    println!("tally: {}, count {}", tally.total, c.count_2);
    // A routine that changes its own value parameter.
    call(|| c.add_times(10, 3))?;
    println!("add times: {}", c.current());
    // Called directly, without `call`: the Pascal handles its own I/O
    // error (`{$I-}`), so nothing is printed.
    println!("probe: {}", c.probe());

    // Turbo Pascal's arithmetic wraps around (overflow checks are off for
    // the library in this program's Cargo.toml).
    println!("next after 32767: {}", c.next(32767));

    // An `external` routine, written in Rust (counter_rust.rs).
    println!("tally twice: {}", c.tally_twice(tally));
    Ok(())
}
