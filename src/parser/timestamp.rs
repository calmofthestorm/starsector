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

fn parse_dayname(input: &str) -> IResult<&str, &str, ()> {
    let (input, dayname) = verify(
        take_while1(|c: char| !c.is_whitespace() && c != '>' && c != ']'),
        |dayname: &str| {
            !dayname
                .chars()
                .any(|c| c.is_ascii_digit() || c == '+' || c == '-')
        },
    )(input)?;
    Ok((input, dayname))
}

impl Time {
    pub fn parse(input: &str) -> IResult<&str, Time, ()> {
        let (input, hour) = parse_integer_1_2(input)?;
        let (input, _) = tag(":")(input)?;
        let (input, minute) = parse_integer_2(input)?;
        let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0)
            .ok_or_else(|| Err::Error(make_error(input, ErrorKind::Verify)))?;
        Ok((input, time.into()))
    }
}

impl Times {
    pub fn parse(input: &str) -> IResult<&str, Times, ()> {
        let (input, (start, end)) = separated_pair(Time::parse, char('-'), Time::parse)(input)?;
        Ok((input, Times(start, end)))
    }
}

impl TimeSpec {
    pub fn parse(input: &str) -> IResult<&str, TimeSpec, ()> {
        let (input, start) = Time::parse(input)?;
        match opt(preceded(char('-'), Time::parse))(input) {
            Ok((input, Some(end))) => Ok((input, TimeSpec::Times(Times(start, end)))),
            _ => Ok((input, TimeSpec::Time(start))),
        }
    }
}

impl RepeaterMark {
    pub fn parse(input: &str) -> IResult<&str, RepeaterMark, ()> {
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
    pub fn parse(input: &str) -> IResult<&str, DelayMark, ()> {
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
    pub fn parse(input: &str) -> IResult<&str, TimeUnit, ()> {
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
    pub fn parse(input: &str) -> IResult<&str, Interval, ()> {
        let (input, value) = map_res(digit1, |num| usize::from_str_radix(num, 10))(input)?;
        let (input, unit) = TimeUnit::parse(input)?;
        Ok((input, Interval::new(value, unit)))
    }
}

impl Repeater {
    pub fn parse(input: &str) -> IResult<&str, Repeater, ()> {
        let (input, mark) = RepeaterMark::parse(input)?;
        let (input, interval) = Interval::parse(input)?;
        Ok((input, Repeater { mark, interval }))
    }
}

impl Delay {
    pub fn parse(input: &str) -> IResult<&str, Delay, ()> {
        let (input, mark) = DelayMark::parse(input)?;
        let (input, interval) = Interval::parse(input)?;
        Ok((input, Delay { mark, interval }))
    }
}

impl RepeaterAndDelay {
    pub fn parse(input: &str) -> IResult<&str, RepeaterAndDelay, ()> {
        let (input, repeater, delay) = if let Ok((input, (repeater, delay))) =
            pair(Repeater::parse, preceded(space1, Delay::parse))(input)
        {
            (input, Some(repeater), Some(delay))
        } else if let Ok((input, (delay, repeater))) =
            pair(Delay::parse, preceded(space1, Repeater::parse))(input)
        {
            (input, Some(repeater), Some(delay))
        } else if let Ok((input, delay)) = Delay::parse(input) {
            (input, None, Some(delay))
        } else if let Ok((input, repeater)) = Repeater::parse(input) {
            (input, Some(repeater), None)
        } else {
            (input, None, None)
        };
        Ok((input, RepeaterAndDelay { repeater, delay }))
    }
}

impl Date {
    pub fn parse(input: &str) -> IResult<&str, Date, ()> {
        let (input, (year, month, day)) = tuple((
            parse_integer_4,
            preceded(char('-'), parse_integer_2),
            preceded(char('-'), parse_integer_2),
        ))(input)?;
        let (input, _dayname) = opt(preceded(space1, parse_dayname))(input)?;
        Ok((
            input,
            NaiveDate::from_ymd(year as i32, month as u32, day as u32).into(),
        ))
    }
}

fn parse_atomic_timestamp(input: &str) -> IResult<&str, (Point, Option<Time>), ()> {
    // Annoying, but we want to allow RepeaterAndDelay to be parsed in
    // isolation, but also to be empty, and it needs a leading space iff
    // non-empty.
    let inner = |active: Activity| {
        map(
            tuple((
                Date::parse,
                opt(preceded(space1, TimeSpec::parse)),
                terminated(
                    alt((
                        verify(preceded(space1, RepeaterAndDelay::parse), |rad| {
                            rad.repeater.is_some() || rad.delay.is_some()
                        }),
                        verify(RepeaterAndDelay::parse, |rad| {
                            rad.repeater.is_none() && rad.delay.is_none()
                        }),
                    )),
                    opt(is_not(">]\n")),
                ),
            )),
            move |(date, time, cookie)| {
                let (start, end) = match time {
                    None => (None, None),
                    Some(TimeSpec::Time(start)) => (Some(start), None),
                    Some(TimeSpec::Times(Times(start, end))) => (Some(start), Some(end)),
                };
                (
                    Point {
                        active,
                        date,
                        cookie,
                        time: start,
                    },
                    end,
                )
            },
        )
    };

    terminated(
        alt((
            preceded(tag("<"), inner(Activity::Active)),
            preceded(tag("["), inner(Activity::Inactive)),
        )),
        one_of("]>"),
    )(input)
}

impl Point {
    pub fn parse(input: &str) -> IResult<&str, Point, ()> {
        let (input, (point, _none)) = verify(parse_atomic_timestamp, |(_, e)| e.is_none())(input)?;
        Ok((input, point))
    }
}

impl Range {
    pub fn parse(input: &str) -> IResult<&str, Range, ()> {
        let (input, (start, mut end)) =
            separated_pair(Point::parse, tag("--"), Point::parse)(input)?;
        end.active = start.active;
        Ok((input, Range { start, end }))
    }
}

impl TimeRange {
    pub fn parse(input: &str) -> IResult<&str, TimeRange, ()> {
        let (input, (start, end_time)) =
            verify(parse_atomic_timestamp, |(_, e)| e.is_some())(input)?;
        let end_time = end_time.expect("verified");
        Ok((input, TimeRange { start, end_time }))
    }
}

impl Diary<'_> {
    pub fn parse<'a>(input: &'a str) -> IResult<&'a str, Diary<'a>, ()> {
        map(
            verify(
                delimited(tag("<%%("), is_not("\n>"), char('>')),
                |d: &str| d.ends_with(')'),
            ),
            |diary: &str| Diary(diary[..diary.len() - 1].into()),
        )(input)
    }
}

impl Timestamp<'_> {
    pub fn parse(input: &str) -> IResult<&str, Timestamp<'_>, ()> {
        alt((
            map(Diary::parse, |d| Timestamp::Diary(d)),
            map(Range::parse, |r| Timestamp::Range(r)),
            map(TimeRange::parse, |r| Timestamp::TimeRange(r)),
            map(Point::parse, |p| Timestamp::Point(p)),
        ))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(Repeater::parse(*bad).is_err());
        }
    }

    #[test]
    fn test_parse_delay() {
        let delay = |m: &str, i: &str| {
            Delay::new(
                DelayMark::parse(m).unwrap().1,
                Interval::parse(i).unwrap().1,
            )
        };
        assert_eq!(Delay::parse("-5h").unwrap().1, delay("-", "5h"));
        assert_eq!(Delay::parse("--7y").unwrap().1, delay("--", "7y"));
        assert_eq!(Delay::parse("--0m").unwrap().1, delay("--", "0m"));

        let res = Delay::parse("-6h ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(Delay::parse("-6h").unwrap().1, res.1);

        for bad in &["---5w", "", "+7m"] {
            assert!(Delay::parse(*bad).is_err());
        }
    }

    #[test]
    fn test_parse_repeater_and_delay() {
        let repeater = Some(Repeater::new(
            RepeaterMark::Cumulate,
            Interval::new(5, TimeUnit::Day),
        ));
        let delay = Some(Delay::new(
            DelayMark::First,
            Interval::new(7, TimeUnit::Week),
        ));

        assert_eq!(
            RepeaterAndDelay::parse("+5d --7w").unwrap().1,
            RepeaterAndDelay { repeater, delay }
        );

        assert_eq!(
            RepeaterAndDelay::parse("--7w +5d").unwrap().1,
            RepeaterAndDelay { repeater, delay }
        );

        assert_eq!(
            RepeaterAndDelay::parse("--7w \t+5d").unwrap().1,
            RepeaterAndDelay { repeater, delay }
        );

        assert_eq!(
            RepeaterAndDelay::parse("+5d").unwrap().1,
            RepeaterAndDelay {
                repeater,
                delay: None
            }
        );

        assert_eq!(
            RepeaterAndDelay::parse("--7w").unwrap().1,
            RepeaterAndDelay {
                repeater: None,
                delay
            }
        );

        assert_eq!(
            RepeaterAndDelay::parse("").unwrap().1,
            RepeaterAndDelay {
                repeater: None,
                delay: None
            }
        );

        let res = RepeaterAndDelay::parse("---5w").unwrap();
        assert_eq!("---5w", res.0);
        assert_eq!(RepeaterAndDelay::parse("").unwrap().1, res.1);

        let res = RepeaterAndDelay::parse("-6h ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(RepeaterAndDelay::parse("-6h").unwrap().1, res.1);

        let res = RepeaterAndDelay::parse("+7m ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(RepeaterAndDelay::parse("+7m").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_date() {
        let date = |y, m, d| Date::new(y, m, d);

        assert_eq!(Date::parse("2020-01-10").unwrap().1, date(2020, 1, 10));
        assert_eq!(Date::parse("2020-01-10 Fri").unwrap().1, date(2020, 1, 10));
        assert_eq!(Date::parse("2020-01-10 Sat").unwrap().1, date(2020, 1, 10));
        assert_eq!(
            Date::parse("2020-01-10  Zeepsday").unwrap().1,
            date(2020, 1, 10)
        );
        assert_eq!(Date::parse("2020-02-17 Zee").unwrap().1, date(2020, 2, 17));

        assert_eq!(Date::parse("0020-01-10").unwrap().1, date(0020, 1, 10));

        for bad in &[
            " 2020-02-02",
            "2020",
            "",
            "20200110",
            "20200110 3:14",
            "5",
            "202-05-05",
            "-1986-08-24",
            "1987-5-29",
            "1987-03-1",
        ] {
            assert!(Date::parse(*bad).is_err());
        }

        let res = Date::parse("2020-02-02 ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(Date::parse("2020-02-02").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_point() {
        let point = Point::new(Date::new(2020, 1, 1));

        assert_eq!(Point::parse("<2020-01-01>").unwrap().1, point);
        assert_eq!(Point::parse("<2020-01-01   Mon>").unwrap().1, point);
        assert_eq!(
            Point::parse("[2020-01-01   Mon 03:57  --1d .+1w]")
                .unwrap()
                .1,
            point
                .with_repeater(Some(Repeater::new(
                    RepeaterMark::Restart,
                    Interval::new(1, TimeUnit::Week)
                )))
                .with_delay(Some(Delay::new(
                    DelayMark::First,
                    Interval::new(1, TimeUnit::Day)
                )))
                .with_active(false)
                .with_time(Some(Time::new(3, 57)))
        );

        for bad in &["", "2020-01-01", "<%%(hi)>", "[2020-01-01 01:00-02:00]"] {
            eprintln!("This {}", bad);
            assert!(Point::parse(*bad).is_err());
        }

        let res = Point::parse("<2020-01-01> ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(Point::parse("<2020-01-01>").unwrap().1, res.1);

        let res = Point::parse("<2020-01-01>--<2020-02-01>").unwrap();
        assert_eq!("--<2020-02-01>", res.0);
        assert_eq!(Point::parse("<2020-01-01>").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_range() {
        let timestamp = Range::new(
            Point::parse("[2020-01-01]").unwrap().1,
            Point::parse("[2021-01-01]").unwrap().1,
        );

        assert_eq!(
            Range::parse("[2020-01-01]--[2021-01-01]").unwrap().1,
            timestamp
        );
        assert_eq!(
            Range::parse("[2020-01-01    04:59 .+1w]--[2021-01-01 .+2d]")
                .unwrap()
                .1,
            Range {
                start: timestamp
                    .start
                    .with_time(Some(Time::new(4, 59)))
                    .with_repeater(Some(Repeater::parse(".+1w").unwrap().1)),
                end: timestamp
                    .end
                    .with_repeater(Some(Repeater::parse(".+2d").unwrap().1))
            }
        );

        for bad in &[
            "",
            "<2020-01-01>",
            "2020-01-01--2021-01-01",
            "<%%(hi)>",
            "[2020-01-01 01:00-02:00]",
        ] {
            assert!(Range::parse(*bad).is_err());
        }

        let res = Range::parse("<2020-01-01>--<2020-02-01> ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(Range::parse("<2020-01-01>--<2020-02-01>").unwrap().1, res.1);
    }

    #[test]
    fn test_parse_time_range() {
        let timestamp = TimeRange::new(
            Point::parse("[2020-01-01 03:53]").unwrap().1,
            Time::parse("05:57").unwrap().1,
        );

        assert_eq!(
            TimeRange::parse("[2020-01-01 03:53-05:57]").unwrap().1,
            timestamp
        );

        assert_eq!(
            TimeRange::parse("[2020-01-01    04:58-04:59 .+1w]")
                .unwrap()
                .1,
            TimeRange {
                start: timestamp
                    .start
                    .with_time(Some(Time::new(4, 58)))
                    .with_repeater(Some(Repeater::parse(".+1w").unwrap().1)),
                end_time: Time::new(4, 59)
            }
        );

        for bad in &["", "<2020-01-01>", "2020-01-01--2021-01-01", "<%%(hi)>"] {
            assert!(TimeRange::parse(*bad).is_err());
        }

        let res = TimeRange::parse("<2020-01-01 09:00-10:00> ").unwrap();
        assert_eq!(" ", res.0);
        assert_eq!(
            TimeRange::parse("<2020-01-01 09:00-10:00>").unwrap().1,
            res.1
        );
    }

    #[test]
    fn test_parse_diary() {
        assert_eq!(Diary::parse("<%%()>").unwrap().1, Diary("".into()));
        assert_eq!(
            Diary::parse("<%%(diary-date 2020 3 1)>").unwrap().1,
            Diary("diary-date 2020 3 1".into())
        );
    }

    // Most of the tests assume inner types are correctly implemented,
    // however because there is so much overlap between the different
    // timestamp forms, we do the thorough tests here and only basic tests
    // in each form's test.
    mod timestamp {
        use super::*;

        #[test]
        fn test_parse_timestamp_point() {
            let date = |y, m, d| Date::new(y, m, d);
            let point = Point::new(date(2020, 3, 1));

            assert_eq!(Point::parse("<2020-03-01>").unwrap().1, point);
            assert_eq!(
                Point::parse("[2020-03-01]").unwrap().1,
                point.with_active(false)
            );
            assert_eq!(
                Point::parse("[2020-03-01 Wed]").unwrap().1,
                point.with_active(false)
            );
            assert_eq!(Point::parse("<2020-03-01 Zee>").unwrap().1, point);
            assert_eq!(Point::parse("<2020-03-01 >").unwrap().1, point);
            assert_eq!(Point::parse("<2020-03-01 \t>").unwrap().1, point);
            let time = NaiveTime::from_hms(3, 59, 0);
            assert_eq!(
                Point::parse("<2020-03-01 \tFri  3:59>").unwrap().1,
                point.with_time(Some(time))
            );
            assert_eq!(
                Point::parse("<2020-03-01 \tFri  3:59>").unwrap().1,
                point.with_time(Some(time))
            );

            assert_eq!(
                Point::parse("<2020-03-01 3:59\t \t \t>").unwrap().1,
                point.with_time(Some(Time::new(3, 59)))
            );

            assert_eq!(
                Point::parse("[2020-03-01 .+1w]").unwrap().1,
                point
                    .with_repeater(Some(Repeater::parse(".+1w").unwrap().1))
                    .with_active(false)
            );

            assert_eq!(
                Point::parse("<2020-03-01   \t-1d\t  >").unwrap().1,
                point.with_delay(Some(Delay::parse("-1d").unwrap().1))
            );

            assert_eq!(
                Point::parse("<2020-03-01   \t-1d\t  .+1d/1w  >").unwrap().1,
                point
                    .with_repeater(Some(Repeater::parse(".+1d").unwrap().1))
                    .with_delay(Some(Delay::parse("-1d").unwrap().1))
            );
            assert_eq!(
                Point::parse("<2020-03-01 arbitrary text .+1d --2w here 夫妻肺片]")
                    .unwrap()
                    .1,
                point
            );
            assert_eq!(
                Point::parse("<2020-03-01 Fri .+1d .+1d>").unwrap().1,
                point.with_repeater(Some(Repeater::parse(".+1d").unwrap().1))
            );
            assert_eq!(
                Point::parse("[2020-03-01>").unwrap().1,
                point.with_active(false)
            );

            for bad in &["2020-01-01", ""] {
                assert!(Timestamp::parse(*bad).is_err())
            }

            let res = Point::parse("<2020-01-01>>").unwrap();
            assert_eq!(">", res.0);
            assert_eq!(Point::parse("<2020-01-01>").unwrap().1, res.1);

            let res = Point::parse("<2020-01-01> ").unwrap();
            assert_eq!(" ", res.0);
            assert_eq!(Point::parse("<2020-01-01>").unwrap().1, res.1);

            // Org accepts these, thus, so do we:/ Opening determine activity.
            let active = Point::parse("<2020-01-02>").unwrap().1;
            let inactive = Point::parse("[2020-01-02]").unwrap().1;

            let res = Point::parse("<2020-01-02]>").unwrap();
            assert_eq!(">", res.0);
            assert_eq!(active, res.1);

            let res = Point::parse("<2020-01-02>]").unwrap();
            assert_eq!("]", res.0);
            assert_eq!(active, res.1);

            let res = Point::parse("[2020-01-02]>").unwrap();
            assert_eq!(">", res.0);
            assert_eq!(inactive, res.1);

            let res = Point::parse("[2020-01-02>]").unwrap();
            assert_eq!("]", res.0);
            assert_eq!(inactive, res.1);

            let res = Point::parse("<2020-01-02]").unwrap();
            assert_eq!("", res.0);
            assert_eq!(active, res.1);

            let res = Point::parse("<2020-01-02>").unwrap();
            assert_eq!("", res.0);
            assert_eq!(active, res.1);

            let res = Point::parse("[2020-01-02]").unwrap();
            assert_eq!("", res.0);
            assert_eq!(inactive, res.1);

            let res = Point::parse("[2020-01-02>").unwrap();
            assert_eq!("", res.0);
            assert_eq!(inactive, res.1);
        }

        #[test]
        fn test_parse_timestamp_diary() {
            assert_eq!(
                Timestamp::parse("<%%(anything goes here but newline and closy angle bracket)>")
                    .unwrap()
                    .1,
                Timestamp::Diary(Diary(
                    "anything goes here but newline and closy angle bracket".into()
                ))
            );

            assert_eq!(
                Timestamp::parse("<%%())>").unwrap().1,
                Timestamp::Diary(Diary(")".into()))
            );

            assert_eq!(
                Timestamp::parse("<%%([2020-01-01])>").unwrap().1,
                Timestamp::Diary(Diary("[2020-01-01]".into()))
            );

            assert!(Timestamp::parse("<%%(<2020-01-01>)>").is_err())
        }
    }
}
