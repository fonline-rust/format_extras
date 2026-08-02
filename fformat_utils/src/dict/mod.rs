use scc::hash_index::Entry;

use crate::{
    PathChecksum,
    path::{
        conventional::ConventionalPath,
        unconventional::{UnconventionalError, UnconventionalPathRef},
    },
};

mod locator;
#[cfg(test)]
mod tests;

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

#[derive(Default, Debug)]
pub struct Dict {
    // crc32 -> conventional path
    to_path: PathMap,
    locations: locator::Locations,
}

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
}
