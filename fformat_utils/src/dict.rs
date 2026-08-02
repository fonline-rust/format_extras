use scc::{Comparable, hash_index::Entry};

use crate::{
    LocatorPath, PathChecksum,
    path::{
        conventional::ConventionalPath,
        unconventional::{UnconventionalError, UnconventionalPathRef},
        url::FindByParent,
    },
};

pub struct DictEntry {
    pub conventional_path: ConventionalPath,
    pub checksum: PathChecksum,
    pub new: bool,
}

#[cfg(not(feature = "ahash"))]
type DictHasher = std::hash::RandomState;
#[cfg(feature = "ahash")]
type DictHasher = ahash::RandomState;

type DictMap<K, V> = scc::hash_index::HashIndex<K, V, DictHasher>;
type DictTree<K, V> = scc::tree_index::TreeIndex<K, V>;
type PathMap = DictMap<PathChecksum, ConventionalPath>;
type Locations = DictTree<LocatorPath, ()>;

#[derive(Default, Debug)]
pub struct Dict {
    // crc32 -> conventional path
    to_path: PathMap,
    locations: Locations,
}
/*
impl Dict {
    pub fn merge(&mut self, other: &mut Dict) -> Result<(), Vec<DictError>> {
        if self.to_path.is_empty() {
            std::mem::swap(&mut self.to_path, &mut other.to_path);
            return Ok(());
        }
        let mut errors = vec![];
        for (checksum, conventional) in other.to_path.drain() {
            match self.to_path.entry(checksum) {
                Entry::Vacant(entry) => {
                    entry.insert_entry(conventional);
                }
                Entry::Occupied(entry) => {
                    if *entry.get() != conventional {
                        errors.push(DictError::Collision {
                            checksum,
                            stored: entry.get().clone(),
                            new: conventional,
                        });
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
*/

#[derive(Debug, thiserror::Error)]
pub enum DictError {
    #[error("path can't be made conventional")]
    Unconventional(#[from] UnconventionalError),
    #[error("CRC32 collision for {checksum}: {stored:?} != {new:?} ")]
    Collision {
        checksum: PathChecksum,
        stored: ConventionalPath,
        new: ConventionalPath,
    },
}

#[derive(Debug, thiserror::Error, Clone, Copy)]
pub enum IndexResult {
    #[error("indexed successfully")]
    Indexed,
    #[error("already indexed")]
    AlredyIndexed,
}
impl IndexResult {
    fn already_indexed(self) -> bool {
        matches!(self, Self::AlredyIndexed)
    }
}

/*
#[derive(Debug, thiserror::Error)]
enum LocatorError {
    #[error("Invalid path")]
    InvalidPath,
    #[error("Can't find such folder")]
    CantFindFolder,
    #[error("Not a folder")]
    NotAFolder,
}
*/

impl Dict {
    pub fn insert(&self, unconventional_path: &str) -> Result<PathChecksum, DictError> {
        Ok(self.insert_entry(unconventional_path)?.checksum)
    }

    pub fn insert_entry(&self, unconventional_path: &str) -> Result<DictEntry, DictError> {
        let unconventional = UnconventionalPathRef::try_new(unconventional_path)?;
        let checksum = unconventional.checksum();

        let occupied = |stored| {
            if stored != unconventional {
                Err(DictError::Collision {
                    checksum,
                    stored,
                    new: unconventional.make_conventional(),
                })
            } else {
                Ok(DictEntry {
                    conventional_path: stored,
                    checksum,
                    new: false,
                })
            }
        };

        if let Some(stored) = self.get(checksum) {
            return occupied(stored);
        }

        match self.to_path.entry_sync(checksum) {
            Entry::Occupied(entry) => {
                let stored = entry.get().clone();
                drop(entry);
                occupied(stored)
            }
            Entry::Vacant(entry) => {
                let new = unconventional.make_conventional();
                entry.insert_entry(new.clone());
                self.remember_file(&new);
                Ok(DictEntry {
                    conventional_path: new,
                    checksum,
                    new: true,
                })
            }
        }
    }

    pub fn get(&self, hash: PathChecksum) -> Option<ConventionalPath> {
        self.to_path.peek_with(&hash, |_, path| path.clone())
    }

    fn peek_location(&self, locator_path: &impl Comparable<LocatorPath>) -> Option<LocatorPath> {
        self.locations.peek_with(locator_path, |key, _| key.clone())
    }

    /*
    pub fn iter_folder<'g>(&'g self, path: &'g str, guard: &'g Guard) -> Result<impl 'g + Iterator<Item = &'g str>, LocatorError>{
        let find = FindByParent::from_path(path).ok_or(LocatorError::InvalidPath)?;
        if !self.peek_location(locator_path).ok_or(LocatorError::CantFindFolder)?.is_folder {
            Err(LocatorError::NotAFolder)
        } else {

        }
        if locator_path.is_folder() {

            Some(self.locations.range::<LocatorPathRef, _>((Bound::Excluded(locator_path), Bound::Unbounded), guard)
                .map(|(k, _)| k.borrow())
                //.take_while(|path| path.p)
            )
        } else {
            None
        }
    }
     */

    fn remember_one<P: Into<LocatorPath> + Comparable<LocatorPath>>(
        &self,
        locator_path: P,
    ) -> IndexResult {
        if self.peek_location(&locator_path).is_some()
            || self.locations.insert_sync(locator_path.into(), ()).is_err()
        {
            IndexResult::AlredyIndexed
        } else {
            IndexResult::Indexed
        }
    }

    fn remember_file(&self, file: &ConventionalPath) {
        if !self.remember_one(file).already_indexed() {
            file.parents()
                .all(|parent| !self.remember_one(dbg!(&parent)).already_indexed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    impl Dict {
        fn collect_files(&self) -> Vec<String> {
            let mut vec: Vec<_> = self
                .to_path
                .iter(&mut scc::Guard::new())
                .map(|(_, path)| path.as_str().to_owned())
                .collect();
            vec.sort();
            vec
        }

        fn collect_locations(&self) -> Vec<String> {
            self.locations
                .iter(&mut scc::Guard::new())
                .map(|(path, _)| path.as_str().to_owned())
                .collect()
        }
    }

    #[test]
    fn test_insert_folder() {
        std::assert_matches!(
            Dict::default().insert("foo/"),
            Err(DictError::Unconventional(
                UnconventionalError::EndsWithSlash
            ))
        );
    }

    #[test]
    fn test_insert_file() {
        let dict = Dict::default();
        dict.insert("foo").unwrap();
        dbg!(&dict);
        assert_eq!(dict.collect_files(), &["foo"]);
        assert_eq!(dict.collect_locations(), &["foo"]);
    }

    #[test]
    fn test_insert_nested_file() {
        let dict = Dict::default();
        dict.insert("foo/bar").unwrap();
        dbg!(&dict);
        assert_eq!(dict.collect_files(), &["foo/bar"]);
        assert_eq!(dict.collect_locations(), &["foo/", "foo/bar"]);
    }

    #[test]
    fn test_insert_two_files() {
        let dict = Dict::default();
        dict.insert("foo").unwrap();
        dict.insert("bar").unwrap();
        dbg!(&dict);
        assert_eq!(dict.collect_files(), &["bar", "foo"]);
        assert_eq!(dict.collect_locations(), &["bar", "foo"]);
    }

    #[test]
    fn test_insert_two_files_with_colliding_prefix() {
        let dict = Dict::default();
        dict.insert("foo").unwrap();
        dict.insert("foo/bar").unwrap();
        dbg!(&dict);
        assert_eq!(dict.collect_files(), &["foo", "foo/bar"]);
        assert_eq!(dict.collect_locations(), &["foo", "foo/", "foo/bar"]);
    }
}
