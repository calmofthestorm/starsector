use starsector::*;

use rand::RngCore;

// Test that we can parse gibberish without crashing, and that parsing it,
// writing out a headline, then parsing that, is identical to the initial
// parse.
#[test]
fn parse_fuzz() {
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(36);

    let mut alphabet: Vec<String> = " \n\t@:#[]*A9饭".chars().map(|c| c.to_string()).collect();

    alphabet.extend(
        vec![
            "\n**",
            "\n** ",
            "\n*****",
            "\n\n",
            "TODO ",
            "COMMENT ",
            "[#B] ",
            " :hello: ",
            " :a:b:",
        ]
        .iter()
        .map(|x| x.to_string()),
    );

    let mut arena = Arena::default();

    for _ in 0..25000 {
        let mut s = String::default();

        for _ in 0..(rng.next_u32() % 100) {
            let c = (rng.next_u32() as usize) % alphabet.len();
            s += &alphabet[c];
        }

        let doc = arena.parse_str(&s);
        if let Some(headline) = doc.root.parse_headline(&mut arena, None) {
            headline.to_builder().headline(None).unwrap();
        }
    }
}
