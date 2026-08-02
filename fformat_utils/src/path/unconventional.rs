use super::{ChecksumResult, PathChecksum};
use crate::arc_path::ArcPath;

#[derive(Debug, Clone, Copy)]
pub(crate) struct UnconventionalPathRef<'a>(pub &'a str);

impl<'a> UnconventionalPathRef<'a> {
    pub(crate) fn make_conventional<META>(&self, utf8_len: usize) -> ArcPath<META> {
        let mut buf = String::with_capacity(utf8_len);
        buf.extend(self.conventional_chars());
        ArcPath::new(buf)
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

    pub(super) fn conventional_chars(&self) -> impl '_ + Iterator<Item = char> {
        let path = self.0.trim().trim_end_matches(['/', '\\']);
        path.chars()
            .map(|ch| if ch == '\\' { '/' } else { ch })
            .flat_map(|ch| ch.to_lowercase())
    }
}
