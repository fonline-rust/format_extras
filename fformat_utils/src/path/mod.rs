use std::fmt;

pub(crate) mod conventional;
pub(crate) mod unconventional;
pub(crate) mod url;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
#[repr(transparent)]
pub struct PathChecksum(u32);

impl fmt::Debug for PathChecksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "serde")]
mod impl_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<'de> Deserialize<'de> for super::PathChecksum {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            u32::deserialize(deserializer).map(Self)
        }
    }

    impl Serialize for super::PathChecksum {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.0.serialize(serializer)
        }
    }
}

impl fmt::Display for PathChecksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PathChecksum {
    pub fn value(self) -> u32 {
        self.0
    }
}
/*
pub(crate) struct ChecksumResult {
    pub(crate) checksum: PathChecksum,
    pub(crate) utf8_len: usize,
}
*/

#[cfg(test)]
mod test {
    //[16:860] Script callback: qwerty - 55151997 : main : void init() : 424, 2 : FOServer::InitReal : Game.
    //[16:860] Script callback: 123456 - 158520161 : main : void init() : 425, 2 : FOServer::InitReal : Game.
    //[16:860] Script callback: !@#$%^ - 3424808321 : main : void init() : 426, 2 : FOServer::InitReal : Game.

    #[test]
    fn verify_crc32() {
        assert_eq!(crc32fast::hash(b"qwerty"), 55151997);
        assert_eq!(crc32fast::hash(b"123456"), 158520161);
        assert_eq!(crc32fast::hash(b"!@#$%^"), 3424808321);
    }

    #[test]
    fn verify_unconventional() {
        fn checksum(path: &str) -> u32 {
            UnconventionalPathRef::try_new(path).unwrap().checksum().0
        }
        use super::unconventional::UnconventionalPathRef;
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo/bar/"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo\\bar"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo\\bar\\"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo/BAR"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("FOO\\bar"));
    }
}
