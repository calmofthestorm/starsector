use nom::{
    bytes::complete::{tag, take_till, take_while},
    character::complete::{char, one_of, space0},
    combinator::verify,
    error::{make_error, ErrorKind},
    sequence::{delimited, preceded},
    Err, IResult,
};
use ropey::{Rope, RopeSlice};

use crate::{Context, Headline, HeadlinePod, RopeSliceExt};

lazy_static! {
    static ref DEFAULT_CONTEXT: Context<'static> = Context::default();
}

// Matches a headline's stars, consisting of LEVEL stars followed by one ASCII
// space ' '. Don't match the space.
fn parse_level(input: &str) -> IResult<&str, u16, ()> {
    match crate::util::lex_level_str(input) {
        0 => Err(Err::Error(make_error(input, ErrorKind::Tag))),
        level => Ok((&input[level as usize..], level)),
    }
}

fn parse_keyword<'a>(input: &'a str, context: &'_ Context) -> IResult<&'a str, &'a str, ()> {
    verify(
        preceded(
            // org-mode does not allow Unicode whitespace before the keyword. If
            // any is present, the keyword will not be recognized. Tabs are ok.
            take_while(|c| c == ' '),
            take_till(|c: char| c.is_whitespace()),
        ),
        |keyword: &str| context.keywords.split(':').any(|k| k == keyword),
    )(input)
}

fn parse_priority(input: &str) -> IResult<&str, char, ()> {
    // Priorities may be preceded by any whitespace, or none at all. Actually,
    // org-mode will recognize a priority anywhere in the title, even in the
    // middle of a word somewhere, but we choose to not go quite that far.
    preceded(
        space0,
        delimited(tag("[#"), one_of("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), char(']')),
    )(input)
}

fn parse_tags(input: &str) -> IResult<&str, &str, ()> {
    let tail_space = input.len();
    let maybe_tags = input.trim_end_matches(|c: char| c.is_ascii_whitespace());
    let tail_space = tail_space - maybe_tags.len();

    // I verified that org-element and org-mode don't respect Unicode whitespace
    // here. This includes trimming unicode whitespace after the end of the tags.
    let maybe_tags = maybe_tags
        .split_ascii_whitespace()
        .last()
        .unwrap_or_default();
    let length = maybe_tags.len();

    if length < 2 || maybe_tags.as_bytes()[0] != b':' || maybe_tags.as_bytes()[length - 1] != b':' {
        return Ok((input, ""));
    }

    let maybe_tags = &maybe_tags[1..length - 1];
    for c in maybe_tags.chars() {
        if c != ':' && c != '#' && c != '@' && c != '%' && c != '_' && !c.is_alphanumeric() {
            return Ok((input, ""));
        }
    }

    Ok((
        &input[..input.len() - maybe_tags.len() - 2 - tail_space],
        maybe_tags,
    ))
}

// Parse the title line of a headline starting at text. Does not parse planning,
// properties, the body, or child headlines -- just the title line.
pub(crate) fn parse_headline(input: RopeSlice, context: &Context) -> Option<Headline> {
    let (headline_rope, body) = crate::parser::structure::line(&input);
    let body = match body.get_char(0) {
        Some('\n') => body.slice(1..),
        _ => body,
    };

    // FIXME: Nom does support streaming, so at least in theory it's possible to
    // parse ropes directly.
    let headline = headline_rope.to_contiguous();
    let headline_contiguous = &*headline;
    let (headline, level) = parse_level(headline_contiguous).ok()?;

    let (headline, keyword) = match parse_keyword(headline, context) {
        Ok((headline, keyword)) => (
            headline,
            Some(Rope::from(
                headline_rope.discontangle(headline_contiguous, keyword),
            )),
        ),
        Err(..) => (headline, None),
    };

    let (headline, priority) = match parse_priority(headline) {
        Ok((headline, priority)) => (headline, Some(priority)),
        Err(..) => (headline, None),
    };

    let (title, raw_tags_rope, raw_tags_string) = match parse_tags(headline) {
        Ok((headline, tags)) => (
            headline.trim(),
            Rope::from(headline_rope.discontangle(headline_contiguous, tags)),
            tags.to_string(),
        ),
        Err(..) => (headline.trim(), Rope::default(), String::default()),
    };

    let (commented, title) =
        if title.starts_with("COMMENT") && title.chars().nth(7).unwrap_or(' ').is_whitespace() {
            (true, title[7..].trim_start())
        } else {
            (false, title)
        };

    Some(Headline(HeadlinePod {
        level,
        commented,
        keyword,
        priority,
        title: title.into(),
        raw_tags_string,
        raw_tags_rope,
        body: body.into(),
    }))
}
