mod dict;
mod path;
mod slice;

pub use self::{
    dict::{Dict, DictEntry, DictError, Location, Guard},
    path::{PathChecksum, conventional::ConventionalPath, url::LocatorPath},
    slice::trim_matches,
};
