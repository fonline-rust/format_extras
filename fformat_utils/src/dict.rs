use scc::hash_index::Entry;

use crate::{
    path::{ChecksumResult, ConventionalPath, UnconventionalPathRef},
    PathChecksum,
};

pub struct DictEntry {
    pub conventional_path: ConventionalPath,
    pub checksum: PathChecksum,
}

#[cfg(not(feature = "ahash"))]
type DictHasher = std::hash::RandomState;
#[cfg(feature = "ahash")]
type DictHasher = ahash::RandomState;

type DictMap = scc::hash_index::HashIndex<PathChecksum, ConventionalPath, DictHasher>;

#[derive(Default, Debug)]
pub struct Dict {
    // crc32 -> conventional path
    to_path: DictMap,
}

/*
impl Dict {
    pub fn merge(&mut self, other: &mut Dict) -> Result<(), Vec<DictError>> {
        if self.to_path.is_empty() {
            std::mem::swap(&mut self.to_path, &mut other.to_path);
            return Ok(());
        }
        let mut errors = vec![];
        for (hash, conventional) in other.to_path.drain() {
            match self.to_path.entry(hash) {
                Entry::Vacant(entry) => {
                    entry.insert_entry(conventional);
                }
                Entry::Occupied(entry) => {
                    if *entry.get() != conventional {
                        errors.push(DictError::Collision {
                            hash,
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
}

impl DictError {
    pub fn checksum(&self) -> PathChecksum {
        match *self {
            DictError::Collision { checksum, .. } => checksum,
        }
    }
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

        match self.to_path.entry_sync(checksum) {
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
}
