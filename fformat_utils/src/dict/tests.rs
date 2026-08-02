use super::*;

impl Dict {
    fn collect_files(&self) -> Vec<String> {
        let mut vec: Vec<_> = self
            .to_path
            .iter(&mut scc::Guard::new())
            .map(|(_, path)| path.as_str().to_owned())
            .collect();
        vec.sort();
        vec
    }

    fn collect_locations(&self) -> Vec<String> {
        self.locations
            .iter(&mut scc::Guard::new())
            .map(|(path, _)| path.as_str().to_owned())
            .collect()
    }
}

#[test]
fn test_insert_folder() {
    std::assert_matches!(
        Dict::default().insert("foo/"),
        Err(DictError::Unconventional(
            UnconventionalError::EndsWithSlash
        ))
    );
}

#[test]
fn test_insert_file() {
    let dict = Dict::default();
    dict.insert("foo").unwrap();
    dbg!(&dict);
    assert_eq!(dict.collect_files(), &["foo"]);
    assert_eq!(dict.collect_locations(), &["foo"]);
}

#[test]
fn test_insert_nested_file() {
    let dict = Dict::default();
    dict.insert("foo/bar").unwrap();
    dbg!(&dict);
    assert_eq!(dict.collect_files(), &["foo/bar"]);
    assert_eq!(dict.collect_locations(), &["foo/", "foo/bar"]);
}

#[test]
fn test_insert_two_files() {
    let dict = Dict::default();
    dict.insert("foo").unwrap();
    dict.insert("bar").unwrap();
    dbg!(&dict);
    assert_eq!(dict.collect_files(), &["bar", "foo"]);
    assert_eq!(dict.collect_locations(), &["bar", "foo"]);
}

#[test]
fn test_insert_two_files_with_colliding_prefix() {
    let dict = Dict::default();
    dict.insert("foo").unwrap();
    dict.insert("foo/bar").unwrap();
    dbg!(&dict);
    assert_eq!(dict.collect_files(), &["foo", "foo/bar"]);
    assert_eq!(dict.collect_locations(), &["foo", "foo/", "foo/bar"]);
}
