#[cfg(feature = "headline-parser")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "headline-parser")]
mod headline;

mod arena;
mod emit;
mod errors;
mod iter;
mod parser;
mod ropeext;
mod tree;

#[cfg(feature = "orgize-integration")]
mod orgize_util;

pub mod util {
    pub use super::parser::structure::lex_level;
    pub use super::parser::structure::lex_level_str;
    pub use super::parser::structure::line;
}

#[cfg(feature = "headline-parser")]
pub use crate::headline::*;

pub use crate::arena::*;
pub use crate::errors::*;
pub use crate::iter::*;
pub(crate) use crate::orgize_util::*;
pub use crate::ropeext::*;
pub use crate::tree::*;
