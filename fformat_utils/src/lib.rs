mod dict;
mod path;
mod slice;
mod arc_path;

pub use self::{
    dict::{Dict, DictEntry, DictError, Location},
    path::{PathChecksum, conventional::ConventionalPath, url::LocatorPath},
    slice::trim_matches,
};
