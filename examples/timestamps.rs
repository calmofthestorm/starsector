use starsector::{
    Activity, Date, Delay, DelayMark, Diary, Interval, Point, Range, Repeater, RepeaterAndDelay,
    RepeaterMark, Time, TimeRange, TimeSpec, TimeUnit, Timestamp, TimestampExt,
};

fn main() {
    let active = Activity::new(true);
    assert!(active.is_active());
    assert_eq!(active.delimiters(), ('<', '>'));
    println!(
        "Activity::new(true) => active={}, delimiters={:?}",
        active.is_active(),
        active.delimiters()
    );

    let t = Time::new(9, 30);
    assert_eq!(t.hour(), 9);
    assert_eq!(t.minute(), 30);
    let t2 = t.with_hour(10).with_minute(45);
    assert_eq!(t2.hour(), 10);
    println!("Time API: start={} end={}", t, t2);

    let spec1: TimeSpec = t.into();
    assert_eq!(spec1.start(), t);
    assert!(spec1.end().is_none());
    println!(
        "TimeSpec from single time: start={} end={:?}",
        spec1.start(),
        spec1.end().map(|x| x.to_string())
    );

    let date = Date::new(2026, 3, 14);
    let interval = Interval::new(2, TimeUnit::Week);
    assert_eq!(interval.value(), 2);
    assert_eq!(interval.unit(), TimeUnit::Week);
    let interval2 = interval.with_value(3).with_unit(TimeUnit::Day);

    let rep = Repeater::new(RepeaterMark::CatchUp, interval2)
        .with_mark(RepeaterMark::Cumulate)
        .with_interval(interval)
        .with_value(4)
        .with_unit(TimeUnit::Month);
    assert_eq!(rep.mark(), RepeaterMark::Cumulate);
    assert_eq!(rep.value(), 4);

    let delay = Delay::new(DelayMark::First, Interval::new(1, TimeUnit::Day))
        .with_mark(DelayMark::All)
        .with_interval(Interval::new(2, TimeUnit::Day))
        .with_value(3)
        .with_unit(TimeUnit::Week);
    assert_eq!(delay.mark(), DelayMark::All);
    assert_eq!(delay.unit(), TimeUnit::Week);

    let cookie = RepeaterAndDelay::new(Some(rep), Some(delay))
        .with_repeater(Some(rep))
        .with_delay(Some(delay));
    println!("Repeater/Delay API: repeater={} delay={}", rep, delay);

    let point = Point::new(date)
        .with_active(Activity::Inactive)
        .with_time(Some(Time::new(11, 15)))
        .with_cookie(cookie)
        .with_repeater(Some(rep))
        .with_delay(Some(delay))
        .with_date(Date::new(2026, 3, 15));

    let range = Range::new(point, point.with_date(Date::new(2026, 3, 20)))
        .with_active(Activity::Active)
        .with_start(point)
        .with_end(point.with_date(Date::new(2026, 3, 22)));

    let time_range = TimeRange::new(point, Time::new(12, 0))
        .with_start(point)
        .with_start_time(Time::new(10, 0))
        .with_end_time(Time::new(13, 0))
        .with_start_and_end(Time::new(9, 0), Time::new(11, 0))
        .with_active(Activity::Inactive);
    println!(
        "Point/Range/TimeRange: point={:?}, range={:?}, time_range={:?}",
        point, range, time_range
    );

    let diary = Diary::new("%%(diary-float t 2 3)");
    let diary_owned = diary.clone().into_owned();
    let _diary_borrowed = diary_owned.to_borrowed();
    println!("Diary example: {:?}", diary);

    let ts_point: Timestamp<'_> = point.into();
    let ts_range: Timestamp<'_> = Timestamp::Range(range);
    let ts_time_range: Timestamp<'_> = Timestamp::TimeRange(time_range);
    let ts_diary: Timestamp<'_> = Timestamp::Diary(diary);

    assert!(ts_point.start_point().is_some());
    assert!(ts_range.end_point().is_some());
    assert!(ts_time_range.end_time().is_some());
    assert!(ts_diary.start_point().is_none());
    println!(
        "Timestamp variants: point={:?}, range={:?}, time_range={:?}, diary={:?}",
        ts_point, ts_range, ts_time_range, ts_diary
    );

    let overlap = ts_range.date_overlap(&ts_point);
    assert!(overlap);
    println!("date_overlap(range, point) => {overlap}");

    let owned = ts_point.clone().into_owned();
    let borrowed = owned.to_borrowed();
    assert_eq!(borrowed.active(), owned.active());
    println!(
        "Borrow/own conversion preserved activity={:?}",
        owned.active()
    );

    let _ = rep.interval();
    let _ = rep.unit();
    let _ = delay.interval();
    let _ = delay.value();
}
