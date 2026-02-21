use ropey::Rope;
use starsector::{RopeExt, RopeSliceExt};

#[test]
fn test_slice_bytes_ascii_and_unicode() {
    let rope = Rope::from("ab饭cd");
    let slice = rope.slice(..);

    // "ab"
    assert_eq!(rope.slice_bytes(0..2).to_string(), "ab");
    assert_eq!(slice.slice_bytes(0..2).to_string(), "ab");

    // "饭" occupies bytes 2..5 in UTF-8.
    assert_eq!(rope.slice_bytes(2..5).to_string(), "饭");
    assert_eq!(slice.slice_bytes(2..5).to_string(), "饭");

    // Inclusive end over "c" (byte index 5).
    assert_eq!(rope.slice_bytes(..=5).to_string(), "ab饭c");
    assert_eq!(slice.slice_bytes(..=5).to_string(), "ab饭c");
}

#[test]
fn test_discontangle_round_trip() {
    let rope = Rope::from("alpha\nbeta\ngamma");
    let full = rope.slice(..);
    let contiguous = full.to_contiguous();
    let contiguous = &*contiguous;

    let sub = &contiguous[6..10];
    let mapped = full.discontangle(contiguous, sub);
    assert_eq!(mapped.to_string(), "beta");

    let empty = full.discontangle(contiguous, "");
    assert_eq!(empty.len_bytes(), 0);
}

#[test]
fn test_memchr_found_and_not_found() {
    let rope = Rope::from("a\nb\nc");
    let slice = rope.slice(..);

    assert_eq!(rope.memchr(b'\n', 0), 1);
    assert_eq!(rope.memchr(b'\n', 2), 3);
    assert_eq!(rope.memchr(b'x', 0), rope.len_bytes());

    assert_eq!(slice.memchr(b'\n', 0), 1);
    assert_eq!(slice.memchr(b'\n', 2), 3);
    assert_eq!(slice.memchr(b'x', 0), slice.len_bytes());
}

#[test]
fn test_find_and_rfind() {
    let rope = Rope::from("one two one two");
    let slice = rope.slice(..);

    assert_eq!(rope.find("two"), Some(4));
    assert_eq!(rope.rfind("two"), Some(12));
    assert_eq!(rope.find("missing"), None);
    assert_eq!(rope.rfind("missing"), None);

    assert_eq!(slice.find("two"), Some(4));
    assert_eq!(slice.rfind("two"), Some(12));
    assert_eq!(slice.find("missing"), None);
    assert_eq!(slice.rfind("missing"), None);
}
