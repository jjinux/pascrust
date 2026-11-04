//! A Rust program that uses `tests/embed/counter.pas`, a unit with
//! variables, built with `pascrust build --lib`. The unit's variables, and
//! those of `helper.pas` that it uses, are the fields of `CounterContext`,
//! and its interface routines are its methods.

use counter::pascrust_runtime::{RuntimeError, call};
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
    Ok(())
}
