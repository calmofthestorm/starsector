use nom::{
    bytes::complete::{tag, take_till, take_while},
    character::complete::{char, one_of, space0},
    combinator::verify,
    error::{make_error, ErrorKind},
    multi::many0,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanningKeyword {
    Deadline,
    Scheduled,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPattern {
    pub keyword: PlanningKeyword,
    // ELDRITCH parse timestamp
    pub timestamp: String,
}

#[derive(Debug, Clone, Default)]
pub struct Planning {
    pub deadline: Option<String>,
    pub scheduled: Option<String>,
    pub closed: Option<String>,
}

fn parse_planning_keyword(input: &str) -> IResult<&str, PlanningKeyword, ()> {
    if input.starts_with("DEADLINE:") {
        Ok((&input[8..], PlanningKeyword::Deadline))
    } else if input.starts_with("SCHEDULED:") {
        Ok((&input[9..], PlanningKeyword::Scheduled))
    } else if input.starts_with("CLOSED:") {
        Ok((&input[6..], PlanningKeyword::Closed))
    } else {
        return Err(nom::Err::Error(()));
    }
}

fn parse_info_pattern(input: &str) -> IResult<&str, InfoPattern, ()> {
    separated_pair(
        parse_planning_keyword,
        pair(char(':'), space0),
        tag("<timestamp>"),
    )(input)
    .map(|(rest, (keyword, timestamp))| {
        let info = InfoPattern {
            keyword,
            timestamp: timestamp.to_string(),
        };
        (rest, info)
    })
}

// Matches a single line that is the planning line.
fn parse_planning_line(input: &str) -> Option<Planning> {
    match preceded(space0, many0(terminated(parse_info_pattern, space0)))(input) {
        // A planning line needs to have at least one info pattern to be
        // considered a planning line rather than part of the body.
        Ok((_rest, infos)) if !infos.is_empty() => {
            let mut planning = Planning::default();
            for info in infos {
                // Per Org spec, in case of duplicates, keep the final.
                match info.keyword {
                    PlanningKeyword::Closed => {
                        planning.closed = Some(info.timestamp);
                    }
                    PlanningKeyword::Scheduled => {
                        planning.scheduled = Some(info.timestamp);
                    }
                    PlanningKeyword::Deadline => {
                        planning.deadline = Some(info.timestamp);
                    }
                }
            }
            Some(planning)
        }
        // FIXME: All these should distinguish nom error from failure.
        _ => None,
    }
}

// Parse the title line of a headline starting at text. Also parses planning and
// properties drawer, but not the body or child headlines,
pub(crate) fn parse_headline(input: RopeSlice, context: &Context) -> Option<Headline> {
    let (headline_rope, body) = crate::parser::structure::consuming_line(&input);

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

    // Attempt to parse a planning line out of the body.
    // ELDRITCH
    // let (planning_line, remaining_body) = crate::parser::structure::consuming_line(&body);
    // let (planning, body) = match parse_planning_line(&planning_line.to_string()) {
    //     Some(planning) => (Some(planning), remaining_body),
    //     None => (None, body),
    // };

    // Regardless of whether we parsed the planning line, attempt to parse a properties drawer.
    // ELDRITCH
    // let (properties, body) = parse_properties_drawer(body);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_planning_keyword() {
        assert_eq!(
            parse_planning_keyword("DEADLINE:").unwrap().1,
            PlanningKeyword::Deadline
        );
        assert_eq!(
            parse_planning_keyword("SCHEDULED:").unwrap().1,
            PlanningKeyword::Scheduled
        );
        assert_eq!(
            parse_planning_keyword("CLOSED:").unwrap().1,
            PlanningKeyword::Closed
        );
        assert!(parse_planning_keyword("CLOSED :").is_err());
        assert!(parse_planning_keyword(" SCHEDULED :").is_err());
        assert!(parse_planning_keyword("idk lol").is_err());
        assert!(parse_planning_keyword(" DEADLINE:").is_err());
    }

    #[test]
    fn test_parse_info_pattern() {
        let pattern = parse_info_pattern("DEADLINE: <timestamp>").unwrap().1;
        assert_eq!(pattern.keyword, PlanningKeyword::Deadline);

        let pattern = parse_info_pattern("SCHEDULED:<timestamp>").unwrap().1;
        assert_eq!(pattern.keyword, PlanningKeyword::Scheduled);

        let pattern = parse_info_pattern("CLOSED:  <timestamp>     ").unwrap().1;
        assert_eq!(pattern.keyword, PlanningKeyword::Closed);

        assert!(parse_info_pattern(" CLOSED: <timestamp>").is_err());
        assert!(parse_info_pattern("Closed: <timestamp>").is_err());
        assert!(parse_info_pattern(" ").is_err());
    }

    #[test]
    fn test_parse_planning_line() {
        let planning = parse_planning_line("DEADLINE: <timestamp>DEADLINE: <timestamp>").unwrap();
        assert_eq!(planning.deadline.unwrap(), "<timestamp>");
        assert!(planning.scheduled.is_none());
        assert!(planning.closed.is_none());

        let planning = parse_planning_line("SCHEDULED: <timestamp> DEADLINE: <timestamp>").unwrap();
        assert_eq!(planning.deadline.unwrap(), "<timestamp>");
        assert_eq!(planning.scheduled.unwrap(), "<timestamp>");
        assert!(planning.closed.is_none());

        let planning = parse_planning_line("CLOSED: <timestamp>").unwrap();
        assert_eq!(planning.closed.unwrap(), "<timestamp>");
        assert!(planning.scheduled.is_none());
        assert!(planning.deadline.is_none());

        let planning = parse_planning_line(
            "  DEADLINE: <timestamp> SCHEDULED: <timestamp> CLOSED: <timestamp>   ",
        )
        .unwrap();
        assert_eq!(planning.closed.unwrap(), "<timestamp>");
        assert_eq!(planning.deadline.unwrap(), "<timestamp>");
        assert_eq!(planning.scheduled.unwrap(), "<timestamp>");

        assert!(parse_planning_line("").is_none());
        assert!(parse_planning_line(" ").is_none());
        assert!(parse_planning_line("ESCHEDULED: <timestamp>").is_none());
        assert!(parse_planning_line("DEADLINE <timestamp>").is_none());
    }
}
