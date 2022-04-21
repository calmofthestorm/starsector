use crate::{Arena, Document, RopeSliceExt, Section, SectionData};

use ropey::{Rope, RopeSlice};

pub fn lex_level(line: &RopeSlice) -> u16 {
    headline_level(line, 0)
}

pub fn lex_level_str(line: &str) -> u16 {
    headline_level_str(line, 0)
}

// Returns the level of the headline at offset, or 0 if no headline. This will
// always be exactly one line, including the terminal \n if one is present. Must
// only be called at the start of a line.
//
// For compatibility with org-mode itself, a headline is defined by one or more
// '*' followed by an ASCII space, ' '.
pub fn headline_level(input: &RopeSlice, offset: usize) -> u16 {
    for (i, c) in input.bytes_at(offset).enumerate() {
        match c {
            b'*' => {}
            b' ' if i > 0 => return i as u16,
            _ => return 0,
        }
    }
    0
}

pub fn headline_level_str(input: &str, offset: usize) -> u16 {
    for (i, c) in input[offset..].as_bytes().iter().enumerate() {
        match c {
            b'*' => {}
            b' ' if i > 0 => return i as u16,
            _ => return 0,
        }
    }
    0
}

// Returns a pair of the current line (including terminal \n if present) and the
// rest of the string.
pub fn line<'a>(input: &'a RopeSlice<'a>) -> (RopeSlice<'a>, RopeSlice<'a>) {
    let split = next_line(input, 0);
    (input.slice_bytes(..split), input.slice_bytes(split..))
}

// Returns either the start of the next line, or input.len() if none.
pub fn next_line(input: &RopeSlice, offset: usize) -> usize {
    input.memchr(b'\n', offset)
}

pub(crate) fn parse_document(arena: &mut Arena, input: &RopeSlice) -> Document {
    let mut offset = 0;

    // Special case empty document.
    if input.is_empty() {
        let root_id = arena.arena.new_node(SectionData {
            level: 0,
            text: Rope::default(),
        });

        return Document {
            root: Section { id: root_id },
            terminal_newline: false,
            empty_root_section: true,
        };
    }

    let (new_offset, end) = parse_section(input, offset);
    let empty_root_section = new_offset == end && offset == end;
    let root_id = arena.arena.new_node(SectionData {
        level: 0,
        text: Rope::from(input.slice_bytes(offset..end)),
    });
    offset = new_offset;

    let mut stack = vec![root_id];

    let mut level = headline_level(input, offset);
    while level > 0 {
        let (new_offset, end) = parse_section(input, next_line(input, offset));
        let section = SectionData {
            text: Rope::from(input.slice_bytes(offset..end)),
            level,
        };
        offset = new_offset;

        while level
            <= arena.arena[*stack.last().expect("stack never empty")]
                .get()
                .level
        {
            stack.pop().expect("stack never empty");
        }

        let node_id = arena.arena.new_node(section);

        stack
            .last()
            .expect("stack never empty")
            .append(node_id, &mut arena.arena);

        stack.push(node_id);

        level = headline_level(input, offset);
    }

    assert_eq!(input.len_bytes(), offset);

    // #[cfg(debug_assertions)]
    let re = regex::Regex::new("(^|.*\n)\\*\\** .*").expect("failed to assemble headline regex");

    // #[cfg(debug_assertions)]
    fn checker(re: &regex::Regex, node: Section, arena: &Arena, input: &RopeSlice) {
        let level = node.level(&arena);
        let text = node.text(&arena);
        let lexed_level = lex_level(&text.slice(..));
        if lexed_level != level
            || text.len_bytes() >= level as usize
                && re.is_match(&text.to_contiguous()[(level as usize)..])
        {
            // use std::io::Write;
            // let mut ff = std::fs::File::create("/tmp/lll.org").unwrap();
            // ff.write_all(input).unwrap();
            // panic!("Error written to /tmp/lll.org");
            panic!("Check failed");
        }
        assert_eq!(lexed_level, level);
        for node in node.children(&arena) {
            checker(re, node, arena, input);
        }
    }

    // #[cfg(debug_assertions)]
    checker(&re, Section { id: root_id }, &arena, input);

    Document {
        root: Section { id: root_id },
        terminal_newline: input.bytes().last() == Some(b'\n'),
        empty_root_section,
    }
}

// Returns either the start of the next headline or input.len(), whichever comes
// first. Must start at the start of the line. Will return nothing if started at
// a headline.
fn parse_section(input: &RopeSlice, offset: usize) -> (usize, usize) {
    // Collect lines until EOF or a headline.
    let mut last = offset;
    while last < input.len_bytes() {
        let i = input.memchr(b'\n', last);
        // Fastpath: skip lines that don't start with *.
        if i >= input.len_bytes() || input.byte(last) == b'*' && headline_level(input, last) != 0 {
            break;
        }
        last = i + 1;
    }

    let last = if last < input.len_bytes() && headline_level(input, last) == 0 {
        input.len_bytes()
    } else {
        last
    };
    if last > offset && last <= input.len_bytes() && input.byte(last - 1) == b'\n' {
        (last, last - 1)
    } else {
        (last, last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn next_line(s: &[u8], offset: usize) -> usize {
        let r = Rope::from(std::str::from_utf8(s).unwrap());
        let r = r.slice(..);
        crate::parser::structure::next_line(&r, offset)
    }

    fn parse_section(s: &[u8], offset: usize) -> (usize, usize) {
        let r = Rope::from(std::str::from_utf8(s).unwrap());
        let r = r.slice(..);
        crate::parser::structure::parse_section(&r, offset)
    }

    fn headline_level(s: &[u8], offset: usize) -> u16 {
        let r = Rope::from(std::str::from_utf8(s).unwrap());
        let r = r.slice(..);
        crate::parser::structure::headline_level(&r, offset)
    }

    #[test]
    fn test_next_line() {
        assert_eq!(0, next_line(b"", 0));
        assert_eq!(1, next_line(b" ", 0));
        assert_eq!(1, next_line(b" ", 1));
        assert_eq!(0, next_line(b"\n", 0));
        assert_eq!(1, next_line(b"\n", 1));
        assert_eq!(1, next_line(b" \n", 0));
        assert_eq!(1, next_line(b" \n", 1));
        assert_eq!(0, next_line(b"\n ", 0));
        assert_eq!(2, next_line(b"\n ", 1));
        assert_eq!(0, next_line(b"\ntest\n", 0));
        assert_eq!(5, next_line(b"\ntest\n", 1));
        assert_eq!(0, next_line(b"\n\na\n", 0));
        assert_eq!(1, next_line(b"\n\na\n", 1));
        assert_eq!(3, next_line(b"\n\na\n", 2));
        assert_eq!(3, next_line(b"\n\na\n", 3));
    }

    #[test]
    fn test_parse_section() {
        assert_eq!((0, 0), parse_section(b"", 0));
        assert_eq!((1, 1), parse_section(b"*", 0));
        assert_eq!((1, 1), parse_section(b"*", 1));
        assert_eq!((0, 0), parse_section(b"* ", 0));
        assert_eq!((2, 2), parse_section(b"* ", 1));
        assert_eq!((2, 2), parse_section(b"* ", 2));
        assert_eq!((1, 0), parse_section(b"\n", 0));
        assert_eq!((1, 1), parse_section(b"\n", 1));
        assert_eq!((0, 0), parse_section(b"* \n", 0));
        assert_eq!((3, 2), parse_section(b"* \n", 1));
        assert_eq!((3, 2), parse_section(b"* \n", 2));
        assert_eq!((1, 0), parse_section(b"\n*** \n", 0));
        assert_eq!((1, 1), parse_section(b"\n*** \n", 1));
        assert_eq!((2, 2), parse_section(b"\n*** \n", 2));
        assert_eq!((3, 3), parse_section(b"\n*** \n", 3));
        assert_eq!((6, 5), parse_section(b"\n*** \n", 4));
        assert_eq!((3, 2), parse_section(b"Hi\n*** \n", 0));
    }

    #[test]
    fn test_headline_level() {
        assert_eq!(0, headline_level(b"", 0));

        assert_eq!(0, headline_level(b" ", 0));
        assert_eq!(0, headline_level(b"*", 0));
        assert_eq!(0, headline_level(b"a", 0));

        assert_eq!(0, headline_level(b"  ", 0));
        assert_eq!(1, headline_level(b"* ", 0));
        assert_eq!(0, headline_level(b"a ", 0));
        assert_eq!(0, headline_level(b" *", 0));
        assert_eq!(0, headline_level(b"**", 0));
        assert_eq!(0, headline_level(b"a*", 0));
        assert_eq!(0, headline_level(b" a", 0));
        assert_eq!(0, headline_level(b"*a", 0));
        assert_eq!(0, headline_level(b"aa", 0));

        assert_eq!(0, headline_level(b"   ", 0));
        assert_eq!(1, headline_level(b"*  ", 0));
        assert_eq!(0, headline_level(b"a  ", 0));
        assert_eq!(0, headline_level(b" * ", 0));
        assert_eq!(2, headline_level(b"** ", 0));
        assert_eq!(0, headline_level(b"a* ", 0));
        assert_eq!(0, headline_level(b" a ", 0));
        assert_eq!(0, headline_level(b"*a ", 0));
        assert_eq!(0, headline_level(b"aa ", 0));

        assert_eq!(0, headline_level(b"  *", 0));
        assert_eq!(1, headline_level(b"* *", 0));
        assert_eq!(0, headline_level(b"a *", 0));
        assert_eq!(0, headline_level(b" **", 0));
        assert_eq!(0, headline_level(b"***", 0));
        assert_eq!(0, headline_level(b"a**", 0));
        assert_eq!(0, headline_level(b" a*", 0));
        assert_eq!(0, headline_level(b"*a*", 0));
        assert_eq!(0, headline_level(b"aa*", 0));

        assert_eq!(0, headline_level(b"  a", 0));
        assert_eq!(1, headline_level(b"* a", 0));
        assert_eq!(0, headline_level(b"a a", 0));
        assert_eq!(0, headline_level(b" *a", 0));
        assert_eq!(0, headline_level(b"**a", 0));
        assert_eq!(0, headline_level(b"a*a", 0));
        assert_eq!(0, headline_level(b" aa", 0));
        assert_eq!(0, headline_level(b"*aa", 0));
        assert_eq!(0, headline_level(b"aaa", 0));

        assert_eq!(0, headline_level(b"***", 0));
        assert_eq!(3, headline_level(b"*** ", 0));
        assert_eq!(3, headline_level(b"***  ", 0));
        assert_eq!(0, headline_level(b"***a", 0));
        assert_eq!(3, headline_level(b"*** a", 0));
        assert_eq!(3, headline_level(b"*** aaaaa", 0));
    }
}
