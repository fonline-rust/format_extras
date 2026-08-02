use super::{PathChecksum, unconventional::UnconventionalPathRef};
use crate::arc_path::ArcPath;

pub struct Conventional;

pub type ConventionalPath = ArcPath<Conventional>;

impl ConventionalPath {
    pub fn checksum(&self) -> PathChecksum {
        PathChecksum(crc32fast::hash(self.as_str().as_bytes()))
    }
}

impl<'a> PartialEq<UnconventionalPathRef<'a>> for ConventionalPath {
    fn eq(&self, other: &UnconventionalPathRef<'a>) -> bool {
        let other = other.conventional_chars();
        matches!(
            self.as_str().chars().partial_cmp(other),
            Some(std::cmp::Ordering::Equal)
        )
    }
}
