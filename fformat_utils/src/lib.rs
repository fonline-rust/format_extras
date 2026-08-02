mod arc_path;
#[cfg(feature = "dict")]
pub mod dict;
#[cfg(feature = "ini_to_toml")]
pub mod ini_to_toml;
mod path;
mod slice;

pub use self::{
    dict::{Dict, DictEntry, DictError},
    path::{PathChecksum, conventional::ConventionalPath, url::LocatorPath},
    slice::trim_matches,
};

#[cfg(feature = "smallvec")]
pub type List<T> = smallvec::SmallVec<[T; 3]>;
#[cfg(feature = "smallvec")]
pub use smallvec;
