#[cfg(feature = "dict")]
pub mod dict;
mod path;
mod slice;
#[cfg(feature = "ini_to_toml")]
pub mod ini_to_toml;

pub use self::{
    path::{ConventionalPath, PathChecksum},
    slice::trim_matches,
};

#[cfg(feature = "smallvec")]
pub type List<T> = smallvec::SmallVec<[T; 3]>;
#[cfg(feature = "smallvec")]
pub use smallvec;
