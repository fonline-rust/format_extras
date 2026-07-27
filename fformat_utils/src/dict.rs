use core::error;
use std::{borrow::Borrow, ops::Bound};

use scc::{hash_index::Entry, Comparable};

use crate::{
    path::{conventional::ConventionalPath, unconventional::UnconventionalPathRef, url::FindByParent, ChecksumResult}, LocatorPath, PathChecksum
};

pub use scc::ebr::Guard;

pub struct DictEntry {
    pub conventional_path: ConventionalPath,
    pub checksum: PathChecksum,
}

#[cfg(not(feature = "ahash"))]
type DictHasher = std::hash::RandomState;
#[cfg(feature = "ahash")]
type DictHasher = ahash::RandomState;

type DictMap<K, V> = scc::hash_index::HashIndex<K, V, DictHasher>;
type DictTree<K, V> = scc::tree_index::TreeIndex<K, V>;
type PathMap = DictMap<PathChecksum, ConventionalPath>;
type Locations = DictTree<LocatorPath, Location>;

#[derive(Debug, Clone, Copy)]
pub struct Location {
    is_folder: bool,
}
impl Location {
    pub fn is_folder(&self) -> bool {
        self.is_folder
    }
}

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
    #[error("CRC32 collision for {checksum}: {stored:?} != {new:?} ")]
    Collision {
        checksum: PathChecksum,
        stored: ConventionalPath,
        new: ConventionalPath,
    },
    #[error("location {locator_path:?} had problem while indexing new file {file}: {reason:#}")]
    Location {
        checksum: PathChecksum,
        locator_path: LocatorPath,
        file: ConventionalPath,
        reason: IndexResult,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexResult {
    #[error("indexed successfully")]
    Indexed,
    #[error("folder already indexed")]
    FolderAlredyIndexed,
    #[error("file already indexed")]
    FileAlreadyIndexed,
    #[error("attempt to index folder and file with same path")]
    FolderFileConflict,
    #[error("storage failure - can't store, it's a bug")]
    CantInsert,
}
impl IndexResult {
    fn already_indexed(self, checksum: PathChecksum, locator_path: impl Into<LocatorPath>, file: &ConventionalPath) -> Result<bool, DictError> {
        match self {
            Self::Indexed => Ok(false),
            Self::FolderAlredyIndexed => Ok(true),
            _ => Err(DictError::Location { checksum, locator_path: locator_path.into(), file: file.clone(), reason: self })
        }
    }
}

impl DictError {
    pub fn checksum(&self) -> PathChecksum {
        match *self {
            DictError::Collision { checksum, .. } | DictError::Location { checksum, .. } => checksum,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum LocatorError {
    #[error("Invalid path")]
    InvalidPath,
    #[error("Can't find such folder")]
    CantFindFolder,
    #[error("Not a folder")]
    NotAFolder,
}

impl Dict {
    pub fn insert(&self, unconventional_path: &str) -> Result<PathChecksum, DictError> {
        Ok(self.insert_entry(unconventional_path)?.checksum)
    }

    pub fn insert_entry(&self, unconventional_path: &str) -> Result<DictEntry, DictError> {
        let unconventional = UnconventionalPathRef(unconventional_path);
        let ChecksumResult { checksum, utf8_len } = unconventional.checksum();
        let make_conventional = || unconventional.make_conventional(utf8_len);

        if let Some(stored) = self.get(checksum) {
            return if stored != unconventional {
                Err(DictError::Collision {
                    checksum,
                    stored,
                    new: make_conventional(),
                })
            } else {
                Ok(DictEntry {
                    conventional_path: stored,
                    checksum,
                })
            };
        }

        match self.to_path.entry(checksum) {
            Entry::Occupied(entry) => {
                let stored = entry.get().clone();
                drop(entry);
                if stored != unconventional {
                    Err(DictError::Collision {
                        checksum,
                        stored,
                        new: make_conventional(),
                    })
                } else {
                    Ok(DictEntry {
                        conventional_path: stored,
                        checksum,
                    })
                }
            }
            Entry::Vacant(entry) => {
                let new = make_conventional();
                entry.insert_entry(new.clone());
                self.remember_file(&new, checksum)?;
                Ok(DictEntry {
                    conventional_path: new,
                    checksum,
                })
            }
        }
    }

    pub fn get(&self, hash: PathChecksum) -> Option<ConventionalPath> {
        self.to_path.peek_with(&hash, |_, path| path.clone())
    }

    /*pub fn locate(&self, locator_path: &LocatorPathRef) -> Option<Location>{
        self.locations.peek_with(locator_path, |path, stored| (path, stored))
    }*/

    fn peek_location(&self, locator_path: &impl Comparable<LocatorPath>) -> Option<Location>{
        self.locations.peek_with(locator_path, |_, stored| *stored)
    }

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

    fn remember_one<P: Comparable<LocatorPath> + Into<LocatorPath>>(&self, locator_path: P, location: Location, tries: u8) -> IndexResult {
        if let Some(stored) = self.peek_location(&locator_path) {
            return match (stored.is_folder, location.is_folder) {
                (true, true) => IndexResult::FolderAlredyIndexed,
                (false, false) => IndexResult::FileAlreadyIndexed,
                _ => IndexResult::FolderFileConflict,
            }
        }
        match self.locations.insert(locator_path.into(), location) {
            Ok(()) => IndexResult::Indexed,
            Err((locator_path, _)) => {
                if tries == 0 {
                    IndexResult::CantInsert
                } else {
                    self.remember_one(locator_path, location, tries - 1)
                }
            }
        }
    }

    fn remember_file(&self, file: &ConventionalPath, checksum: PathChecksum) -> Result<(), DictError>{
        self.remember_one(file, Location { is_folder: false }, 2).already_indexed(checksum, file, file)?;
        for parent in file.parents() {
            if self.remember_one(parent, Location { is_folder: true }, 2).already_indexed(checksum, parent, file)? {
                return Ok(())
            }
        }
        Ok(())
    }
}
