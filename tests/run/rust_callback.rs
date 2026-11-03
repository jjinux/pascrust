//! The Rust half of rust_callback.pas: a sort that calls back into Pascal.

use crate::{TLess, TList};

/// An insertion sort that asks Pascal which of two items comes first.
pub fn sort_with(items: &mut TList, less: TLess) {
    let less = less.get();
    for i in 2..=6 {
        let mut j = i;
        while j > 1 && less(items[j], items[j - 1]) {
            let t = items[j];
            items[j] = items[j - 1];
            items[j - 1] = t;
            j -= 1;
        }
    }
}
