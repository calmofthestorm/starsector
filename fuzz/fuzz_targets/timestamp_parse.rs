#![no_main]

use libfuzzer_sys::fuzz_target;
use starsector::Timestamp;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok((_rest, parsed)) = Timestamp::parse(text) {
        let emitted = parsed.to_string();
        let (rest2, reparsed) = Timestamp::parse(&emitted).expect("emitted timestamp must parse");
        assert_eq!(rest2, "");
        assert_eq!(reparsed, parsed);
    }
});
