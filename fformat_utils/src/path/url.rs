use std::{borrow::Borrow, cmp::Ordering, ops::Bound, sync::Arc};

use scc::{Comparable, Equivalent};
use serde::de::value;

use super::unconventional::UnconventionalPathRef;
use crate::{ConventionalPath, arc_path::ArcPath};

pub struct Locator;

pub type LocatorPath = ArcPath<Locator>;

impl Ord for LocatorPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.ptr_eq(other) {
            return std::cmp::Ordering::Equal;
        }
        self.as_path_ref().cmp(&other.as_path_ref())
    }
}
impl PartialOrd for LocatorPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub(crate) struct FindByParent<'a> {
    parent: &'a str,
    first: bool,
}

impl<'a> FindByParent<'a> {
    pub(crate) fn from_path(path: &'a str) -> Option<(Bound<Self>, Bound<Self>)> {
        let path = path.trim().trim_end_matches('/');
        if path.is_empty()
            || path.starts_with('/')
            || path.contains("//")
            || path.chars().any(|ch| ch.is_uppercase() || ch == '\\')
        {
            None
        } else {
            Some((
                Bound::Included(Self {
                    parent: path,
                    first: true,
                }),
                Bound::Excluded(Self {
                    parent: path,
                    first: false,
                }),
            ))
        }
    }
}

impl<'a> Equivalent<LocatorPath> for FindByParent<'a> {
    fn equivalent(&self, key: &LocatorPath) -> bool {
        let (parent, _) = key.split_parent();
        if self.first {
            self.parent.eq(parent)
        } else {
            self.parent.ne(parent)
        }
    }
}

impl<'a> Comparable<LocatorPath> for FindByParent<'a> {
    fn compare(&self, key: &LocatorPath) -> std::cmp::Ordering {
        let (parent, _) = key.split_parent();
        if self.first {
            self.parent.cmp(parent)
        } else {
            match self.parent.cmp(parent) {
                Ordering::Less | Ordering::Equal => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
            }
        }
    }
}

impl Equivalent<LocatorPath> for &ConventionalPath {
    fn equivalent(&self, key: &LocatorPath) -> bool {
        LocatorPathRef(self.as_str()).eq(&key.as_path_ref())
    }
}

impl Comparable<LocatorPath> for &ConventionalPath {
    fn compare(&self, key: &LocatorPath) -> std::cmp::Ordering {
        LocatorPathRef(self.as_str()).cmp(&key.as_path_ref())
    }
}

impl LocatorPath {
    fn split_parent(&self) -> (&str, &str) {
        let str = self.as_str().trim_end_matches("/");
        if let Some((pos, _)) = str.char_indices().rev().find(|(_, ch)| *ch == '/') {
            (&str[..=pos], &str[pos + 1..])
        } else {
            ("", str)
        }
    }

    fn as_path_ref(&self) -> LocatorPathRef<'_> {
        LocatorPathRef(self.as_str())
    }

    pub fn is_folder(&self) -> bool {
        self.as_str().ends_with('/')
    }

    pub fn is_file(&self) -> bool {
        !self.is_folder()
    }

    // TODO: proper path len
    // TODO: get it from dict?
    /*
    pub fn make_conventional(path: &str) -> Self {
        UnconventionalPathRef(path).make_conventional(path.len())
    }
    */
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct LocatorPathRef<'a>(&'a str);

impl<'a> LocatorPathRef<'a> {
    fn parts(&self) -> impl Iterator<Item = &str> {
        self.0.split_inclusive('/')
    }

    pub(crate) fn split_last(&self) -> (LocatorPathRef<'a>, &'a str) {
        let str = self.0;
        let trimmed = str.trim_end_matches("/");
        if let Some((pos, _)) = trimmed.char_indices().rev().find(|(_, ch)| *ch == '/') {
            (LocatorPathRef(&str[..pos]), &str[pos + 1..])
        } else {
            (LocatorPathRef(""), str)
        }
    }
}
impl<'a> From<LocatorPathRef<'a>> for LocatorPath {
    fn from(value: LocatorPathRef<'a>) -> Self {
        ArcPath::new(value.0)
    }
}
impl<'a> From<&LocatorPathRef<'a>> for LocatorPath {
    fn from(value: &LocatorPathRef<'a>) -> Self {
        ArcPath::new(value.0)
    }
}

impl<'a> Ord for LocatorPathRef<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (this_parent, this_last) = self.split_last();
        let (other_parent, other_last) = other.split_last();
        std::cmp::Ordering::Equal
            //.then_with(|| this_parent.parts().count().cmp(&other_parent.parts().count()))
            .then_with(|| this_parent.parts().cmp(other_parent.parts()))
            //.then_with(|| self.is_folder().cmp(&other.is_folder()).reverse())
            .then_with(|| this_last.cmp(other_last))
    }
}
impl<'a> PartialOrd for LocatorPathRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a> Equivalent<LocatorPath> for &LocatorPathRef<'a> {
    fn equivalent(&self, key: &LocatorPath) -> bool {
        (*self).eq(&key.as_path_ref())
    }
}

impl<'a> Comparable<LocatorPath> for &LocatorPathRef<'a> {
    fn compare(&self, key: &LocatorPath) -> std::cmp::Ordering {
        (*self).cmp(&key.as_path_ref())
    }
}

impl From<ConventionalPath> for LocatorPath {
    fn from(value: ConventionalPath) -> Self {
        ArcPath::new(value.into_inner())
    }
}
impl From<&ConventionalPath> for LocatorPath {
    fn from(value: &ConventionalPath) -> Self {
        ArcPath::new(value.clone().into_inner())
    }
}

impl ConventionalPath {
    pub(crate) fn parents(&self) -> impl Iterator<Item = LocatorPathRef<'_>> {
        let str = self.as_str();
        str.char_indices()
            .rev()
            .filter(|(_, ch)| *ch == '/')
            .map(|(end, ..)| LocatorPathRef(&str[..=end]))
    }
}
