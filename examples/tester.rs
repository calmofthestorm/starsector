use std::io::{Read, Result};

use starsector::*;

fn main() -> Result<()> {
    let mut arena = Arena::default();

    let mut fd = std::fs::File::open("/tmp/test.org")?;
    let mut s = String::default();

    fd.read_to_string(&mut s)?;

    let doc = arena.parse_str(&s);
    // println!("Hello yes this is dog: {:?}", &s[pp..pp + 50]);
    // println!("Result: {} {:?}", s.chars().count(), &doc.at(&arena, pp));

    for pp in 77..200 {
        if let Some((a, b)) = doc.at(&arena, pp) {
            println!("Result is {:?} {:?}", &a, &b);
        }
    }

    Ok(())
}
