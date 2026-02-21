use std::io::Cursor;

use ropey::Rope;
use starsector::{Arena, util};

fn main() {
    let input = "Intro\n* One\nBody\n** Child\n";
    println!("Input org:\n{input}");

    let mut arena = Arena::default();

    let from_str = arena.parse_str(input);
    let from_string = arena.parse_string(input.to_string());
    let from_rope = arena.parse_rope(Rope::from(input));

    let rope = Rope::from(input);
    let from_slice = arena.parse(&rope.slice(..));

    let mut reader = Cursor::new(input.as_bytes());
    let from_reader = arena
        .parse_reader(&mut reader)
        .expect("parse_reader failed");

    let emitted = from_str.to_rope(&arena).to_string();
    assert_eq!(emitted, from_string.to_rope(&arena).to_string());
    assert_eq!(emitted, from_rope.to_rope(&arena).to_string());
    assert_eq!(emitted, from_slice.to_rope(&arena).to_string());
    assert_eq!(emitted, from_reader.to_rope(&arena).to_string());

    let out_rope = from_str.to_rope(&arena);
    let out_slice = out_rope.slice(..);
    let (line, rest) = util::line(&out_slice);
    assert_eq!(line.to_string(), "Intro");
    assert!(rest.to_string().contains("* One"));

    let headline = Rope::from("*** heading");
    assert_eq!(util::lex_level(&headline.slice(..)), 3);
    assert_eq!(util::lex_level_str("** h"), 2);

    println!("Parsed/emitted org:\n{emitted}");
    println!("All parse entrypoints produced equivalent documents.");
}
