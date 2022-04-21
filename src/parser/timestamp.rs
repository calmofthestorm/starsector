use std::convert::TryFrom;

use chrono::{NaiveDate, NaiveTime};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1, take_while_m_n},
    character::complete::{char, digit1, one_of, space1},
    combinator::{map, map_res, opt, verify},
    error::{make_error, ErrorKind},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Err, IResult,
};

use crate::headline::*;

fn parse_integer_4(input: &str) -> IResult<&str, u16, ()> {
    map_res(take_while_m_n(4, 4, |c: char| c.is_ascii_digit()), |num| {
        u16::from_str_radix(num, 10)
    })(input)
}

fn parse_integer_2(input: &str) -> IResult<&str, u8, ()> {
    map_res(take_while_m_n(2, 2, |c: char| c.is_ascii_digit()), |num| {
        u8::from_str_radix(num, 10)
    })(input)
}

fn parse_integer_1_2(input: &str) -> IResult<&str, u8, ()> {
    map_res(take_while_m_n(1, 2, |c: char| c.is_ascii_digit()), |num| {
        u8::from_str_radix(num, 10)
    })(input)
}

// fn parse_dayname(input: &str) -> IResult<&str, &str, ()> {
//     let (input, dayname) = verify(
//         take_while1(|c: char| !c.is_whitespace() && c != '>' && c != ']'),
//         |dayname: &str| {
//             !dayname
//                 .chars()
//                 .any(|c| c.is_ascii_digit() || c == '+' || c == '-')
//         },
//     )(input)?;
//     Ok((input, dayname))
// }

impl Time {
    pub(crate) fn parse(input: &str) -> IResult<&str, Time, ()> {
        let (input, hour) = parse_integer_1_2(input)?;
        let (input, _) = tag(":")(input)?;
        let (input, minute) = parse_integer_2(input)?;
        let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0)
            .ok_or_else(|| Err::Error(make_error(input, ErrorKind::Verify)))?;
        Ok((input, time.into()))
    }
}

impl Times {
    pub(crate) fn parse(input: &str) -> IResult<&str, Times, ()> {
        let (input, (start, end)) = separated_pair(Time::parse, char('-'), Time::parse)(input)?;
        Ok((input, Times(start, end)))
    }
}

impl TimeSpec {
    pub(crate) fn parse(input: &str) -> IResult<&str, TimeSpec, ()> {
        let (input, start) = Time::parse(input)?;
        match opt(preceded(char('-'), Time::parse))(input) {
            Ok((input, Some(end))) => Ok((input, TimeSpec::Times(Times(start, end)))),
            _ => Ok((input, TimeSpec::Time(start))),
        }
    }
}

impl RepeaterMark {
    fn parse(input: &str) -> IResult<&str, RepeaterMark, ()> {
        let (input, mark) = alt((tag("++"), tag("+"), tag(".+")))(input)?;
        Ok((
            input,
            match mark {
                "++" => RepeaterMark::CatchUp,
                "+" => RepeaterMark::Cumulate,
                ".+" => RepeaterMark::Restart,
                _ => unreachable!(),
            },
        ))
    }
}

impl DelayMark {
    fn parse(input: &str) -> IResult<&str, DelayMark, ()> {
        let (input, mark) = alt((tag("--"), tag("-")))(input)?;
        Ok((
            input,
            match mark {
                "--" => DelayMark::First,
                "-" => DelayMark::All,
                _ => unreachable!(),
            },
        ))
    }
}

impl TimeUnit {
    fn parse(input: &str) -> IResult<&str, TimeUnit, ()> {
        let (input, unit) = one_of("hdwmy")(input)?;
        Ok((
            input,
            match unit {
                'h' => TimeUnit::Hour,
                'd' => TimeUnit::Day,
                'w' => TimeUnit::Week,
                'm' => TimeUnit::Month,
                'y' => TimeUnit::Year,
                _ => unreachable!(),
            },
        ))
    }
}

impl Interval {
    fn parse(input: &str) -> IResult<&str, Interval, ()> {
        let (input, value) = map_res(digit1, |num| usize::from_str_radix(num, 10))(input)?;
        let (input, unit) = TimeUnit::parse(input)?;
        Ok((input, Interval::new(value, unit)))
    }
}

impl Repeater {
    fn parse(input: &str) -> IResult<&str, Repeater, ()> {
        let (input, mark) = RepeaterMark::parse(input)?;
        let (input, interval) = Interval::parse(input)?;
        Ok((input, Repeater { mark, interval }))
    }
}

// impl TryFrom<&str> for Repeater {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// impl Parse for Delay {
//     fn parse(input: &str) -> IResult<&str, Delay, ()> {
//         let (input, mark) = DelayMark::parse(input)?;
//         let (input, interval) = Interval::parse(input)?;
//         Ok((input, Delay { mark, interval }))
//     }
// }

// impl TryFrom<&str> for Delay {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// impl Parse for RepeaterAndDelay {
//     fn parse(input: &str) -> IResult<&str, RepeaterAndDelay, ()> {
//         let (input, repeater, delay) = if let Ok((input, (repeater, delay))) =
//             pair(Repeater::parse, preceded(space1, Delay::parse))(input)
//         {
//             (input, Some(repeater), Some(delay))
//         } else if let Ok((input, (delay, repeater))) =
//             pair(Delay::parse, preceded(space1, Repeater::parse))(input)
//         {
//             (input, Some(repeater), Some(delay))
//         } else if let Ok((input, delay)) = Delay::parse(input) {
//             (input, None, Some(delay))
//         } else if let Ok((input, repeater)) = Repeater::parse(input) {
//             (input, Some(repeater), None)
//         } else {
//             (input, None, None)
//         };
//         Ok((input, RepeaterAndDelay { repeater, delay }))
//     }
// }

// impl TryFrom<&str> for RepeaterAndDelay {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// impl Parse for Date {
//     fn parse(input: &str) -> IResult<&str, Date, ()> {
//         let (input, (year, month, day)) = tuple((
//             parse_integer_4,
//             preceded(char('-'), parse_integer_2),
//             preceded(char('-'), parse_integer_2),
//         ))(input)?;
//         let (input, _dayname) = opt(preceded(space1, parse_dayname))(input)?;
//         Ok((
//             input,
//             NaiveDate::from_ymd(year as i32, month as u32, day as u32).into(),
//         ))
//     }
// }

// impl TryFrom<&str> for Date {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// fn parse_atomic_timestamp(input: &str) -> IResult<&str, (Point, Option<Time>), ()> {
//     // Annoying, but we want to allow RepeaterAndDelay to be parsed in
//     // isolation, but also to be empty, and it needs a leading space iff
//     // non-empty.
//     let inner = |active: Activity| {
//         map(
//             tuple((
//                 Date::parse,
//                 opt(preceded(space1, TimeSpec::parse)),
//                 terminated(
//                     alt((
//                         verify(preceded(space1, RepeaterAndDelay::parse), |rad| {
//                             rad.repeater.is_some() || rad.delay.is_some()
//                         }),
//                         verify(RepeaterAndDelay::parse, |rad| {
//                             rad.repeater.is_none() && rad.delay.is_none()
//                         }),
//                     )),
//                     opt(is_not(">]\n")),
//                 ),
//             )),
//             move |(date, time, cookie)| {
//                 let (start, end) = match time {
//                     None => (None, None),
//                     Some(TimeSpec::Time(start)) => (Some(start), None),
//                     Some(TimeSpec::Times(Times(start, end))) => (Some(start), Some(end)),
//                 };
//                 (
//                     Point {
//                         active,
//                         date,
//                         cookie,
//                         time: start,
//                     },
//                     end,
//                 )
//             },
//         )
//     };

//     terminated(
//         alt((
//             preceded(tag("<"), inner(Activity::Active)),
//             preceded(tag("["), inner(Activity::Inactive)),
//         )),
//         one_of("]>"),
//     )(input)
// }

// impl Parse for Point {
//     fn parse(input: &str) -> IResult<&str, Point, ()> {
//         let (input, (point, _none)) = verify(parse_atomic_timestamp, |(_, e)| e.is_none())(input)?;
//         Ok((input, point))
//     }
// }

// impl TryFrom<&str> for Point {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// impl Parse for Range {
//     fn parse(input: &str) -> IResult<&str, Range, ()> {
//         let (input, (start, mut end)) =
//             separated_pair(Point::parse, tag("--"), Point::parse)(input)?;
//         end.active = start.active;
//         Ok((input, Range { start, end }))
//     }
// }

// impl TryFrom<&str> for Range {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// impl Parse for TimeRange {
//     fn parse(input: &str) -> IResult<&str, TimeRange, ()> {
//         let (input, (start, end_time)) =
//             verify(parse_atomic_timestamp, |(_, e)| e.is_some())(input)?;
//         let end_time = end_time.expect("verified");
//         Ok((input, TimeRange { start, end_time }))
//     }
// }

// impl TryFrom<&str> for TimeRange {
//     type Error = ();
//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         try_from_helper::<Self>(Self::parse(input))
//     }
// }

// impl<'a> ParseB<'a> for Diary<'a> {
//     fn parse(input: &'a str) -> IResult<&'a str, Diary<'a>, ()> {
//         map(
//             verify(
//                 delimited(tag("<%%("), is_not("\n>"), char('>')),
//                 |d: &str| d.ends_with(')'),
//             ),
//             |diary: &str| Diary(diary[..diary.len() - 1].into()),
//         )(input)
//     }
// }

// impl<'a> ParseB<'a> for Timestamp<'a> {
//     fn parse(input: &'a str) -> IResult<&'a str, Timestamp<'a>, ()> {
//         alt((
//             map(Diary::parse, Into::into),
//             map(Range::parse, Into::into),
//             map(TimeRange::parse, Into::into),
//             map(Point::parse, Into::into),
//         ))(input)
//     }
// }

// // TODO: Move this
// impl<'a, D: AsRef<Diary<'a>>> From<D> for Timestamp<'a> {
//     fn from(diary: D) -> Self {
//         Timestamp::Diary(diary.as_ref().clone())
//     }
// }

// // TODO: Move this
// impl<'a> From<Diary<'a>> for Timestamp<'a> {
//     fn from(diary: Diary<'a>) -> Self {
//         Timestamp::Diary(diary)
//     }
// }

// // TODO: Move this
// impl<'a> From<Point> for Timestamp<'a> {
//     fn from(point: Point) -> Self {
//         (&point).into()
//     }
// }

// // TODO: Move this
// impl<'a> From<&Point> for Timestamp<'a> {
//     fn from(point: &Point) -> Self {
//         Timestamp::Point(*point)
//     }
// }

// // TODO: Move this
// impl<'a> From<Range> for Timestamp<'a> {
//     fn from(range: Range) -> Self {
//         (&range).into()
//     }
// }

// // TODO: Move this
// impl<'a> From<&Range> for Timestamp<'a> {
//     fn from(range: &Range) -> Self {
//         Timestamp::Range(*range)
//     }
// }

// // TODO: Move this
// impl<'a> From<TimeRange> for Timestamp<'a> {
//     fn from(time_range: TimeRange) -> Self {
//         (&time_range).into()
//     }
// }

// // TODO: Move this
// impl<'a> From<&TimeRange> for Timestamp<'a> {
//     fn from(time_range: &TimeRange) -> Self {
//         Timestamp::TimeRange(*time_range)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_parse_time() {
        let time = |h, m| Time::new(h, m);
        assert_eq!(time(1, 20), Time::parse("01:20").unwrap().1);
        assert_eq!(time(1, 20), Time::parse("1:20").unwrap().1);
        assert_eq!(time(0, 0), Time::parse("00:00").unwrap().1);
        assert_eq!(time(0, 0), Time::parse("0:00").unwrap().1);

        for bad in &[
            "x:1", "5:1", "00:0", "1:61", "", "5", ":", "-5", "1:-5", ":05", "01:", "-1:5", "161",
            "0161",
        ] {
            assert!(Time::parse(*bad).is_err());
        }

        let res = Time::parse("5:55 ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(time(5, 55), res.1);
    }

    #[test]
    fn test_parse_times() {
        let time = |h, m| Time::new(h, m);
        let times = |(h1, m1), (h2, m2)| Times(time(h1, m1), time(h2, m2));

        let t = times((5, 20), (8, 25));
        for good in &["05:20-08:25", "05:20-8:25", "5:20-08:25", "5:20-8:25"] {
            let result = Times::parse(good).unwrap();
            assert_eq!("", result.0);
            assert_eq!(t, result.1);
        }

        for bad in &[
            "0:0",
            "",
            "00:0",
            "0:00",
            "05:05--10:10",
            "05:05 -10:10",
            "05:05- 10:10",
            "1:11",
            "1:11-",
            "-",
            "-1:11",
            "5-5",
            "05:-05:",
        ] {
            assert!(Times::parse(*bad).is_err());
        }

        let result = Times::parse("05:05-10:10 ").unwrap();
        assert_eq!(" ", result.0);
        assert_eq!(Times::parse("05:05-10:10").unwrap().1, result.1);
    }

    #[test]
    fn test_parse_time_spec() {
        let time = |h, m| Time::new(h, m);
        let times = |(h1, m1), (h2, m2)| TimeSpec::Times(Times(time(h1, m1), time(h2, m2)));
        let time = |h, m| TimeSpec::Time(time(h, m));

        assert_eq!(time(5, 20), TimeSpec::parse("05:20").unwrap().1);
        assert_eq!(
            times((5, 20), (8, 25)),
            TimeSpec::parse("05:20-8:25").unwrap().1
        );

        for bad in &[
            "05:55 ",
            "05:05--10:10",
            "0:0",
            "",
            "00:0",
            "05:05 -10:10",
            "05:05- 10:10",
            "1:11-",
            "-",
            "-1:11",
            "5-5",
            "05:-05:",
        ] {
            assert!(Times::parse(*bad).is_err());
        }
    }

    #[test]
    fn test_parse_repeater_mark() {
        assert_eq!(RepeaterMark::parse("+").unwrap().1, RepeaterMark::Cumulate);
        assert_eq!(RepeaterMark::parse("++").unwrap().1, RepeaterMark::CatchUp);
        assert_eq!(RepeaterMark::parse(".+").unwrap().1, RepeaterMark::Restart);

        for bad in &["", ".", "-"] {
            assert!(RepeaterMark::parse(*bad).is_err());
        }

        let res = RepeaterMark::parse("+ ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(RepeaterMark::parse("+").unwrap().1, res.1);

        let res = RepeaterMark::parse("++ ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(RepeaterMark::parse("++").unwrap().1, res.1);

        let res = RepeaterMark::parse("+++").unwrap();
        assert_eq!("+", res.0);
        assert_eq!(RepeaterMark::parse("+++").unwrap().1, res.1);

        let res = RepeaterMark::parse(".++").unwrap();
        assert_eq!("+", res.0);
        assert_eq!(RepeaterMark::parse(".++").unwrap().1, res.1);

        let res = RepeaterMark::parse("++.").unwrap();
        assert_eq!(".", res.0);
        assert_eq!(RepeaterMark::parse("++.").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_delay_mark() {
        assert_eq!(DelayMark::parse("-").unwrap().1, DelayMark::All);
        assert_eq!(DelayMark::parse("--").unwrap().1, DelayMark::First);

        assert!(DelayMark::parse("").is_err());

        let res = DelayMark::parse("---").unwrap();
        assert_eq!("-", res.0);
        assert_eq!(DelayMark::parse("--").unwrap().1, res.1);

        assert!(DelayMark::parse("+").is_err());

        let res = DelayMark::parse("-- ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(DelayMark::parse("--").unwrap().1, res.1);

        let res = DelayMark::parse("- ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(DelayMark::parse("-").unwrap().1, res.1);

        assert!(DelayMark::parse(".+").is_err());

        assert!(DelayMark::parse("+").is_err());

        let res = DelayMark::parse("-5d").unwrap();
        assert_eq!("5d", res.0);
        assert_eq!(DelayMark::parse("-").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_time_unit() {
        assert_eq!(TimeUnit::parse("h").unwrap().1, TimeUnit::Hour);
        assert_eq!(TimeUnit::parse("d").unwrap().1, TimeUnit::Day);
        assert_eq!(TimeUnit::parse("w").unwrap().1, TimeUnit::Week);
        assert_eq!(TimeUnit::parse("m").unwrap().1, TimeUnit::Month);
        assert_eq!(TimeUnit::parse("y").unwrap().1, TimeUnit::Year);

        let res = TimeUnit::parse("y ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(TimeUnit::parse("y").unwrap().1, res.1);

        assert!(TimeUnit::parse("a").is_err());
        assert!(TimeUnit::parse("").is_err());
        assert!(TimeUnit::parse("5d").is_err());
    }

    #[test]
    fn test_parse_interval() {
        assert_eq!(
            Interval::parse("5h").unwrap().1,
            Interval::new(5, TimeUnit::Hour)
        );
        assert_eq!(
            Interval::parse("03d").unwrap().1,
            Interval::new(3, TimeUnit::Day)
        );
        assert_eq!(
            Interval::parse("273w").unwrap().1,
            Interval::new(273, TimeUnit::Week)
        );

        assert!(Interval::parse("5a").is_err());
        assert!(Interval::parse("").is_err());
        assert!(Interval::parse("+5d").is_err());
        assert!(Interval::parse("h").is_err());

        let res = Interval::parse("5m ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(Interval::parse("5m").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_repeater() {
        let repeater = |m: &str, i: &str| {
            Repeater::new(
                RepeaterMark::parse(m).unwrap().1,
                Interval::parse(i).unwrap().1,
            )
        };
        assert_eq!(Repeater::parse("+5h").unwrap().1, repeater("+", "5h"));
        assert_eq!(Repeater::parse(".+7y").unwrap().1, repeater(".+", "7y"));
        assert_eq!(Repeater::parse("++0m").unwrap().1, repeater("++", "0m"));

        assert!(Interval::parse("+6h ").is_err());

        for bad in &["+++5w", "", "-7m"] {
            eprintln!("Foo {}", bad);
            assert!(Repeater::parse(*bad).is_err());
        }
    }

    //     #[test]
    //     fn test_parse_delay() {
    //         let delay = |m: &str, i: &str| Some(Repeater::new(m, i));
    //         assert_eq!("+5h".try_into().ok(), delay("+", "5h"));
    //         assert_eq!(".+7y".try_into().ok(), delay(".+", "7y"));
    //         assert_eq!("++0m".try_into().ok(), delay("++", "0m"));

    //         for bad in &["---5w", "", "-6h ", "+7m"] {
    //             assert!(Delay::try_from(*bad).is_err());
    //         }
    //     }

    //     #[test]
    //     fn test_parse_repeater_and_delay() {
    //         let repeater = Some(Repeater::new("+", "5d"));
    //         let delay = Some(Delay::new("--", "7w"));

    //         assert_eq!(
    //             "+5d --7w".try_into().ok(),
    //             Some(RepeaterAndDelay { repeater, delay })
    //         );

    //         assert_eq!(
    //             "--7w +5d".try_into().ok(),
    //             Some(RepeaterAndDelay { repeater, delay })
    //         );

    //         assert_eq!(
    //             "--7w \t+5d".try_into().ok(),
    //             Some(RepeaterAndDelay { repeater, delay })
    //         );

    //         assert_eq!(
    //             "+5d".try_into().ok(),
    //             Some(RepeaterAndDelay {
    //                 repeater,
    //                 delay: None
    //             })
    //         );

    //         assert_eq!(
    //             "--7w".try_into().ok(),
    //             Some(RepeaterAndDelay {
    //                 repeater: None,
    //                 delay
    //             })
    //         );

    //         assert_eq!(
    //             "".try_into().ok(),
    //             Some(RepeaterAndDelay {
    //                 repeater: None,
    //                 delay: None
    //             })
    //         );

    //         for bad in &["---5w", "-6h ", "+7m "] {
    //             assert!(RepeaterAndDelay::try_from(*bad).is_err());
    //         }
    //     }

    //     #[test]
    //     fn test_parse_date() {
    //         let date = |y, m, d| Some(Date::from(NaiveDate::from_ymd(y, m, d)));

    //         assert_eq!("2020-01-10".try_into().ok(), date(2020, 1, 10));
    //         assert_eq!("2020-01-10 Fri".try_into().ok(), date(2020, 1, 10));
    //         assert_eq!("2020-01-10 Sat".try_into().ok(), date(2020, 1, 10));
    //         assert_eq!("2020-01-10  Zeepsday".try_into().ok(), date(2020, 1, 10));

    //         assert_eq!("0020-01-10".try_into().ok(), date(0020, 1, 10));

    //         for bad in &[
    //             "2020-02-02 ",
    //             " 2020-02-02",
    //             "2020",
    //             "",
    //             "20200110",
    //             "20200110 3:14",
    //             "5",
    //             "202-05-05",
    //             "-1986-08-24",
    //             "1987-5-29",
    //             "1987-03-1",
    //         ] {
    //             assert!(Date::try_from(*bad).is_err());
    //         }
    //     }

    //     #[test]
    //     fn test_parse_point() {
    //         let point = Point::new("2020-01-01".try_into().unwrap());

    //         assert_eq!("<2020-01-01>".try_into().ok(), Some(point));
    //         assert_eq!("<2020-01-01   Mon>".try_into().ok(), Some(point));
    //         assert_eq!(
    //             "[2020-01-01   Mon 03:57  --1d .+1w]".try_into().ok(),
    //             Some(
    //                 point
    //                     .with_repeater(Some(".+1w"))
    //                     .with_delay(Some("--1d"))
    //                     .with_active(false)
    //                     .with_time(Some("3:57"))
    //             )
    //         );

    //         for bad in &[
    //             "<2020-01-01>--<2020-02-01>",
    //             "",
    //             "<2020-01-01> ",
    //             "2020-01-01",
    //             "<%%(hi)>",
    //             "[2020-01-01 01:00-02:00]",
    //         ] {
    //             assert!(Point::try_from(*bad).is_err());
    //         }
    //     }

    //     #[test]
    //     fn test_parse_range() {
    //         let timestamp = Range::new(
    //             "[2020-01-01]".try_into().unwrap(),
    //             "[2021-01-01]".try_into().unwrap(),
    //         );

    //         assert_eq!(
    //             "[2020-01-01]--[2021-01-01]".try_into().ok(),
    //             Some(timestamp)
    //         );
    //         assert_eq!(
    //             "[2020-01-01    04:59 .+1w]--[2021-01-01 .+2d]"
    //                 .try_into()
    //                 .ok(),
    //             Some(Range {
    //                 start: timestamp
    //                     .start
    //                     .with_time(Some("4:59"))
    //                     .with_repeater(Some(".+1w")),
    //                 end: timestamp.end.with_repeater(Some(".+2d"))
    //             })
    //         );

    //         for bad in &[
    //             "<2020-01-01>--<2020-02-01> ",
    //             "",
    //             "<2020-01-01>",
    //             "2020-01-01--2021-01-01",
    //             "<%%(hi)>",
    //             "[2020-01-01 01:00-02:00]",
    //         ] {
    //             assert!(Range::try_from(*bad).is_err());
    //         }
    //     }

    //     #[test]
    //     fn test_parse_time_range() {
    //         let timestamp = Range::new(
    //             "[2020-01-01]".try_into().unwrap(),
    //             "[2021-01-01]".try_into().unwrap(),
    //         );

    //         assert_eq!(
    //             "[2020-01-01]--[2021-01-01]".try_into().ok(),
    //             Some(timestamp)
    //         );
    //         assert_eq!(
    //             "[2020-01-01    04:59 .+1w]--[2021-01-01 .+2d]"
    //                 .try_into()
    //                 .ok(),
    //             Some(Range {
    //                 start: timestamp
    //                     .start
    //                     .with_time(Some("4:59"))
    //                     .with_repeater(Some(".+1w")),
    //                 end: timestamp.end.with_repeater(Some(".+2d"))
    //             })
    //         );

    //         for bad in &[
    //             "<2020-01-01>--<2020-02-01> ",
    //             "",
    //             "<2020-01-01>",
    //             "2020-01-01--2021-01-01",
    //             "<%%(hi)>",
    //             "[2020-01-01 01:00-02:00]",
    //         ] {
    //             assert!(Range::try_from(*bad).is_err());
    //         }
    //     }

    //     #[test]
    //     fn test_parse_diary() {
    //         assert_eq!(Diary::try_from_str("<%%()>").ok(), Some(Diary("".into())));
    //         assert_eq!(
    //             Diary::try_from_str("<%%(diary-date 2020 3 1)>").ok(),
    //             Some(Diary("diary-date 2020 3 1".into()))
    //         );
    //     }

    //     // Most of the tests assume inner types are correctly implemented,
    //     // however because there is so much overlap between the different
    //     // timestamp forms, we do the thorough tests here and only basic tests
    //     // in each form's test.
    //     mod timestamp {
    //         use super::*;

    //         #[test]
    //         fn test_parse_timestamp_point() {
    //             let date = |y, m, d| Date::from(NaiveDate::from_ymd(y, m, d));
    //             let point = Point::new(date(2020, 3, 1));

    //             assert_eq!("<2020-03-01>".try_into().ok(), Some(point));
    //             assert_eq!(
    //                 "[2020-03-01]".try_into().ok(),
    //                 Some(point.with_active(false))
    //             );
    //             assert_eq!(
    //                 "[2020-03-01 Wed]".try_into().ok(),
    //                 Some(point.with_active(false))
    //             );
    //             assert_eq!("<2020-03-01 Zee>".try_into().ok(), Some(point));
    //             assert_eq!("<2020-03-01 >".try_into().ok(), Some(point));
    //             assert_eq!("<2020-03-01 \t>".try_into().ok(), Some(point));
    //             let time = Some(NaiveTime::from_hms(3, 59, 0));
    //             assert_eq!(
    //                 "<2020-03-01 \tFri  3:59>".try_into().ok(),
    //                 Some(point.with_time(time))
    //             );
    //             assert_eq!(
    //                 "<2020-03-01 \tFri  3:59>".try_into().ok(),
    //                 Some(point.with_time(time))
    //             );

    //             assert_eq!(
    //                 "<2020-03-01 3:59\t \t \t>".try_into().ok(),
    //                 Some(point.with_time(Some(Time::new(3, 59))))
    //             );

    //             assert_eq!(
    //                 "[2020-03-01 .+1w]".try_into().ok(),
    //                 Some(point.with_repeater(Some(".+1w")).with_active(false))
    //             );

    //             assert_eq!(
    //                 "<2020-03-01   \t-1d\t  >".try_into().ok(),
    //                 Some(point.with_delay(Some("-1d")))
    //             );

    //             assert_eq!(
    //                 "<2020-03-01   \t-1d\t  .+1d/1w  >".try_into().ok(),
    //                 Some(point.with_repeater(Some(".+1d")).with_delay(Some("-1d")))
    //             );
    //             assert_eq!(
    //                 "<2020-03-01 arbitrary text .+1d --2w here 夫妻肺片]"
    //                     .try_into()
    //                     .ok(),
    //                 Some(point)
    //             );
    //             assert_eq!(
    //                 "<2020-03-01 Fri .+1d .+1d>".try_into().ok(),
    //                 Some(point.with_repeater(Some(".+1d")))
    //             );
    //             assert_eq!(
    //                 "[2020-03-01>".try_into().ok(),
    //                 Some(point.with_active(false))
    //             );

    //             for bad in &[
    //                 "<2020-01-01>>",
    //                 "2020-01-01",
    //                 "",
    //                 "<2020-01-01> ",
    //                 "<2020-01-02]>",
    //                 "<2020-01-02>]",
    //                 "[2020-01-02]>",
    //                 "[2020-01-02>]",
    //             ] {
    //                 assert!(Timestamp::try_from_str(*bad).is_err())
    //             }
    //         }

    //         #[test]
    //         fn test_parse_timestamp_diary() {
    //             assert_eq!(
    //                 Timestamp::try_from_str(
    //                     "<%%(anything goes here but newline and closy angle bracket)>"
    //                 )
    //                 .ok(),
    //                 Some(Timestamp::Diary(Diary(
    //                     "anything goes here but newline and closy angle bracket".into()
    //                 )))
    //             );

    //             assert_eq!(
    //                 Timestamp::try_from_str("<%%())>").ok(),
    //                 Some(Timestamp::Diary(Diary(")".into())))
    //             );

    //             assert_eq!(
    //                 Timestamp::try_from_str("<%%([2020-01-01])>").ok(),
    //                 Some(Timestamp::Diary(Diary("[2020-01-01]".into())))
    //             );

    //             for bad in &["<%%(<2020-01-01>)>"] {
    //                 assert!(Timestamp::try_from_str(*bad).is_err())
    //             }
    //         }

    //         // FIXME
    //         #[test]
    //         fn test_parse_timestamp_time_range() {}

    //         // FIXME
    //         #[test]
    //         fn test_parse_timestamp_range() {}
    //     }

    //     #[test]
    //     fn test_parse_activity() {
    //         assert_eq!(parse_activity(""), None);
    //         assert_eq!(parse_activity("<"), None);
    //         assert_eq!(parse_activity(">"), None);
    //         assert_eq!(parse_activity("<>"), Some((Activity::Active, "")));
    //         assert_eq!(parse_activity("[]"), Some((Activity::Inactive, "")));
    //         assert_eq!(parse_activity("[hello] world"), None);
    //         assert_eq!(parse_activity("<hello>\nhow are you"), None);
    //         assert_eq!(
    //             parse_activity("<2020-01-01>"),
    //             Some((Activity::Active, "2020-01-01"))
    //         );
    //         assert_eq!(parse_activity("nope <2020-01-01 Mon>"), None);
    //         assert_eq!(
    //             parse_activity("<2020-01-01 Mon>"),
    //             Some((Activity::Active, "2020-01-01 Mon"))
    //         );
    //         assert_eq!(
    //             parse_activity("[2020-01-01 Mon]"),
    //             Some((Activity::Inactive, "2020-01-01 Mon"))
    //         );
    //         assert_eq!(parse_activity("[2020-01-01 Mon>"), None);
    //         assert_eq!(parse_activity("[2020-01-01 Mon>"), None);
    //     }

    //     #[test]
    //     fn test_parse_ymd() {
    //         assert_eq!(
    //             parse_ymd("2020-03-01"),
    //             Some((NaiveDate::from_ymd(2020, 3, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-03-01 Sun"),
    //             Some((NaiveDate::from_ymd(2020, 3, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-3-01 Zeepsday"),
    //             Some((NaiveDate::from_ymd(2020, 3, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-07-1 Sun"),
    //             Some((NaiveDate::from_ymd(2020, 7, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-1-1 FRI"),
    //             Some((NaiveDate::from_ymd(2020, 1, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-1-1 .+1d"),
    //             Some((NaiveDate::from_ymd(2020, 1, 1).into(), ".+1d"))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-1-1 .+1d/2d"),
    //             Some((NaiveDate::from_ymd(2020, 1, 1).into(), ".+1d/2d"))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-03-01"),
    //             Some((NaiveDate::from_ymd(2020, 3, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-03-01 Sun"),
    //             Some((NaiveDate::from_ymd(2020, 3, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-3-01 Zeepsday"),
    //             Some((NaiveDate::from_ymd(2020, 3, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-07-1 Sun"),
    //             Some((NaiveDate::from_ymd(2020, 7, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-1-1 FRI"),
    //             Some((NaiveDate::from_ymd(2020, 1, 1).into(), ""))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-1-1 .+1d"),
    //             Some((NaiveDate::from_ymd(2020, 1, 1).into(), ".+1d"))
    //         );

    //         assert_eq!(
    //             parse_ymd("2020-1-1 .+1d/2d"),
    //             Some((NaiveDate::from_ymd(2020, 1, 1).into(), ".+1d/2d"))
    //         );

    //         assert_eq!(
    //             parse_ymd("0000-1-1 .+1d/2d"),
    //             Some((NaiveDate::from_ymd(0, 1, 1).into(), ".+1d/2d"))
    //         );

    //         assert_eq!(
    //             parse_ymd("0005-1-1 .+1d/2d"),
    //             Some((NaiveDate::from_ymd(5, 1, 1).into(), ".+1d/2d"))
    //         );

    //         assert_eq!(
    //             parse_ymd("9999-1-1 .+1d/2d"),
    //             Some((NaiveDate::from_ymd(9999, 1, 1).into(), ".+1d/2d"))
    //         );

    //         // FIXME
    //         // assert_eq!(
    //         //     parse_ymd("2-1-1 .+1d/2d"),
    //         //     None,
    //         // );

    //         // FIXME
    //         // assert_eq!(
    //         //     parse_ymd("2020-13-1 "),
    //         //     Some((NaiveDate::from_ymd(2021, 1, 1).into(), ""))
    //         // );

    //         // FIXME
    //         // assert_eq!(
    //         //     parse_ymd("2020-1-32 "),
    //         //     Some((NaiveDate::from_ymd(2021, 2, 1).into(), ""))
    //         // );

    //         // FIXME
    //         // assert_eq!(
    //         //     parse_ymd("2019-02-29 "),
    //         //     Some((NaiveDate::from_ymd(2019, 3, 1).into(), ""))
    //         // );

    //         assert_eq!(parse_ymd("-1-1"), None,);
    //         assert_eq!(parse_ymd("1-1"), None,);
    //         assert_eq!(parse_ymd("1"), None,);
    //         assert_eq!(parse_ymd(""), None,);
    //     }

    //     #[test]
    //     fn test_parse_time() {
    //         assert_eq!(parse_time(""), None);
    //         assert_eq!(parse_time(":"), None);
    //         assert_eq!(parse_time("9"), None);
    //         assert_eq!(parse_time("9-9"), None);
    //         assert_eq!(
    //             parse_time("00:09-00:10"),
    //             Some((NaiveTime::from_hms(0, 9, 0).into(), "-00:10"))
    //         );

    //         // FIXME
    //         // assert_eq!(parse_time("5:5"), None);

    //         assert_eq!(
    //             parse_time("5:05"),
    //             Some((NaiveTime::from_hms(5, 5, 0).into(), ""))
    //         );
    //         assert_eq!(
    //             parse_time("05:05"),
    //             Some((NaiveTime::from_hms(5, 5, 0).into(), ""))
    //         );
    //         assert_eq!(
    //             parse_time("00:05\tbees"),
    //             Some((NaiveTime::from_hms(0, 5, 0).into(), "bees"))
    //         );
    //         assert_eq!(
    //             parse_time("00:00 .+1w"),
    //             Some((NaiveTime::from_hms(0, 0, 0).into(), ".+1w"))
    //         );
    //         assert_eq!(
    //             parse_time("0:00"),
    //             Some((NaiveTime::from_hms(0, 0, 0).into(), ""))
    //         );
    //         assert_eq!(
    //             parse_time("0:01"),
    //             Some((NaiveTime::from_hms(0, 1, 0).into(), ""))
    //         );
    //     }

    //     #[test]
    //     fn test_parse_point_only() {
    //         assert_eq!(parse_point_only(""), None);
    //         assert_eq!(parse_point_only(" "), None);
    //         let date = NaiveDate::from_ymd(2020, 1, 1).into();
    //         let time = Some(NaiveTime::from_hms(5, 37, 0).into());
    //         assert_eq!(
    //             parse_point_only("<2020-01-01>"),
    //             Some((
    //                 Activity::Active,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie: RepeaterAndDelay::default()
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("<2020-01-01 Fri>"),
    //             Some((
    //                 Activity::Active,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie: RepeaterAndDelay::default()
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("<2020-01-01 Sat 05:37>"),
    //             Some((
    //                 Activity::Active,
    //                 Point {
    //                     date,
    //                     time,
    //                     cookie: RepeaterAndDelay::default()
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 05:37]"),
    //             Some((
    //                 Activity::Inactive,
    //                 Point {
    //                     date,
    //                     time,
    //                     cookie: RepeaterAndDelay::default()
    //                 }
    //             ))
    //         );
    //         // FIXME -- day, delay, repeater, time are unambiguous and Org accepts them in any order. This and general case.
    //         // assert_eq!(
    //         //     parse_point_only("[2020-01-01 05:37 Mon]"),
    //         //     Some((
    //         //         Activity::Inactive,
    //         //         Point {
    //         //             date,
    //         //             time,
    //         //             cookie: RepeaterAndDelay::default()
    //         //         }
    //         //     ))
    //         // );

    //         // day x delay x repeater x habit x time

    //         let delay = Some(Delay {
    //             mark: DelayMark::First,
    //             unit: Unit::Week,
    //             value: 1,
    //         });
    //         let repeater = Some(Repeater {
    //             mark: RepeaterMark::Restart,
    //             unit: Unit::Day,
    //             value: 2,
    //             habit: None,
    //         });
    //         let with_habit = repeater.map(|r| Repeater {
    //             habit: Some((5, Unit::Week)),
    //             ..r
    //         });
    //         let inactive = Activity::Inactive;

    //         let cookie = RepeaterAndDelay {
    //             delay,
    //             repeater: None,
    //         };
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );

    //         let cookie = RepeaterAndDelay {
    //             delay: None,
    //             repeater,
    //         };
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );

    //         let cookie = RepeaterAndDelay {
    //             delay: None,
    //             repeater: with_habit,
    //         };
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d/5w ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d/5w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d/5w ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d/5w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d/5w ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d/5w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d/5w ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d/5w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );

    //         let cookie = RepeaterAndDelay::default();
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );

    //         let cookie = RepeaterAndDelay { delay, repeater };
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w .+2d ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d --1w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w .+2d ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w .+2d ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d --1w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w .+2d ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );

    //         let cookie = RepeaterAndDelay {
    //             delay,
    //             repeater: with_habit,
    //         };
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w .+2d/5w ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d/5w --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w .+2d/5w ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d/5w --1w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri --1w .+2d/5w ]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri .+2d/5w --1w]"),
    //             Some((
    //                 inactive,
    //                 Point {
    //                     date,
    //                     time: None,
    //                     cookie
    //                 }
    //             ))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 --1w .+2d/5w ]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             parse_point_only("[2020-01-01 Fri 05:37 .+2d/5w --1w]"),
    //             Some((inactive, Point { date, time, cookie }))
    //         );
    //     }

    //     #[test]
    //     fn test_parse_value_unit_only() {
    //         assert_eq!(parse_value_unit_only("5d"), Some((5, Unit::Day)));
    //         assert_eq!(parse_value_unit_only("0h"), Some((0, Unit::Hour)));
    //         assert_eq!(parse_value_unit_only("1h"), Some((1, Unit::Hour)));
    //         assert_eq!(parse_value_unit_only("2m"), Some((2, Unit::Month)));
    //         assert_eq!(parse_value_unit_only("02w"), Some((2, Unit::Week)));
    //         assert_eq!(parse_value_unit_only("02w "), None);
    //         assert_eq!(parse_value_unit_only("222y"), Some((222, Unit::Year)));
    //         assert_eq!(parse_value_unit_only(""), None);
    //         assert_eq!(parse_value_unit_only("5"), None);
    //         assert_eq!(parse_value_unit_only("y"), None);
    //         assert_eq!(parse_value_unit_only("y5"), None);
    //     }

    //     #[test]
    //     fn test_parse_cookie() {
    //         assert_eq!(parse_cookie(""), None);
    //         assert_eq!(parse_cookie("-1"), None);
    //         assert_eq!(parse_cookie("1"), None);
    //         assert_eq!(parse_cookie("+"), None);
    //         assert_eq!(parse_cookie("1d"), None);
    //         assert_eq!(
    //             parse_cookie("+1d"),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::Cumulate,
    //                     habit: None,
    //                     value: 1,
    //                     unit: Unit::Day
    //                 }),
    //                 ""
    //             ))
    //         );
    //         assert_eq!(
    //             parse_cookie("++2w -3d"),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::CatchUp,
    //                     habit: None,
    //                     value: 2,
    //                     unit: Unit::Week
    //                 }),
    //                 "-3d"
    //             ))
    //         );
    //         assert_eq!(
    //             parse_cookie(".+2w hello"),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::Restart,
    //                     habit: None,
    //                     value: 2,
    //                     unit: Unit::Week
    //                 }),
    //                 "hello"
    //             ))
    //         );
    //         assert_eq!(
    //             parse_cookie("+1m/2d"),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::Cumulate,
    //                     habit: Some((2, Unit::Day)),
    //                     value: 1,
    //                     unit: Unit::Month
    //                 }),
    //                 ""
    //             ))
    //         );
    //         assert_eq!(
    //             parse_cookie("++2w/3y"),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::CatchUp,
    //                     habit: Some((3, Unit::Year)),
    //                     value: 2,
    //                     unit: Unit::Week
    //                 }),
    //                 ""
    //             ))
    //         );
    //         assert_eq!(
    //             parse_cookie(".+3w/4w"),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::Restart,
    //                     habit: Some((4, Unit::Week)),
    //                     value: 3,
    //                     unit: Unit::Week
    //                 }),
    //                 ""
    //             ))
    //         );
    //         assert_eq!(
    //             parse_cookie(".+3w/4w "),
    //             Some((
    //                 Ok(Repeater {
    //                     mark: RepeaterMark::Restart,
    //                     habit: Some((4, Unit::Week)),
    //                     value: 3,
    //                     unit: Unit::Week
    //                 }),
    //                 ""
    //             ))
    //         );
    //     }

    //     #[test]
    //     fn test_parse_cookies_only() {
    //         assert_eq!(parse_cookies_only(""), Some(RepeaterAndDelay::default()));
    //         assert_eq!(parse_cookies_only("5"), None);
    //         assert_eq!(parse_cookies_only("-"), None);
    //         assert_eq!(parse_cookies_only(" "), Some(RepeaterAndDelay::default()));

    //         let repeater = Some(Repeater {
    //             mark: RepeaterMark::Restart,
    //             value: 5,
    //             habit: None,
    //             unit: Unit::Day,
    //         });
    //         let delay = Some(Delay {
    //             mark: DelayMark::All,
    //             value: 7,
    //             unit: Unit::Week,
    //         });

    //         assert_eq!(
    //             parse_cookies_only(".+5d"),
    //             Some(RepeaterAndDelay {
    //                 repeater,
    //                 delay: None
    //             })
    //         );
    //         assert_eq!(
    //             parse_cookies_only(".+5d "),
    //             Some(RepeaterAndDelay {
    //                 repeater,
    //                 delay: None
    //             })
    //         );
    //         assert_eq!(
    //             parse_cookies_only("-7w"),
    //             Some(RepeaterAndDelay {
    //                 repeater: None,
    //                 delay
    //             })
    //         );
    //     }

    //     #[test]
    //     fn test_parse() {
    //         let date = NaiveDate::from_ymd(2020, 1, 1).into();
    //         let time = Some(NaiveTime::from_hms(13, 29, 0).into());
    //         let end_time = NaiveTime::from_hms(19, 27, 0).into();
    //         let times = Times(time.unwrap(), end_time);
    //         let cookie = RepeaterAndDelay::default();
    //         let active = Activity::Active;
    //         let inactive = Activity::Inactive;
    //         let start = Point {
    //             date,
    //             time,
    //             cookie: RepeaterAndDelay {
    //                 delay: None,
    //                 repeater: Some(Repeater {
    //                     mark: RepeaterMark::Restart,
    //                     habit: None,
    //                     unit: Unit::Day,
    //                     value: 1,
    //                 }),
    //             },
    //         };
    //         let end = Point {
    //             date: NaiveDate::from_ymd(2020, 1, 8).into(),
    //             time: None,
    //             cookie: RepeaterAndDelay {
    //                 delay: None,
    //                 repeater: Some(Repeater {
    //                     habit: None,
    //                     mark: RepeaterMark::Cumulate,
    //                     unit: Unit::Day,
    //                     value: 2,
    //                 }),
    //             },
    //         };
    //         assert_eq!(Timestamp::try_from("<2020-01-01 Mon 13:29]"), Err(()));
    //         assert_eq!(
    //             Timestamp::try_from("<2020-01-01 Mon 13:29>"),
    //             Ok(Timestamp::Point(active, Point { date, time, cookie }))
    //         );
    //         assert_eq!(
    //             Timestamp::try_from("<2020-01-01 Mon 13:29 .+1d>--<2020-01-08 Mon +2d>"),
    //             Ok(Timestamp::Range(active, Range { start, end }))
    //         );
    //         assert_eq!(
    //             Timestamp::try_from("[2020-01-01 Mon 13:29-15:37 .+1d]--[2020-01-08 Mon +2d]"),
    //             Err(())
    //         );
    //         assert_eq!(
    //             Timestamp::try_from("[2020-01-01 Mon 13:29 .+1d]--[2020-01-08 Mon +2d]"),
    //             Ok(Timestamp::Range(inactive, Range { start, end }))
    //         );
    //         assert_eq!(
    //             Timestamp::try_from("[2020-01-01 Mon 13:29 .+1d]-[2020-01-08 Mon +2d]"),
    //             Err(())
    //         );
    //         assert_eq!(
    //             Timestamp::try_from("[2020-01-01 Mon 13:29 .+1d>--[2020-01-08 Mon +2d]"),
    //             Err(())
    //         );
    //         assert_eq!(
    //             "<%%(diary-date 5 5 2005)>".try_into(),
    //             Ok(Timestamp::Diary(Diary(Cow::Borrowed(
    //                 "diary-date 5 5 2005"
    //             ))))
    //         );
    //         assert_eq!(Timestamp::try_from("<%%diary-date 5 5 2005>"), Err(()));
    //         assert_eq!(Timestamp::try_from("<%(diary-date 5 5 2005)>"), Err(()));
    //         assert_eq!(Timestamp::try_from("<%%(diary-date 5 5 2005)]"), Err(()));
    //         assert_eq!(Timestamp::try_from("[%%(diary-date 5 5 2005)]"), Err(()));
    //         assert_eq!(Timestamp::try_from("[2020-01-01 Mon 13:29-19:27>"), Err(()));
    //         assert_eq!(
    //             "[2020-01-01 Mon 13:29-19:27]".try_into(),
    //             Ok(Timestamp::TimeRange(
    //                 inactive,
    //                 TimeRange {
    //                     times,
    //                     date,
    //                     cookie
    //                 }
    //             ))
    //         );
    //     }
}
