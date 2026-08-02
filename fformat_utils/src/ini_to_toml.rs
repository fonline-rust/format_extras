use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
#[error("line {line_num}: {kind:#}: {line:?}")]
pub struct Error<'a> {
    pub kind: ErrorKind,
    pub line_num: usize,
    pub line: Cow<'a, str>,
}
impl<'a> Error<'a> {
    pub fn into_owned(self) -> Error<'static> {
        let Self {
            kind,
            line_num,
            line,
        } = self;
        Error {
            kind,
            line_num,
            line: Cow::Owned(line.into_owned()),
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy)]
pub enum ErrorKind {
    #[error("malformed section")]
    MalformedSection,
    #[error("line without equals sign")]
    LineWithoutEquals,
}

pub fn translate<'a>(ini: &'a str, with_comments: bool) -> Result<String, Error<'a>> {
    let mut toml = String::with_capacity(ini.len() + ini.len() / 20);
    for (line_num, mut line) in ini.lines().enumerate() {
        line = line.trim_start_matches('\u{feff}').trim();
        let (line, comment) = split_until(line, '#');
        let line = line.trim_end();
        let err = |kind| Error {
            kind,
            line: Cow::Borrowed(line),
            line_num,
        };

        if line.starts_with('[') {
            if !line.ends_with(']') || line.len() < 3 {
                return Err(err(ErrorKind::MalformedSection));
            }
            let section = &line[1..line.len() - 1].trim();
            if section.is_empty() {
                return Err(err(ErrorKind::MalformedSection));
            }
            if section.contains(' ') {
                let sections = section.split(' ').map(|sec| {
                    if sec.parse::<u16>().is_ok() {
                        ["", sec, ""]
                    } else {
                        ["\"", sec, "\""]
                    }
                });
                toml.push_str("[");
                toml.extend(itertools::intersperse(sections, [".", "", ""]).flatten());
                toml.push_str("]")
            } else {
                toml.push_str("[[\"");
                toml.push_str(section);
                toml.push_str("\"]]")
            }
        } else if !line.is_empty() {
            let (mut key, val) = split_until(line, '=');
            let mut second_key = None;
            if let Some(val) = val {
                if let Some((new_key, num)) = key.rsplit_once('_') {
                    if num.parse::<u8>().is_ok() {
                        key = new_key;
                        second_key = Some(num);
                    }
                }

                toml.extend(itertools::intersperse(key.split('.'), "_"));
                if let Some(key) = second_key {
                    toml.push('.');
                    toml.push_str(key);
                }
                toml.push('=');
                if val == "0"
                    || !val.starts_with('0')
                        && (1..20).contains(&val.len())
                        && val
                            .bytes()
                            .all(|byte| byte == b'-' || byte.is_ascii_digit())
                {
                    //let val_u64: Result<i64, _> = val.parse();
                    //if val_u64.is_ok(){
                    toml.push_str(val);
                } else {
                    toml.push('"');
                    toml.extend(itertools::intersperse(val.split('\\'), "\\\\"));
                    toml.push('"');
                }
            } else {
                return Err(err(ErrorKind::LineWithoutEquals));
            }
        }

        if with_comments {
            if let Some(comment) = comment {
                toml.push_str(" #");
                toml.push_str(comment);
            }
        }

        toml.push('\n');
    }
    Ok(toml)
}

fn split_until(s: &str, delimeter: char) -> (&str, Option<&str>) {
    let mut split = s.splitn(2, delimeter);
    (
        split
            .next()
            .expect("first SplitN::next() is always Some")
            .trim_end(),
        split.next().map(|s| s.trim_start()),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fopro_generic() {
        let ini = std::fs::read_to_string("../../FO4RP/proto/items/generic.fopro").unwrap();
        let _toml = translate(&ini, true).unwrap();
        let _ = dbg!(_toml);
    }
    #[test]
    fn fopro_food() {
        let ini = std::fs::read_to_string("../../FO4RP/proto/items/food.fopro").unwrap();
        let _toml = translate(&ini, true).unwrap();
        let _ = dbg!(_toml);
    }
    #[test]
    fn locations() {
        let ini = std::fs::read_to_string("../../FO4RP/maps/Locations.cfg").unwrap();
        let _toml = translate(&ini, true).unwrap();
        let _ = dbg!(_toml);
    }
}
