//! Everything generated code needs, in one `use`.
//!
//! A "prelude" is a Rust convention: a module that re-exports the most
//! commonly used items of a crate so that users can glob-import them with
//! `use some_crate::prelude::*;`. The standard library has one too
//! (`std::prelude`), which is why `Vec` and `Option` are always in scope.

pub use crate::array::PArray;
pub use crate::bytes::{
    ByteView, Bytes, bytes_after, bytes_from, fill_bytes, move_bytes, sized_bytes, sized_view_bytes, string_bytes,
    string_view_bytes,
};
pub use crate::io::{FileRecord, PBlockFile, PFile, PText, PasWrite, eof, eoln, seek_eof, seek_eoln, w, wd};
pub use crate::ordinal::{Enumerated, Ordinal, enum_range};
pub use crate::overlay::*;
pub use crate::pchar::*;
pub use crate::printer::Lst;
pub use crate::ptr::{AnyPtr, ProcVar, Ptr};
pub use crate::run::{halt, io_result, io_unchecked, rte, run_error};
pub use crate::set::PSet;
pub use crate::string::{PString, pos, pstr};
pub use crate::system::{
    ch_dir, exit_code, file_mode, get_dir, max_avail, mem_avail, mk_dir, param_count, param_str, pas_str, rand_seed,
    random, random_real, random_word, randomize, rm_dir, set_exit_code, set_file_mode, set_rand_seed, val,
};
pub use crate::variants::{StringField, VariantField, VariantPart};
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
pub use crate::{file_record, fread, freadln, fwrite, fwriteln, pread, preadln, pset, pwrite, pwriteln};
