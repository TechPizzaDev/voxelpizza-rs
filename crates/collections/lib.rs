#![feature(allocator_api)]
#![feature(new_range_api)]
#![feature(associated_type_defaults)]

mod index_map;
pub mod pack;
mod subslice;

pub use index_map::*;
pub use subslice::*;
