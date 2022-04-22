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

use crate::{
    Context, Headline, HeadlinePod, InfoPattern, Planning, PlanningKeyword, RopeSliceExt, Timestamp,
};

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
        Timestamp::parse,
    )(input)
    .map(|(rest, (keyword, timestamp))| {
        let info = InfoPattern {
            keyword,
            timestamp: timestamp.into_owned(),
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
    let (planning_line, remaining_body) = crate::parser::structure::consuming_line(&body);
    let (planning, body) = match parse_planning_line(&planning_line.to_string()) {
        Some(planning) => (Some(planning.into_owned()), remaining_body),
        None => (None, body),
    };

    Some(Headline(HeadlinePod {
        level,
        commented,
        keyword,
        priority,
        title: title.into(),
        raw_tags_string,
        raw_tags_rope,
        planning: planning.unwrap_or_default(),
        body: body.into(),
    }))
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::{Activity, Interval, Point, Repeater, RepeaterMark, Time, TimeUnit, TimestampExt};

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
        let pattern = parse_info_pattern("DEADLINE: [2022-08-28]").unwrap().1;
        assert_eq!(pattern.keyword, PlanningKeyword::Deadline);

        let pattern = parse_info_pattern("SCHEDULED:[2022-08-28]").unwrap().1;
        assert_eq!(pattern.keyword, PlanningKeyword::Scheduled);

        let pattern = parse_info_pattern("CLOSED:  [2022-08-28]     ").unwrap().1;
        assert_eq!(pattern.keyword, PlanningKeyword::Closed);

        assert!(parse_info_pattern(" CLOSED: [2022-08-28]").is_err());
        assert!(parse_info_pattern("Closed: [2022-08-28]").is_err());
        assert!(parse_info_pattern(" ").is_err());
    }

    #[test]
    fn test_parse_planning_line() {
        let timestamp = Timestamp::parse("[2022-08-28]").unwrap().1;
        let planning = parse_planning_line("DEADLINE: [2022-08-28]DEADLINE: [2022-08-28]").unwrap();
        assert_eq!(planning.deadline.unwrap(), timestamp);
        assert!(planning.scheduled.is_none());
        assert!(planning.closed.is_none());

        let planning =
            parse_planning_line("SCHEDULED: [2022-08-28] DEADLINE: [2022-08-28]").unwrap();
        assert_eq!(planning.deadline.unwrap(), timestamp);
        assert_eq!(planning.scheduled.unwrap(), timestamp);
        assert!(planning.closed.is_none());

        let planning = parse_planning_line("CLOSED: [2022-08-28]").unwrap();
        assert_eq!(planning.closed.unwrap(), timestamp);
        assert!(planning.scheduled.is_none());
        assert!(planning.deadline.is_none());

        let planning = parse_planning_line(
            "  DEADLINE: [2022-08-28] SCHEDULED: [2022-08-28] CLOSED: [2022-08-28]   ",
        )
        .unwrap();
        assert_eq!(planning.closed.unwrap(), timestamp);
        assert_eq!(planning.deadline.unwrap(), timestamp);
        assert_eq!(planning.scheduled.unwrap(), timestamp);

        assert!(parse_planning_line("").is_none());
        assert!(parse_planning_line(" ").is_none());
        assert!(parse_planning_line("ESCHEDULED: [2022-08-28]").is_none());
        assert!(parse_planning_line("DEADLINE [2022-08-28]").is_none());
    }

    // A regression test for one of my files. Orgize can't handle timestamps
    // that are missing the day of week (it can be invalid or wrong, but must be
    // there), but I have a lot of them.
    #[test]
    fn test_day_of_week_is_optional() {
        const TEXT: &str = r#"* DONE Send a card for her retirement
  CLOSED: [2018-05-28 Mon 10:57] SCHEDULED: <2018-05-28>
  :PROPERTIES:
  :ARCHIVE_OLPATH: Calendar/One-off Misc
  :ARCHIVE_CATEGORY: org
  :ARCHIVE_TODO: DONE
  :END:
  :LOGBOOK:
  - State "DONE"       from "TODO"       [2018-05-28 Mon 10:57]
  :END:"#;

        let h = parse_headline(Rope::from(TEXT).slice(..), &Context::default()).unwrap();
        assert!(h.planning().deadline.is_none());
        assert_eq!(3, h.properties().unwrap().len());

        let s = h.planning().scheduled.as_ref().unwrap();
        let s: Point = s.try_into().unwrap();
        assert_eq!(s.active(), Activity::Active);
        assert_eq!(
            s.date().unwrap().0.format("%Y%m%d").to_string(),
            "20180528".to_string()
        );
        assert!(s.time().is_none());
        assert!(s.cookie.repeater.is_none());
        assert!(s.cookie.delay.is_none());

        let s = h.planning().closed.as_ref().unwrap();
        let s: Point = s.try_into().unwrap();
        assert_eq!(s.active(), Activity::Inactive);
        assert_eq!(
            s.date().unwrap().0.format("%Y%m%d").to_string(),
            "20180528".to_string()
        );
        assert_eq!(s.time().unwrap(), Time::new(10, 57));
        assert!(s.cookie.repeater.is_none());
        assert!(s.cookie.delay.is_none());
    }

    // A regression test for one of my files. Ensure we permit timestamps to contain
    // Org Habit annotations even if we aren't parsing them.
    #[test]
    fn test_org_habit_annotations_allowed() {
        const TEXT: &str = r#"*** TODO Test UPS
    SCHEDULED: <2020-11-10 Tue .+20d/25d>
    :PROPERTIES:
    :STYLE:    habit
    :ACTIVE_KEYWORD: TODO
    :LAST_REPEAT: [2020-10-21 Wed 11:07]
    :END:
    - State "SKIP"       from "TODO"       [2020-09-19 Sat 08:40]
    :LOGBOOK:
    - State "DONE"       from "TODO"       [2020-10-21 Wed 11:07]
    - State "DONE"       from "TODO"       [2020-07-31 Fri 11:17]
    - State "DONE"       from "TODO"       [2020-05-30 Sat 20:01]
    - State "DONE"       from "TODO"       [2020-05-22 Fri 23:04]
    :END:"#;

        let h = parse_headline(Rope::from(TEXT).slice(..), &Context::default()).unwrap();
        assert!(h.planning().deadline.is_none());
        assert_eq!(3, h.properties().unwrap().len());

        let s = h.planning().scheduled.as_ref().unwrap();
        let s: Point = s.try_into().unwrap();
        assert_eq!(s.active(), Activity::Active);
        assert_eq!(
            s.date().unwrap().0.format("%Y%m%d").to_string(),
            "20201110".to_string()
        );
        assert!(s.time().is_none());
        assert!(s.cookie.delay.is_none());
        assert_eq!(
            s.cookie.repeater.unwrap(),
            Repeater::new(RepeaterMark::Restart, Interval::new(20, TimeUnit::Day))
        );
    }
}
