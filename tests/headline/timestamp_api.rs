use std::convert::TryFrom;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use starsector::*;

#[test]
fn timestamp_builders_and_accessors_work() {
    let active = Activity::new(true);
    let inactive = Activity::new(false);
    assert!(active.is_active());
    assert_eq!(active.delimiters(), ('<', '>'));
    assert_eq!(inactive.delimiters(), ('[', ']'));

    let interval = Interval::new(3, TimeUnit::Day);
    assert_eq!(interval.value(), 3);
    assert_eq!(interval.unit(), TimeUnit::Day);
    assert_eq!(interval.with_value(4).value(), 4);
    assert_eq!(interval.with_unit(TimeUnit::Week).unit(), TimeUnit::Week);

    let repeater = Repeater::new(RepeaterMark::Cumulate, interval)
        .with_mark(RepeaterMark::Restart)
        .with_value(5)
        .with_unit(TimeUnit::Month);
    assert_eq!(repeater.mark(), RepeaterMark::Restart);
    assert_eq!(repeater.value(), 5);
    assert_eq!(repeater.unit(), TimeUnit::Month);

    let delay = Delay::new(DelayMark::All, interval)
        .with_mark(DelayMark::First)
        .with_value(6)
        .with_unit(TimeUnit::Year);
    assert_eq!(delay.mark(), DelayMark::First);
    assert_eq!(delay.value(), 6);
    assert_eq!(delay.unit(), TimeUnit::Year);

    let cookie = RepeaterAndDelay::new(None, None)
        .with_repeater(Some(repeater))
        .with_delay(Some(delay));
    assert_eq!(cookie, RepeaterAndDelay::new(Some(repeater), Some(delay)));

    let t = Time::new(9, 18);
    assert_eq!(t.hour(), 9);
    assert_eq!(t.minute(), 18);
    assert_eq!(t.with_hour(10), Time::new(10, 18));
    assert_eq!(t.with_minute(42), Time::new(9, 42));

    let times = Times::try_from(TimeSpec::parse("08:00-11:30").unwrap().1).unwrap();
    assert_eq!(times.start(), Time::new(8, 0));
    assert_eq!(times.end(), Time::new(11, 30));
    assert_eq!(times.with_start(Time::new(7, 45)).start(), Time::new(7, 45));
    assert_eq!(times.with_end(Time::new(12, 0)).end(), Time::new(12, 0));

    let spec = TimeSpec::Times(times);
    assert_eq!(spec.start(), Time::new(8, 0));
    assert_eq!(spec.end(), Some(Time::new(11, 30)));
    assert_eq!(TimeSpec::Time(Time::new(6, 0)).end(), None);

    let point = Point::new(Date::new(2024, 1, 2))
        .with_active(inactive)
        .with_time(Some(t))
        .with_cookie(cookie);
    assert_eq!(point.active(), Activity::Inactive);
    assert_eq!(point.date(), Some(Date::new(2024, 1, 2)));
    assert_eq!(point.time(), Some(Time::new(9, 18)));
    assert_eq!(point.cookie(), Some(cookie));
}

#[test]
fn timestamp_ext_overlap_and_start_end_behavior() {
    let p1 = Point::new(Date::new(2024, 1, 1)).with_time(Some(Time::new(10, 0)));
    let p2 = Point::new(Date::new(2024, 1, 1)).with_time(Some(Time::new(11, 0)));
    let p3 = Point::new(Date::new(2024, 1, 2));

    assert!(p1.date_overlap(&p2));
    assert!(!p1.date_overlap(&p3));

    let r = Range::new(
        Point::new(Date::new(2024, 1, 1)),
        Point::new(Date::new(2024, 1, 3)),
    );
    assert!(r.date_overlap(&p2));
    assert!(r.date_overlap(&p3));
    assert_eq!(
        r.start_timestamp(),
        Some(Timestamp::Point(Point::new(Date::new(2024, 1, 1))))
    );
    assert_eq!(
        r.end_timestamp(),
        Some(Timestamp::Point(Point::new(Date::new(2024, 1, 3))))
    );

    let tr = TimeRange::new(
        Point::new(Date::new(2024, 1, 1)).with_time(Some(Time::new(8, 0))),
        Time::new(9, 0),
    );
    assert!(tr.date_overlap(&p1));
    assert_eq!(tr.start_time(), Some(Time::new(8, 0)));
    assert_eq!(tr.end_time(), Some(Time::new(9, 0)));

    let diary = Diary::new("my-diary");
    assert!(!diary.date_overlap(&p1));
    assert_eq!(diary.start_timestamp(), None);
    assert_eq!(diary.end_timestamp(), None);
}

#[test]
fn timestamp_conversions_and_roundtrips_work() {
    let start = Point::new(Date::new(2024, 6, 1)).with_time(Some(Time::new(9, 0)));
    let tr = TimeRange::new(start, Time::new(10, 30));

    let r: Range = tr.into();
    assert_eq!(r.start_time(), Some(Time::new(9, 0)));
    assert_eq!(r.end_time(), Some(Time::new(10, 30)));

    let tr2 = TimeRange::try_from(&r).unwrap();
    assert_eq!(tr2, TimeRange::new(start, Time::new(10, 30)));

    let ts_point: Timestamp<'static> = start.into();
    assert_eq!(Point::try_from(ts_point.clone()).unwrap(), start);
    assert!(Range::try_from(ts_point.clone()).is_err());
    assert!(TimeRange::try_from(ts_point).is_err());

    let ts_range: Timestamp<'static> = Timestamp::Range(r);
    assert_eq!(Range::try_from(ts_range.clone()).unwrap(), r);
    assert!(TimeRange::try_from(ts_range).is_err());
    assert_eq!(TimeRange::try_from(&r).unwrap(), tr2);

    let d = Diary::new("x");
    let ts_diary = Timestamp::Diary(d.to_borrowed());
    assert!(Diary::try_from(&ts_diary).is_ok());
    assert!(Point::try_from(ts_diary).is_err());

    let owned = d.into_owned();
    assert_eq!(owned.to_borrowed(), owned);
}

#[test]
fn timestamp_try_from_error_paths_are_covered() {
    let point = Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(9, 0)));
    let range = Range::new(
        Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(9, 0))),
        Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(10, 0))),
    );
    let time_range = TimeRange::new(point, Time::new(10, 0));
    let diary = Diary::new("d").into_owned();

    let ts_point = Timestamp::Point(point);
    let ts_range = Timestamp::Range(range);
    let ts_time_range = Timestamp::TimeRange(time_range);
    let ts_diary = Timestamp::Diary(diary.to_borrowed());

    assert!(Diary::try_from(ts_point.clone()).is_err());
    assert!(Diary::try_from(ts_range.clone()).is_err());
    assert!(Diary::try_from(ts_time_range.clone()).is_err());

    assert!(Point::try_from(ts_range.clone()).is_err());
    assert!(Point::try_from(ts_time_range.clone()).is_err());
    assert!(Point::try_from(ts_diary.clone()).is_err());

    assert!(Range::try_from(ts_point.clone()).is_err());
    assert!(Range::try_from(ts_time_range.clone()).is_err());
    assert!(Range::try_from(ts_diary.clone()).is_err());

    assert!(TimeRange::try_from(ts_point).is_err());
    assert!(TimeRange::try_from(ts_range).is_err());
    assert!(TimeRange::try_from(ts_diary).is_err());

    let wrong_date = Range::new(
        Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(9, 0))),
        Point::new(Date::new(2024, 7, 2)).with_time(Some(Time::new(10, 0))),
    );
    assert!(TimeRange::try_from(&wrong_date).is_err());

    let missing_start_time = Range::new(
        Point::new(Date::new(2024, 7, 1)),
        Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(10, 0))),
    );
    assert!(TimeRange::try_from(&missing_start_time).is_err());

    let missing_end_time = Range::new(
        Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(9, 0))),
        Point::new(Date::new(2024, 7, 1)),
    );
    assert!(TimeRange::try_from(&missing_end_time).is_err());

    let different_cookie = Range::new(
        Point::new(Date::new(2024, 7, 1))
            .with_time(Some(Time::new(9, 0)))
            .with_repeater(Some(Repeater::new(
                RepeaterMark::Cumulate,
                Interval::new(1, TimeUnit::Day),
            ))),
        Point::new(Date::new(2024, 7, 1)).with_time(Some(Time::new(10, 0))),
    );
    assert!(TimeRange::try_from(&different_cookie).is_err());
}

#[test]
fn timestamp_ext_accessors_cover_all_variants() {
    let cookie = RepeaterAndDelay::new(
        Some(Repeater::new(
            RepeaterMark::CatchUp,
            Interval::new(2, TimeUnit::Week),
        )),
        Some(Delay::new(
            DelayMark::First,
            Interval::new(1, TimeUnit::Day),
        )),
    );
    let point = Point::new(Date::new(2024, 8, 2))
        .with_active(Activity::Inactive)
        .with_time(Some(Time::new(9, 30)))
        .with_cookie(cookie);
    let range = Range::new(
        point,
        Point::new(Date::new(2024, 8, 3))
            .with_active(Activity::Inactive)
            .with_time(Some(Time::new(11, 0)))
            .with_cookie(cookie),
    );
    let time_range = TimeRange::new(point, Time::new(10, 45));
    let diary = Diary::new("diary").into_owned();

    let ts_point = Timestamp::Point(point);
    assert_eq!(ts_point.start_point(), Some(point));
    assert_eq!(ts_point.end_point(), None);
    assert_eq!(ts_point.start_time(), Some(Time::new(9, 30)));
    assert_eq!(ts_point.end_time(), None);
    assert_eq!(ts_point.active(), Activity::Inactive);
    assert_eq!(ts_point.date(), Some(Date::new(2024, 8, 2)));
    assert_eq!(ts_point.time(), Some(Time::new(9, 30)));
    assert_eq!(ts_point.cookie(), Some(cookie));

    let ts_range = Timestamp::Range(range);
    assert_eq!(ts_range.start_point(), Some(range.start_point().unwrap()));
    assert_eq!(ts_range.end_point(), Some(range.end_point().unwrap()));
    assert_eq!(ts_range.start_time(), Some(Time::new(9, 30)));
    assert_eq!(ts_range.end_time(), Some(Time::new(11, 0)));
    assert_eq!(ts_range.active(), Activity::Inactive);
    assert_eq!(ts_range.date(), None);
    assert_eq!(ts_range.time(), None);
    assert_eq!(ts_range.cookie(), None);

    let ts_time_range = Timestamp::TimeRange(time_range);
    assert_eq!(ts_time_range.start_point(), Some(point));
    assert_eq!(
        ts_time_range.end_point().unwrap().time(),
        Some(Time::new(10, 45))
    );
    assert_eq!(ts_time_range.start_time(), Some(Time::new(9, 30)));
    assert_eq!(ts_time_range.end_time(), Some(Time::new(10, 45)));
    assert_eq!(ts_time_range.date(), Some(Date::new(2024, 8, 2)));
    assert_eq!(ts_time_range.time(), None);
    assert_eq!(ts_time_range.cookie(), Some(cookie));

    let ts_diary = Timestamp::Diary(diary.to_borrowed());
    assert_eq!(ts_diary.start_point(), None);
    assert_eq!(ts_diary.end_point(), None);
    assert_eq!(ts_diary.start_time(), None);
    assert_eq!(ts_diary.end_time(), None);
    assert_eq!(ts_diary.active(), Activity::Active);
    assert_eq!(ts_diary.date(), None);
    assert_eq!(ts_diary.time(), None);
    assert_eq!(ts_diary.cookie(), None);
}

#[test]
fn timestamp_conversion_impls_and_defaults_are_covered() {
    let active: bool = Activity::Active.into();
    let inactive: bool = Activity::Inactive.into();
    assert!(active);
    assert!(!inactive);
    assert_eq!(Activity::from(true), Activity::Active);
    assert_eq!(Activity::from(false), Activity::Inactive);

    assert_eq!(Time::default(), Time::new(0, 0));
    assert_eq!(TimeSpec::default(), TimeSpec::Time(Time::new(0, 0)));

    let ns = NaiveTime::from_hms_opt(7, 45, 59).unwrap();
    assert_eq!(Time::from(ns), Time::new(7, 45));
    assert_eq!(NaiveTime::from(Time::new(7, 45)), Time::new(7, 45).into());

    let nd = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
    let point_from_date: Point = nd.into();
    assert_eq!(point_from_date.date(), Some(Date::new(2024, 9, 1)));
    let ts_from_date: Timestamp<'static> = nd.into();
    assert_eq!(Point::try_from(ts_from_date).unwrap(), point_from_date);

    let ndt = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 9, 2).unwrap(),
        NaiveTime::from_hms_opt(6, 5, 4).unwrap(),
    );
    let point_from_dt: Point = ndt.into();
    assert_eq!(point_from_dt.date(), Some(Date::new(2024, 9, 2)));
    assert_eq!(point_from_dt.time(), Some(Time::new(6, 5)));

    assert_eq!(char::from(TimeUnit::Day), 'd');
    assert_eq!(RepeaterMark::Restart.to_string(), ".+");
    assert_eq!(DelayMark::First.to_string(), "--");

    assert_eq!(
        Time::try_from(TimeSpec::Time(Time::new(1, 2))).unwrap(),
        Time::new(1, 2)
    );
    let spec_times = TimeSpec::parse("01:02-03:04").unwrap().1;
    assert!(Time::try_from(spec_times).is_err());
    let times = Times::try_from(spec_times).unwrap();
    assert_eq!(times.start(), Time::new(1, 2));
    assert_eq!(times.end(), Time::new(3, 4));
    assert!(Times::try_from(TimeSpec::Time(Time::new(1, 2))).is_err());
}
