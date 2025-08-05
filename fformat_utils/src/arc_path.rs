use std::{fmt, marker::PhantomData, sync::Arc};

pub struct ArcPath<META>(Arc<str>, PhantomData<META>);

impl<META> Eq for ArcPath<META> {}

impl<META> PartialEq for ArcPath<META> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<META> Clone for ArcPath<META> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<META> fmt::Debug for ArcPath<META> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<META> fmt::Display for ArcPath<META> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<META> ArcPath<META> {
    pub(crate) fn new(string: impl Into<Arc<str>>) -> Self {
        Self(string.into(), PhantomData)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_inner(self) -> Arc<str> {
        self.0
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
