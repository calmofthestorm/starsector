#![no_main]

use libfuzzer_sys::fuzz_target;
use starsector::{Arena, Context, parse_valid_single_headline};

fn assert_reparse_converges(mut emitted: String) {
    for _ in 0..16 {
        let reparsed = parse_valid_single_headline(
            ropey::Rope::from(emitted.clone()).slice(..),
            &Context::default(),
        );
        let next = reparsed.to_rope().to_string();
        if next == emitted {
            return;
        }
        emitted = next;
    }
    panic!("headline parse/emit did not converge within bound");
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    let mut arena = Arena::default();
    let doc = arena.parse_str(text);

    // Any discovered headline must parse/emit/reparse stably as a single headline.
    for section in doc.root.descendants(&arena) {
        if let Some(headline) = section.headline(&arena, None) {
            let emitted = headline.to_rope().to_string();
            assert_reparse_converges(emitted);
        }
    }
});
