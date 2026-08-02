use std::path;

use super::PathChecksum;
use crate::arc_path::ArcPath;

#[derive(Debug, Clone, Copy)]
pub(crate) struct UnconventionalPathRef<'a> {
    path: &'a str,
    checksum: PathChecksum,
    utf8_len: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum UnconventionalError {
    #[error("Path ends with (back)slash")]
    EndsWithSlash,
    #[error("Path ends with (back)slash")]
    SequentialSlashes,
}

impl<'a> UnconventionalPathRef<'a> {
    pub(crate) fn try_new(path: &'a str) -> Result<Self, UnconventionalError> {
        let path = path.trim();
        if path.ends_with(['/', '\\']) {
            return Err(UnconventionalError::EndsWithSlash);
        }
        if path
            .as_bytes()
            .array_windows()
            .any(|arr| matches!(arr, b"//" | b"\\\\" | b"/\\" | b"\\/"))
        {
            return Err(UnconventionalError::SequentialSlashes);
        }

        let mut chars = conventional_chars(path);

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
        Ok(Self {
            path,
            checksum,
            utf8_len,
        })
    }

    pub(crate) fn checksum(&self) -> PathChecksum {
        self.checksum
    }

    pub(crate) fn make_conventional<META>(&self) -> ArcPath<META> {
        let mut buf = String::with_capacity(self.utf8_len);
        buf.extend(self.conventional_chars());
        ArcPath::new(buf)
    }

    pub(super) fn conventional_chars(&self) -> impl '_ + Iterator<Item = char> {
        conventional_chars(&self.path)
    }
}

fn conventional_chars(path: &str) -> impl '_ + Iterator<Item = char> {
    path.chars()
        .map(|ch| if ch == '\\' { '/' } else { ch })
        .flat_map(|ch| ch.to_lowercase())
}
