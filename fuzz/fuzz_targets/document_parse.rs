#![no_main]

use libfuzzer_sys::fuzz_target;
use starsector::Arena;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    let mut arena = Arena::default();
    let doc = arena.parse_str(text);

    // Core project invariant: parse + emit is identity for valid UTF-8 input.
    let emitted = doc.to_rope(&arena).to_string();
    assert_eq!(emitted, text);

    // Reparse stability.
    let mut arena2 = Arena::default();
    let reparsed = arena2.parse_str(&emitted);
    assert_eq!(reparsed.to_rope(&arena2).to_string(), emitted);

    // Optional differential parsing can disagree on edge syntax (e.g. lone
    // stars) while our core invariant still holds. Keep this non-fatal.
    let org = orgize::Org::parse(text);
    let _ = (
        doc.root.children(&arena).count(),
        org.document().children(&org).count(),
    );
});
