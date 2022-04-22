use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::io::{Read, Result, Write};
use std::path::PathBuf;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use itertools::Itertools;
use rand::Rng;
use rand::RngCore;
use ropey::Rope;

use starsector::*;

fn usage(name: &str) -> Result<()> {
    println!("Parses all Org files (recurses into directories) given on the command line.");
    println!(
        "Verifies that Orgize and Starsector parse them consistently, modulo known differences."
    );
    // FIXME: actually do properties and planning
    println!("Note that this is not an exhaustive check, and any errors on your org files may simply be known differences the logic here could not handle.");
    println!("Checks structure parsing, headline parsing, and properties/planning parsing.\n");
    println!(
        "usage: {} <path> [<path>...] [--fuzz=<iter,threads>] --mutate=<iter,threads>",
        name
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = ::std::env::args().collect();

    for arg in &args {
        if arg == "--help" || arg == "-h" {
            return usage(&args[0]);
        }
    }

    if args.len() < 2 {
        return usage(&args[0]);
    }

    let r = regex::Regex::new("\n(\\*+)($|\t|\n)").unwrap();

    let mut files = HashMap::new();
    let mut skips = HashSet::new();
    let mut io_errors = 0;
    let mut fuzz = Vec::new();
    let mut mutate = Vec::new();
    for arg in &args[1..] {
        if arg.starts_with("--mutate=") {
            let nums: Vec<_> = arg[9..]
                .split(',')
                .map(|n| n.parse::<usize>().unwrap_or(0))
                .collect();
            if nums.len() == 2 && nums[0] > 0 && nums[1] > 0 {
                mutate.push((nums[0], nums[1]));
                continue;
            }
            println!("Use --mutate=<iter,threads> (use no space, and both must be > 0)");
            return Ok(());
        }

        if arg.starts_with("--fuzz=") {
            let nums: Vec<_> = arg[7..]
                .split(',')
                .map(|n| n.parse::<usize>().unwrap_or(0))
                .collect();
            if nums.len() == 2 && nums[0] > 0 && nums[1] > 0 {
                fuzz.push((nums[0], nums[1]));
                continue;
            }
            println!("Use --fuzz=<iter,threads> (use no space, and both must be > 0)");
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&arg).follow_links(true) {
            let entry = match &entry {
                Err(e) => {
                    println!("I/O error reading {:?}: {:?}\n", &entry, &e);
                    io_errors += 1;
                    continue;
                }
                Ok(entry) => entry,
            };

            let entry_type = entry.file_type();
            if entry_type.is_dir()
                || !entry
                    .path()
                    .extension()
                    .map(|e| e == "org")
                    .unwrap_or(false)
            {
                skips.insert(entry.path().to_path_buf());
                continue;
            }

            let mut s = String::default();
            let path = entry.path().to_path_buf();
            let mut fd = std::fs::File::open(&path)?;
            fd.read_to_string(&mut s)?;

            s.insert(0, '\n');
            s = r.replace_all(&s, |caps: &regex::Captures| {
                format!(
                    "\n{} {}",
                    caps.get(1).unwrap().as_str(),
                    caps.get(2).unwrap().as_str()
                )
            })[1..]
                .to_string();

            files.insert(path, s);
        }
    }

    println!(
        "Found {} org files and skipped {} non-org files.",
        files.len(),
        skips.len()
    );
    if io_errors > 0 {
        println!(
            "There were {} files/dirs skipped due to I/O errors (emacs lockfiles?).",
            io_errors
        );
    }

    let mut violations = 0;
    let mut count = 0;

    for doc in files.values() {
        let (a, b) = verify_structure(doc, &r);
        violations += a;
        count += b;
    }

    println!(
        "\nStructure verification: Parsed {} headlines; found {} unexplained violations\n\nBegin headline verification",
        count, violations
    );

    violations = 0;
    count = 0;

    for doc in files.values() {
        let (a, b) = verify_headline_parser(doc);
        violations += a;
        count += b;
    }

    println!(
        "\nHeadline parser verification: Parsed {} headlines; found {} unexplained violations\n\nBegin headline verification",
        count, violations
    );

    let mut words = HashSet::new();

    words.insert("* ".to_string());

    for doc in files.values() {
        for word in doc.split_ascii_whitespace() {
            words.insert(word.to_string());
        }

        for word in doc.split_whitespace() {
            words.insert(word.to_string());
        }
    }

    let characters: String = (0..std::char::MAX as u32)
        .filter_map(|i| i.try_into().ok() as Option<char>)
        .collect();

    let mut keywords = HashSet::new();
    keywords.insert("TODO");
    keywords.insert("DONE");
    keywords.insert("COMMENT");
    keywords.insert("[#A]");
    keywords.insert("[#9]");
    keywords.insert("[#B]");
    keywords.insert("[#.]");
    keywords.insert("::");
    keywords.insert(":a:");
    keywords.insert(":a:b");
    keywords.insert("a:b:");
    keywords.insert(":a:b:");
    keywords.insert(":a:b:c:");
    keywords.insert("* ");
    keywords.insert("** ");
    keywords.insert("*** ");
    keywords.insert("*");
    keywords.insert("**");
    keywords.insert("***");

    let file_count = files.len();

    let config = FuzzConfig {
        files,
        keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
        words: words.into_iter().collect(),
        characters,
    };

    let orgize_config: &FuzzConfig = Box::leak(Box::new(config.to_orgize_safe()));
    let config: &FuzzConfig = Box::leak(Box::new(config));

    for (iter, thread_count) in mutate.iter().copied() {
        let result = Arc::new(Mutex::new((0, 0)));

        println!(
            "Mutation testing {} threads each with {} iterations using {} files.",
            thread_count, iter, file_count,
        );

        let threads: Vec<_> = (0..thread_count)
            .map(|i| {
                let result = result.clone();
                println!("Spawn");
                std::thread::spawn(move || {
                    if i % 2 == 3 {
                        println!("[{}] Begin identity fuzz.", i);
                        do_identity_fuzz(i, iter, config, result.clone());
                        println!("[{}] Begin orgize fuzz.", i);
                        do_orgize_fuzz(i, iter, orgize_config, result.clone());
                    } else {
                        println!("[{}] Begin orgize fuzz.", i);
                        do_orgize_fuzz(i, iter, orgize_config, result.clone());
                        println!("[{}] Begin identity fuzz.", i);
                        do_identity_fuzz(i, iter, config, result.clone());
                    }
                    println!("[{}] End", i);
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }

        let (violations, count) = *result.lock().unwrap();

        println!(
            "Fuzz test complete with {} nodes parsed and {} violations.",
            count, violations
        );
    }

    for (iter, thread_count) in fuzz.iter().copied() {
        let result = Arc::new(Mutex::new((0, 0)));

        println!(
            "Fuzz testing {} threads each with {} iterations.",
            thread_count, iter
        );

        let threads: Vec<_> = (0..thread_count)
            .map(|i| {
                let result = result.clone();
                println!("Spawn");
                std::thread::spawn(move || {
                    if i % 2 == 3 {
                        println!("[{}] Begin identity fuzz.", i);
                        do_identity_fuzz(i, iter, &config, result.clone());
                        println!("[{}] Begin orgize fuzz.", i);
                        do_orgize_fuzz(i, iter, &orgize_config, result.clone());
                    } else {
                        println!("[{}] Begin orgize fuzz.", i);
                        do_orgize_fuzz(i, iter, &orgize_config, result.clone());
                        println!("[{}] Begin identity fuzz.", i);
                        do_identity_fuzz(i, iter, &config, result.clone());
                    }
                    println!("[{}] End", i);
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }

        let (violations, count) = *result.lock().unwrap();

        println!(
            "Fuzz test complete with {} nodes parsed and {} violations.",
            count, violations
        );
    }

    if !fuzz.is_empty() {}

    Ok(())
}

fn do_identity_fuzz(
    index: usize,
    iterations: usize,
    config: &'static FuzzConfig,
    result: Arc<Mutex<(usize, usize)>>,
) {
    let mut s = String::default();

    let (generate_send, generate_recv) = setup_generator(config);

    fn recurse(node: Section, arena: &Arena) -> (usize, usize) {
        let mut violations = 0;
        let mut count = 1;

        let level = starsector::util::lex_level(&node.text(&arena).slice(..));
        if let Some(headline) = node.headline(arena, None) {
            if headline.level() != level {
                violations += 1;
            }

            // To check for crash.
            let _ = headline.to_rope();
        } else {
            if level != 0 {
                violations += 1;
            }
        }

        for child in node.children(&arena) {
            let (a, b) = recurse(child, arena);
            violations += a;
            count += b;
        }

        (violations, count)
    }

    fn run(s: &str) -> (usize, usize) {
        let mut arena = Arena::default();
        let doc = arena.parse_str(&s);
        let out = doc.to_rope(&arena).to_string();
        if s != out {
            return (1, 1);
        }

        recurse(doc.root, &arena)
    }

    for i in 0..iterations {
        println!("Begin thread {} identity iteration {}", index, i);
        generate_send.send(s).unwrap();
        s = generate_recv.recv().unwrap();
        let (violations, count) = run(&s);

        if violations > 0 {
            let mut rng = rand::thread_rng();
            let mut fd =
                std::fs::File::create(&format!("violation.{}.org", &rng.next_u64())).unwrap();
            fd.write_all(&s.as_bytes()).unwrap();
            fd.sync_all().unwrap();
            fd.sync_data().unwrap();
        }

        let mut result = result.lock().unwrap();
        result.0 += violations;
        result.1 += count;
    }
}

impl FuzzConfig {
    fn to_orgize_safe(&self) -> FuzzConfig {
        let mut known_errors = "\u{000b}\u{0085}\u{00a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}".chars().collect::<HashSet<_>>();

        // zero width joiner. Orgize can handle it, but we delta.
        known_errors.insert('\u{200d}');

        FuzzConfig {
            files: self
                .files
                .iter()
                .map(|(k, v)| {
                    (
                        k.to_path_buf(),
                        v.chars().filter(|c| !known_errors.contains(c)).join(""),
                    )
                })
                .collect(),
            keywords: self
                .keywords
                .iter()
                .map(|s| s.chars().filter(|c| !known_errors.contains(c)).join(""))
                .collect(),
            words: self
                .words
                .iter()
                .map(|s| s.chars().filter(|c| !known_errors.contains(c)).join(""))
                .collect(),
            characters: self
                .characters
                .chars()
                .filter(|c| !known_errors.contains(c))
                .collect(),
        }
    }
}

fn do_orgize_fuzz(
    index: usize,
    iterations: usize,
    config: &'static FuzzConfig,
    result: Arc<Mutex<(usize, usize)>>,
) {
    let mut rng = rand::thread_rng();
    let mut s = String::default();

    let r = regex::Regex::new("\n(\\*+)($|\t|\n)").unwrap();

    let (generate_send, generate_recv) = setup_generator(config);

    for i in 0..iterations {
        println!("Begin thread {} orgize delta iteration {}", index, i);

        let mut violations = 0;
        let mut count = 0;

        generate_send.send(s).unwrap();
        s = generate_recv.recv().unwrap();

        if !s.is_empty() {
            s.insert(0, '\n');
            s = r.replace_all(&s, |caps: &regex::Captures| {
                format!(
                    "\n{} {}",
                    caps.get(1).unwrap().as_str(),
                    caps.get(2).unwrap().as_str()
                )
            })[1..]
                .to_string();
        }

        let (a, b) = verify_structure(&s, &r);
        violations += a;
        count += b;

        let (a, b) = verify_headline_parser(&s);
        violations += a;
        count += b;

        let mut result = result.lock().unwrap();
        result.0 += violations;
        result.1 += count;

        if violations > 0 {
            let mut fd =
                std::fs::File::create(&format!("violation.{}.org", &rng.next_u64())).unwrap();
            fd.write_all(&s.as_bytes()).unwrap();
        }
    }
}

fn setup_generator(config: &'static FuzzConfig) -> (Sender<String>, Receiver<String>) {
    let (a, b) = std::sync::mpsc::channel();
    let (c, d) = std::sync::mpsc::channel();
    let _ = std::thread::spawn(move || {
        generator(config, a, d);
    });
    (c, b)
}

fn generator(config: &'static FuzzConfig, sender: Sender<String>, receiver: Receiver<String>) {
    let mut pending = false;
    loop {
        let (a, b) = std::sync::mpsc::channel();
        let (c, d) = std::sync::mpsc::channel();
        let t = std::thread::spawn(move || loop {
            a.send(_generate(config, d.recv().unwrap())).unwrap();
        });

        loop {
            if pending {
                pending = false;
                sender.send(String::default()).unwrap();
            }
            let s = receiver.recv().unwrap();
            if !c.send(s).is_err() {
                if let Ok(s) = b.recv() {
                    sender.send(s).unwrap();
                    continue;
                }
            }
            break;
        }

        println!(
            "Generate panic. You should probably fix it instead of this hack, but it's so rare..."
        );
        let _ = t.join();
        pending = true;
    }
}

// Has a rare index error where it will compute things wrong and slice into
// Unicode. Hard to reproduce and fix + very rare = just hack around it with
// threads.
fn _generate(config: &FuzzConfig, mut s: String) -> String {
    let mut rng = rand::thread_rng();

    let mut chunks: HashSet<&str> = HashSet::default();

    {
        if !config.files.is_empty() {
            let mut source = config
                .files
                .values()
                .nth(rng.gen_range(0, config.files.len()))
                .unwrap()
                .as_str();
            let mut count = source.char_indices().count();
            while !source.is_empty() {
                let pos = (rng.next_u32() as usize) % count + 1;

                let updated = if pos == count {
                    (source, "")
                } else {
                    let mut i = source.char_indices();
                    let i = i.nth(pos);
                    let (i, c) = i.as_ref().unwrap();
                    (&source[..*i], &source[i + c.len_utf8() - 1..])
                };
                assert_eq!(updated.0.chars().count(), pos);
                assert_eq!(
                    source.chars().count(),
                    updated.0.chars().count() + updated.1.chars().count()
                );

                chunks.insert(updated.0);
                count -= pos;
                assert_eq!(updated.1.chars().count(), count);
                source = updated.1;
            }
        }
    }

    if !chunks.is_empty() {
        s.clear();
        for (_, chunk) in chunks.iter().enumerate() {
            for _ in 0..10 {
                let choice = rng.gen_range(0, 50);
                if choice <= 5 {
                    append_random_string(config, &mut s, rng.next_u32() as usize % 75);
                } else if choice <= 10 {
                    append_random_string(config, &mut s, rng.next_u32() as usize % 75);
                    break;
                } else if choice <= 15 {
                    let goal = s.chars().count() + chunk.chars().count();
                    while s.chars().count() < goal {
                        append_random_string(config, &mut s, chunk.len());
                    }
                    while s.chars().count() > goal {
                        s.pop();
                    }
                    while s.chars().count() < goal {
                        s.push('x');
                    }
                    break;
                } else if choice <= 17 {
                    break;
                } else if choice <= 22 {
                    s.extend(chunk.chars().rev());
                    break;
                } else if choice <= 25 {
                    s += "* Hello\n";
                    s += &chunk;
                    s += "* World\n";
                    break;
                } else if choice <= 30 {
                    for word in chunk.split_ascii_whitespace() {
                        s += word;
                        s.push(' ');
                    }
                    s.pop();
                } else if choice <= 32 {
                    for word in chunk.split_whitespace() {
                        s += word;
                        s.push(' ');
                    }
                    s.pop();
                } else if choice <= 33 {
                    for line in chunk.lines() {
                        s += line;
                        s.push('\n');
                    }
                } else if choice <= 35 {
                    s += &chunk.replace('\n', "\t");
                    break;
                } else if choice <= 36 {
                    s += &chunk.replace('\n', "\r\n");
                    break;
                } else if choice <= 37 {
                    s += &chunk.replace('\n', "\r");
                    break;
                } else if choice < 40 {
                    let mut thing = gen_planning();
                    if rng.gen_range(0, 10) == 2 {
                        thing = format!("Hello\n{}", &thing);
                    }
                    appendify(&mut s, &thing);
                } else if choice < 43 {
                    let mut thing = gen_properties();
                    if rng.gen_range(0, 10) == 2 {
                        thing = format!("Hello\n{}", &thing);
                    }
                    appendify(&mut s, &thing);
                } else if choice < 46 {
                    let r = rng.gen_range(0, 13);
                    let stuff = if r == 1 {
                        format!("{}{}", gen_planning(), gen_properties())
                    } else if r == 2 {
                        format!("{}\n{}", gen_properties(), gen_planning())
                    } else if r == 3 {
                        format!("Hello!\n{}\n{}", gen_planning(), gen_properties())
                    } else if r == 4 {
                        format!("Hello!\nWorld!\n{}\n{}", gen_planning(), gen_properties())
                    } else {
                        format!("\n{}\n{}", gen_planning(), gen_properties())
                    };

                    appendify(&mut s, &stuff);
                } else {
                    s += &chunk;
                    break;
                }
            }
        }
        return s;
    }

    let mut length = rng.next_u32() % 32;

    if rng.next_u32() % 3 == 1 {
        if rng.next_u32() % 7 == 3 {
            length += 1024 * 32;
        }

        if rng.next_u32() % 7 == 3 {
            length += 1024 * 1024;
        }
    }

    s.clear();

    append_random_string(config, &mut s, length as usize);

    s.insert(0, '\n');
    s
}

fn appendify(s: &mut String, thing: &str) {
    let mut rng = rand::thread_rng();
    let r = rng.gen_range(0, 10);
    if r == 1 {
        *s += thing;
    } else if r == 2 {
        *s += thing;
        s.push('\n');
    } else if r == 3 {
        s.push('\n');
        *s += thing;
    } else {
        *s += "\n* Hello\n";
        *s += thing;
        s.push('\n');
    }
}

fn gen_properties() -> String {
    let mut rng = rand::thread_rng();
    let mut properties = "  :PROPERTIES:\n".to_string();

    for _ in 0..rng.gen_range(0, 10) {
        let mut name: String = rng
            .sample_iter::<char, _>(rand::distributions::Standard)
            .take(rng.gen_range(1, 5))
            .filter(|c| *c != '+' && !c.is_whitespace())
            .collect();
        if name.is_empty() {
            continue;
        }
        if rng.gen_range(0, 5) == 2 {
            name.push('+');
        }

        let value: String = rng
            .sample_iter::<char, _>(rand::distributions::Standard)
            .take(rng.gen_range(0, 50))
            .filter(|c| *c != '\n')
            .collect();

        properties += &format!("   {}: {}\n", &name, &value);
    }

    properties += ":END:";

    properties
}

fn gen_planning() -> String {
    let mut rng = rand::thread_rng();
    let mut planning = String::new();
    for _ in 0..rng.gen_range(0, 5) {
        let r = rng.gen_range(0, 3);
        if r == 0 {
            planning += "DEADLINE";
        } else if r == 1 {
            planning += "SCHEDULED";
        } else {
            planning += "CLOSED";
        }
        if rng.gen_range(0, 20) == 7 {
            planning += "FROBBLE";
        }
        if rng.gen_range(0, 7) != 1 {
            planning += ": ";
        }
        if rng.gen_range(0, 7) != 1 {
            let r = rng.gen_range(0, 5);
            if r == 0 {
                planning += "<2019-02-04>";
            } else if r == 1 {
                planning += "[2019-02-04]";
            } else if r == 2 {
                planning += "<2019-02-04 .+1w>";
            } else if r == 3 {
                planning += "<2019-02-04 Sun ++1w>";
            } else if r == 4 {
                planning += "<2019-02-04 Sun -1w>";
            }
        }
        if rng.gen_range(0, 10) != 1 {
            planning += " ";
        }
    }
    planning
}

fn append_random_string(config: &FuzzConfig, s: &mut String, length: usize) {
    let mut rng = rand::thread_rng();

    let goal = s.len() + length;
    while s.len() < goal {
        let r = rng.next_u32() % 12;
        if r % 5 == 0 {
            let mut count = rng.next_u32() % 10 + 1;
            if count > 5 {
                count = 1;
            }
            for _ in 0..count {
                let r = rng.next_u32() % 6;
                if r == 0 {
                    s.push('\t');
                } else if r < 3 {
                    s.push(' ');
                } else {
                    s.push('\n');
                }
            }
        } else if r == 1 {
            s.push('\n');
            for _ in 1..(rng.next_u32() % 10) {
                s.push('*');
            }
            s.push('*');
        } else if r == 2 {
            let mut count = rng.next_u32() % 30;
            if count > 20 {
                count = 0;
            }
            for _ in 0..count {
                *s += &config.keywords[(rng.next_u32() as usize) % config.keywords.len()];
                s.push(' ');
            }
            *s += &config.keywords[(rng.next_u32() as usize) % config.keywords.len()];
        } else if r == 3 {
            let mut count = rng.next_u32() % 30;
            if count > 20 {
                count = 0;
            }
            for _ in 0..count {
                *s += &config.words[(rng.next_u32() as usize) % config.words.len()];
                s.push(' ');
            }
            *s += &config.words[(rng.next_u32() as usize) % config.words.len()];
        } else if r == 4 {
            let mut word_count = rng.next_u32() % 25 + 1;
            if word_count > 20 {
                word_count = 1;
            }
            for _ in 0..word_count {
                let word_length = rng.next_u32() % 10 + 1;
                for _ in 0..word_length {
                    s.push(
                        config
                            .characters
                            .chars()
                            .nth((rng.next_u32() as usize) % config.characters.chars().count())
                            .unwrap(),
                    );
                }
                if rng.next_u32() % 4 == 0 {
                    s.push('\t');
                } else {
                    s.push(' ');
                }
            }
        } else if r == 5 || r == 6 {
            let mut count = rng.next_u32() % 30;
            if count > 20 {
                count = 0;
            }
            for _ in 0..count {
                let corpus = if !config.words.is_empty() && rng.next_u32() % 4 == 0 {
                    &config.words
                } else {
                    &config.keywords
                };
                *s += &corpus[(rng.next_u32() as usize) % corpus.len()];
                s.push(' ');
            }
        } else {
            s.push('\n');
        }
    }
}

#[derive(Clone)]
struct FuzzConfig {
    files: HashMap<PathBuf, String>,
    characters: String,
    words: Vec<String>,
    keywords: Vec<String>,
}

fn parse_compare_node<'a>(ours: &Section, arena: &Arena) -> (usize, usize) {
    let mut count = 1;
    let mut violations = 0;

    let text = ours.text(&arena);
    let headline_text = text.lines().next().map(Rope::from).unwrap_or_default();
    let org = orgize::Org::parse_string(text.to_string());

    // Orgize defines headlines oddly.
    let mut orgize_defined_bonus_headlines = 0;

    let text = text.to_contiguous();
    let lines = if text.ends_with('\n') {
        &text[..text.len() - 1]
    } else {
        &text
    }
    .split('\n');

    for line in lines.skip(1) {
        let mut level = 0;
        for c in line.chars() {
            if c == '*' {
                level += 1;
            } else if c == '\n' {
                break;
            } else {
                level = 0;
                break;
            }
            assert!(c != ' ');
        }
        if level > 0 {
            orgize_defined_bonus_headlines += 1;
        }
    }

    let other = if org.headlines().count() == 0 {
        println!(
            "Headline {:?} Orgize did not parse a headline.",
            headline_text,
        );
        return (1, 1);
    } else if org.headlines().count() == orgize_defined_bonus_headlines + 1 {
        org.headlines().next().unwrap()
    } else {
        println!(
            "Headline {:?} Orgize parsed {} headlines when {} were expected:",
            text,
            org.headlines().count(),
            orgize_defined_bonus_headlines + 1,
        );
        return (1, 1);
    };

    match ours.headline(arena, None) {
        Some(parsed) => {
            violations += parse_compare_headline(headline_text, &parsed, other.title(&org));
        }
        None => {
            println!(
                "Headline {:?} Starsector unable to parse headline.",
                headline_text,
            );
        }
    }

    for child in ours.children(&arena) {
        let (a, b) = parse_compare_node(&child, &arena);
        violations += a;
        count += b;
    }

    (violations, count)
}

fn parse_compare_headline<'a>(
    headline_text: Rope,
    ours: &Headline,
    other: &orgize::elements::Title<'a>,
) -> usize {
    let headline_text = headline_text.to_contiguous();

    if ours.level() as usize != other.level {
        println!("Headline {:?} level mismatch", headline_text,);
        return 1;
    }

    let same_keyword = match (ours.keyword(), other.keyword.as_ref()) {
        (Some(a), Some(b)) if a == b => true,
        (None, None) => true,
        _ => false,
    };

    if !same_keyword {
        if let Some(k) = other.keyword.as_ref() {
            if let Some(index) = headline_text.find(&**k) {
                for c in headline_text[..index].chars().rev() {
                    if !c.is_ascii_whitespace() {
                        break;
                    }
                    if c != ' ' {
                        // I am no longer interested in this case.
                        return 0;
                    }
                }
            }
            if let Some(index) = headline_text.find(&**k) {
                for c in headline_text[index + k.len()..].chars() {
                    if !c.is_ascii_whitespace() {
                        break;
                    }
                    if c != ' ' {
                        // I am no longer interested in this case.
                        return 0;
                    }
                }
            }
        }
        println!(
            "Headline {:?} keyword mismatch. orgize {:?} vs starsector {:?}",
            headline_text,
            &other.keyword,
            ours.keyword()
        );
        return 1;
    }

    if ours.priority() != other.priority {
        if let Some(k) = ours.priority() {
            let k = format!("[#{}]", k);
            if let Some(index) = headline_text.find(&k) {
                for c in headline_text[..index].chars().rev() {
                    if !c.is_ascii_whitespace() {
                        break;
                    }
                    if c != ' ' {
                        // I am no longer interested in this case.
                        return 0;
                    }
                }
            }
            if let Some(index) = headline_text.find(&k) {
                for c in headline_text[index + k.len()..].chars() {
                    if !c.is_ascii_whitespace() {
                        break;
                    }
                    if c != ' ' {
                        // I am no longer interested in this case.
                        return 0;
                    }
                }
            }
        }
        println!(
            "Headline {:?} priority mismatch. Orgize gets {:?} we are matching at {:?}",
            headline_text, other, ours
        );
        return 1;
    }

    let mut other_title = other.raw.to_string();

    if ours.commented() != other.is_commented() {
        println!(
            "Headline {:?} commented mismatch orgize {} starsector {}",
            &headline_text,
            other.is_commented(),
            ours.commented()
        );
        return 1;
    }

    if other.is_commented() && other_title.starts_with("COMMENT") {
        other_title = other_title[7..].trim_start().to_string();
    };

    // To reconcile different tag semantics vs Orgize and org-element.
    let mut other_title: &str = other_title.as_str();
    if ours.raw_tags().is_empty() {
        strip_trailing_empty_tags(&mut other_title);
    }
    let tags_match = ours.tags().eq(other.tags.iter());
    if ours.title() == other_title && tags_match {
        // pass
    } else if ours.title().is_empty()
        && !ours.raw_tags().is_empty()
        && other.tags.is_empty()
        && other_title == format!(":{}:", &ours.raw_tags())
    {
        // Orgize considers not parsing tags when the title is empty WAI. This is
        // also the behavior org-element has. Starsector will parse the tags, as
        // will org-mode itself.
        // https://github.com/PoiScript/orgize/issues/17
    } else if !tags_match {
        println!(
            "Headline {:?} parsed tags mismatch. orgize: {:?} vs starsector {:?}",
            &headline_text,
            other.tags,
            ours.tags().collect::<Vec<_>>()
        );
    } else if other_title.trim() != ours.title().to_contiguous().trim() {
        println!(
            "Headline {:?} title mismatch. orgize: {:?} vs starsector: {:?}",
            &headline_text,
            other_title,
            ours.title()
        );
        return 1;
    }

    return 0;
}

fn strip_trailing_empty_tags(text: &mut &str) {
    // org-element and org-mode don't respect Unicode whitespace here.
    if let Some(final_word) = text.split_ascii_whitespace().last() {
        if final_word.len() >= 2 {
            // E.g, "* Hello ::"
            // org-element and Orgize treat this as part of the title.
            let mut found = false;
            for c in final_word.chars() {
                if c != ':' {
                    found = true;
                    break;
                }
            }
            if !found {
                *text = &text[..text.len() - final_word.len()].trim_end();
            }
        }
    }
}

fn structure_compare_headline<'a>(
    level: usize,
    headline: &str,
    other: &orgize::elements::Title<'a>,
) -> usize {
    let mut text = &headline[level + 1..];
    text = text.trim();

    if let Some(keyword) = other.keyword.as_ref() {
        // org-element and org-mode don't respect Unicode whitespace here.
        if text.starts_with(&**keyword) && text.len() == keyword.len()
            || text.len() > keyword.len()
                && text[keyword.len()..]
                    .chars()
                    .next()
                    .unwrap()
                    .is_ascii_whitespace()
        {
            text = &text[keyword.len()..].trim();
        } else {
            println!("Headline {:?} keyword mismatch", headline,);
            return 1;
        }
    }

    // Org mode spec does not require a space after priority. Org mode itself
    // will accept the cookie anywhere, including in the tags (though then the
    // tags won't parse, since they contain invalid characters). I don't care
    // enough to try to be quite as generous as org-mode itself, but I can at
    // least follow the spec by not requiring a space.
    if let Some(priority) = other.priority {
        if text.starts_with(&format!("[#{}]", priority)) {
            // I am no longer interested in this case.
            if let Some(c) = text.chars().nth(4) {
                if !c.is_ascii_whitespace() {
                    return 0;
                }
            }
        } else {
            println!(
                "Headline {:?} priority mismatch. Orgize gets {:?} we are matching at {:?}",
                headline,
                other.priority,
                &text[..4]
            );
            return 1;
        }
    }

    if !text.contains(&*other.raw) {
        println!("Headline {:?} generic mismatch", headline,);
        return 1;
    }

    return 0;
}

// FIXME This only compares bodys, and skips sections (including
// the root).
fn structure_compare_node(
    ours: &Section,
    theirs: &orgize::Headline,
    org: &orgize::Org,
    arena: &Arena,
) -> (usize, usize) {
    let mut violations = 0;
    let mut count = 1;

    let level = ours.level(&arena) as usize;
    let text = ours.text(&arena);
    if level != theirs.level() {
        println!(
            "Headline {:?} level differs: {} vs {}. Not recursing into it.",
            text,
            level,
            theirs.level()
        );
        return (1, 1);
    }

    let our_cc = ours.children(&arena).count();
    let their_cc = theirs.children(&org).count();
    if our_cc != their_cc {
        // Orgize defines headlines oddly.
        let lines = &*text.to_contiguous();
        let lines = if lines.ends_with('\n') {
            &lines[..lines.len() - 1]
        } else {
            lines
        }
        .split('\n');

        for line in lines.skip(1) {
            let mut level = 0;
            for c in line.chars() {
                if c == '*' {
                    level += 1;
                } else if c.is_whitespace() {
                    break;
                } else {
                    level = 0;
                    break;
                }
                assert!(c != ' ');
            }
            if level > 0 {
                // Orgize will treat this as a headline erroneously.
                return (violations, count);
            }
        }

        println!(
            "Headline {:?} child count differs: {} vs {}. Not recursing into it.",
            text, our_cc, their_cc
        );
        return (1, 1);
    }

    let other = theirs.title(&org);

    // FIXME: Compare root text
    if level > 0 {
        violations += structure_compare_headline(
            level,
            &*text
                .lines()
                .next()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            other,
        );
    }

    for (ours, theirs) in ours.children(&arena).zip(theirs.children(&org)) {
        let (a, b) = structure_compare_node(&ours, &theirs, &org, &arena);
        violations += a;
        count += b;
    }

    return (violations, count);
}

fn verify_structure(text: &str, r: &regex::Regex) -> (usize, usize) {
    // Orgize will treat these as headlines.
    let mut text = text.to_string();
    text.insert(0, '\n');
    let text = &r.replace_all(&text, |caps: &regex::Captures| {
        format!(
            "\n{} {}",
            caps.get(1).unwrap().as_str(),
            caps.get(2).unwrap().as_str()
        )
    })[1..];

    // Verify tree structure.
    let mut arena = Arena::default();
    let doc = arena.parse_str(&text);

    let org = orgize::Org::parse(&text);
    if !org.validate().is_empty() {
        let mut rng = rand::thread_rng();
        let mut fd = std::fs::File::create(&format!("violation.{}.org", &rng.next_u64())).unwrap();
        fd.write_all(&text.as_bytes()).unwrap();
        fd.sync_all().unwrap();
        fd.sync_data().unwrap();
        return (1, 1);
    }

    let mut violations = 0;
    let mut count = 0;

    for (ours, theirs) in doc.root.children(&arena).zip(org.document().children(&org)) {
        let (a, b) = structure_compare_node(&ours, &theirs, &org, &arena);
        violations += a;
        count += b;
    }

    (violations, count)
}

fn verify_headline_parser(text: &str) -> (usize, usize) {
    // This time, we do each Orgize parse individually.

    // Verify tree structure.
    let mut arena = Arena::default();
    let doc = arena.parse_str(&text);

    let mut violations = 0;
    let mut count = 0;

    for ours in doc.root.children(&arena) {
        let (a, b) = parse_compare_node(&ours, &arena);
        violations += a;
        count += b;
    }

    (violations, count)
}
