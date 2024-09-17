use std::{fmt, sync::Arc};

#[derive(Debug, Clone, PartialEq)]
pub struct ConventionalPath(Arc<str>);

impl fmt::Display for ConventionalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
#[repr(transparent)]
pub struct PathChecksum(u32);

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
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl ConventionalPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn checksum(&self) -> PathChecksum {
        PathChecksum(crc32fast::hash(self.0.as_bytes()))
    }
}

impl<'a> PartialEq<UnconventionalPathRef<'a>> for ConventionalPath {
    fn eq(&self, other: &UnconventionalPathRef<'a>) -> bool {
        let other = other.conventional_chars();
        matches!(
            self.0.chars().partial_cmp(other),
            Some(std::cmp::Ordering::Equal)
        )
    }
}

pub(crate) struct ChecksumResult {
    pub(crate) checksum: PathChecksum,
    pub(crate) utf8_len: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UnconventionalPathRef<'a>(pub &'a str);

impl<'a> UnconventionalPathRef<'a> {
    pub(crate) fn make_conventional(&self, utf8_len: usize) -> ConventionalPath {
        let mut buf = String::with_capacity(utf8_len);
        buf.extend(self.conventional_chars());
        ConventionalPath(buf.into())
    }

    pub(crate) fn checksum(&self) -> ChecksumResult {
        let mut chars = self.conventional_chars();

        let mut hasher = crc32fast::Hasher::new();

        let mut buf = [0u8; 32 * 4];
        let mut buf_len = 0;
        let mut finish = false;
        let mut utf8_len = 0;
        while !finish {
            for _ in 0..32 {
                let Some(ch) = chars.next() else {
                    finish = true;
                    break;
                };
                ch.encode_utf8(&mut buf[buf_len..]);
                buf_len += ch.len_utf8();
            }
            hasher.update(&buf[..buf_len]);
            utf8_len += buf_len;
            buf_len = 0;
        }
        let checksum = PathChecksum(hasher.finalize());
        ChecksumResult { checksum, utf8_len }
    }

    fn conventional_chars(&self) -> impl '_ + Iterator<Item = char> {
        let path = self.0.trim().trim_end_matches(['/', '\\']);
        path.chars()
            .map(|ch| if ch == '\\' { '/' } else { ch })
            .flat_map(|ch| ch.to_lowercase())
    }
}

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
            UnconventionalPathRef(path).checksum().checksum.0
        }
        use super::UnconventionalPathRef;
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo/bar/"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo\\bar"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo\\bar\\"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("foo/BAR"));
        assert_eq!(crc32fast::hash(b"foo/bar"), checksum("FOO\\bar"));
    }
}
