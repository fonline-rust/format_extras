mod dict;
mod path;
mod slice;

pub use self::{
    dict::{Dict, DictEntry, DictError},
    path::{ConventionalPath, PathChecksum},
    slice::trim_matches,
};
