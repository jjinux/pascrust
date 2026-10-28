//! Everything generated code needs, in one `use`.
//!
//! A "prelude" is a Rust convention: a module that re-exports the most
//! commonly used items of a crate so that users can glob-import them with
//! `use some_crate::prelude::*;`. The standard library has one too
//! (`std::prelude`), which is why `Vec` and `Option` are always in scope.

pub use crate::array::PArray;
pub use crate::io::{FileRecord, PFile, PText, PasWrite, eof, eoln, seek_eof, seek_eoln, w, wd};
pub use crate::ordinal::{Enumerated, Ordinal, enum_range};
pub use crate::ptr::Ptr;
pub use crate::run::{halt, io_result, io_unchecked, rte};
pub use crate::string::{PString, pos, pstr};
pub use crate::system::{ch_dir, mk_dir, param_count, param_str, pas_str, random, random_real, randomize, rm_dir, val};
// The `Crt`, `Dos` and `Graph` units, when the program uses them.
// `#[cfg(feature)]` compiles a line only when that Cargo feature is on.
#[cfg(any(feature = "crt", feature = "graph"))]
pub use crate::colors::*;
#[cfg(feature = "crt")]
pub use crate::crt::*;
#[cfg(feature = "dos")]
pub use crate::dos::*;
#[cfg(feature = "graph")]
pub use crate::graph::*;
// `#[macro_export]` macros live at the crate root; re-exporting them here
// lets the generated code's single glob import pick them up as well.
pub use crate::{file_record, fread, freadln, fwrite, fwriteln, pread, preadln, pwrite, pwriteln};
