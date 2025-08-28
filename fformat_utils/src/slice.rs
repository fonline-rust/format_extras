pub fn trim_matches<'a, T: PartialEq>(mut slice: &'a [T], matcher: &[T]) -> &'a [T] {
    while let Some(new_slice) = slice.strip_prefix(matcher) {
        slice = new_slice;
    }
    while let Some(new_slice) = slice.strip_suffix(matcher) {
        slice = new_slice;
    }
    slice
}
