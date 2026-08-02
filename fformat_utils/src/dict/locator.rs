use scc::Comparable;

use crate::{ConventionalPath, Dict, LocatorPath};

pub(super) type Locations = super::DictTree<LocatorPath, ()>;

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

    fn peek_location(&self, locator_path: &impl Comparable<LocatorPath>) -> Option<LocatorPath> {
        self.locations.peek_with(locator_path, |key, _| key.clone())
    }

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

    pub(super) fn remember_file(&self, file: &ConventionalPath) {
        if !self.remember_one(file).already_indexed() {
            file.parents()
                .all(|parent| !self.remember_one(dbg!(&parent)).already_indexed());
        }
    }
}
