pub fn make_path_conventional(path: &str) -> String {
    let mut buf = String::with_capacity(path.len());
    write_conventional_path(path, &mut buf);
    buf
}

pub fn write_conventional_path(path: &str, buf: &mut String) {
    buf.clear();
    let path = path.trim().trim_end_matches(|ch| ch == '/' || ch == '\\');
    for ch in path.chars() {
        if ch == '\\' {
            buf.push('/');
        } else {
            buf.extend(ch.to_lowercase())
        }
    }
}
