use std::borrow::Cow;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter, Write};

use ::chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// A timestamp may be active (<> in org-mode) or inactive ([] in org-mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Active,
    Inactive,
}

/// A time of day, with minute precision. e.g., `03:14`.
// TODO: type safe seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time(pub(crate) NaiveTime);

/// A range of times of day, with minute precision. e.g., `5:00-7:00`,
/// `23:00-02:00`, or `01:30-1:30`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Times(pub(crate) Time, pub(crate) Time);

/// Either a `Time` or a `Times`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeSpec {
    Time(Time),
    Times(Times),
}

/// A date without a timezone. e.g., `2020-01-23`, `2023-01-25 Tue`, `1977-09-25
/// Zeepsday`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date(pub(crate) NaiveDate);

/// A unit of time duration. One of `h`, `d`, `w`, `m`, and `y`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TimeUnit {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// An org-mode repeater mark. One of `+`, `++`, and `.+`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RepeaterMark {
    Cumulate,
    CatchUp,
    Restart,
}

/// An org-mode delay mark. One of `-` and `--`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelayMark {
    All,
    First,
}

/// An interval of time. e.g., `5d`, `1h`, `7y`, `09w`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Interval {
    value: usize,
    unit: TimeUnit,
}

/// An org-mode repeater. e.g., `+5d`, `++1w`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Repeater {
    pub(crate) mark: RepeaterMark,
    pub(crate) interval: Interval,
}

/// An org-mode delay/warning. e.g., `-1d`, `--1w`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Delay {
    pub(crate) mark: DelayMark,
    pub(crate) interval: Interval,
}

/// An org-mode repeater and delay (both optional). e.g., ``, `+1d -1w`, `--1d,
/// .+2y`, `--1y`, `++1y`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct RepeaterAndDelay {
    pub(crate) repeater: Option<Repeater>,
    pub(crate) delay: Option<Delay>,
}

/// The `Diary` variant of an org-mode timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Diary<'a>(pub(crate) Cow<'a, str>);

/// The `Active` or `Inactive` variant of an org-mode timestamp. Note that these
/// do not include a time-range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub(crate) active: Activity,
    pub(crate) date: Date,
    pub(crate) time: Option<Time>,
    pub(crate) cookie: RepeaterAndDelay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub(crate) start: Point,
    pub(crate) end: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeRange {
    pub(crate) start: Point,
    pub(crate) end_time: Time,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Timestamp<'a> {
    Diary(Diary<'a>),
    Point(Point),
    Range(Range),
    TimeRange(TimeRange),
}

pub trait TimestampExt {
    fn start_timestamp(&self) -> Option<Timestamp> {
        self.start_point().map(Into::into)
    }

    fn end_timestamp(&self) -> Option<Timestamp> {
        self.end_point().map(Into::into)
    }

    // Returns a single point representing the start of this timestamp, or the
    // timestamp itself if it is a point.
    fn start_point(&self) -> Option<Point>;

    // Returns a single point representing the end of this timestamp if it
    // is a range, or None otherwise.
    fn end_point(&self) -> Option<Point>;

    fn start_time(&self) -> Option<Time>;

    fn end_time(&self) -> Option<Time>;

    fn active(&self) -> Activity;

    fn to_timestamp<'a>(&'a self) -> Timestamp<'a>;

    // Returns whether the two timestamps overlap ignoring time.
    fn date_overlap<T: TimestampExt>(&self, other: &T) -> bool {
        match (self.to_timestamp(), other.to_timestamp()) {
            (Timestamp::Diary(..), _) => false,
            (_, Timestamp::Diary(..)) => false,
            (Timestamp::Point(a), Timestamp::Point(b)) => a.date.0 == b.date.0,
            (Timestamp::Range(b), Timestamp::Point(a))
            | (Timestamp::Point(a), Timestamp::Range(b)) => {
                b.start.date.0 <= a.date.0 && a.date.0 <= b.end.date.0
            }
            (Timestamp::Range(a), Timestamp::Range(b)) => {
                !(a.start.date.0 > b.end.date.0 || b.start.date.0 > a.end.date.0)
            }
            (Timestamp::TimeRange(range), _) | (_, Timestamp::TimeRange(range)) => {
                Timestamp::Point(range.start).date_overlap(other)
            }
        }
    }

    // Returns the singular date, if the timestamp has one at the type level.
    fn date(&self) -> Option<Date>;

    // Returns the singular time, if the timestamp has one at the type level.
    fn time(&self) -> Option<Time>;

    // Returns the singular cookie, if the timestamp has only one at the type
    // level.
    fn cookie(&self) -> Option<RepeaterAndDelay>;
}

impl Activity {
    pub fn new(active: bool) -> Activity {
        if active {
            Activity::Active
        } else {
            Activity::Inactive
        }
    }

    pub fn is_active(&self) -> bool {
        self.into()
    }

    pub fn delimiters(&self) -> (char, char) {
        match self {
            Activity::Active => ('<', '>'),
            Activity::Inactive => ('[', ']'),
        }
    }
}

impl Interval {
    pub fn new(value: usize, unit: TimeUnit) -> Interval {
        Interval { value, unit }
    }

    pub fn value(&self) -> usize {
        self.value
    }

    pub fn unit(&self) -> TimeUnit {
        self.unit
    }

    pub fn with_value(&self, value: usize) -> Interval {
        Interval { value, ..*self }
    }

    pub fn with_unit(&self, unit: TimeUnit) -> Interval {
        Interval { unit, ..*self }
    }
}

impl Repeater {
    pub fn new(mark: RepeaterMark, interval: Interval) -> Repeater {
        Repeater { mark, interval }
    }

    pub fn mark(&self) -> RepeaterMark {
        self.mark
    }

    pub fn interval(&self) -> Interval {
        self.interval
    }

    pub fn value(&self) -> usize {
        self.interval.value
    }

    pub fn unit(&self) -> TimeUnit {
        self.interval.unit
    }

    pub fn with_mark(&self, mark: RepeaterMark) -> Repeater {
        let mark = mark.try_into().map_err(|_| ()).unwrap();
        Repeater { mark, ..*self }
    }

    pub fn with_interval(&self, interval: Interval) -> Repeater {
        let interval = interval.try_into().map_err(|_| ()).unwrap();
        Repeater { interval, ..*self }
    }

    pub fn with_unit(&self, unit: TimeUnit) -> Repeater {
        Repeater {
            interval: self.interval.with_unit(unit),
            ..*self
        }
    }

    pub fn with_value(&self, value: usize) -> Repeater {
        Repeater {
            interval: self.interval.with_value(value),
            ..*self
        }
    }
}

impl Delay {
    pub fn new(mark: DelayMark, interval: Interval) -> Delay {
        Delay { mark, interval }
    }

    pub fn mark(&self) -> DelayMark {
        self.mark
    }

    pub fn interval(&self) -> Interval {
        self.interval
    }

    pub fn value(&self) -> usize {
        self.interval.value
    }

    pub fn unit(&self) -> TimeUnit {
        self.interval.unit
    }

    pub fn with_mark(&self, mark: DelayMark) -> Delay {
        Delay { mark, ..*self }
    }

    pub fn with_interval(&self, interval: Interval) -> Delay {
        Delay { interval, ..*self }
    }

    pub fn with_unit(&self, unit: TimeUnit) -> Delay {
        Delay {
            interval: self.interval.with_unit(unit),
            ..*self
        }
    }

    pub fn with_value(&self, value: usize) -> Delay {
        Delay {
            interval: self.interval.with_value(value),
            ..*self
        }
    }
}

impl RepeaterAndDelay {
    pub fn new(repeater: Option<Repeater>, delay: Option<Delay>) -> RepeaterAndDelay {
        RepeaterAndDelay { repeater, delay }
    }

    pub fn with_repeater(&self, repeater: Option<Repeater>) -> RepeaterAndDelay {
        RepeaterAndDelay { repeater, ..*self }
    }

    pub fn with_delay(&self, delay: Option<Delay>) -> RepeaterAndDelay {
        RepeaterAndDelay { delay, ..*self }
    }
}

impl<'a> Timestamp<'a> {
    pub fn into_owned(self) -> Timestamp<'static> {
        match self {
            Timestamp::Diary(diary) => Timestamp::Diary(diary.into_owned()),
            Timestamp::Point(point) => Timestamp::Point(point),
            Timestamp::Range(range) => Timestamp::Range(range),
            Timestamp::TimeRange(range) => Timestamp::TimeRange(range),
        }
    }

    pub fn to_borrowed(&self) -> Timestamp<'a> {
        match self {
            Timestamp::Diary(diary) => Timestamp::Diary(diary.to_borrowed()),
            Timestamp::Point(point) => Timestamp::Point(*point),
            Timestamp::Range(range) => Timestamp::Range(*range),
            Timestamp::TimeRange(range) => Timestamp::TimeRange(*range),
        }
    }
}

impl Time {
    pub fn new(hour: u32, minute: u32) -> Time {
        NaiveTime::from_hms(hour, minute, 0)
            .try_into()
            .map_err(|_| ())
            .unwrap()
    }

    pub fn hour(self) -> u8 {
        self.0.hour().try_into().unwrap()
    }

    pub fn minute(self) -> u8 {
        self.0.minute().try_into().unwrap()
    }

    pub fn with_hour(self, hour: u32) -> Time {
        Time::new(hour, self.minute().into())
    }

    pub fn with_minute(self, minute: u32) -> Time {
        Time::new(minute, self.hour().into())
    }
}

impl Times {
    pub fn start(&self) -> Time {
        self.0
    }

    pub fn end(&self) -> Time {
        self.0
    }

    pub fn with_start(&self, start: Time) -> Times {
        Times(start, self.1)
    }

    pub fn with_end(&self, end: Time) -> Times {
        Times(self.0, end)
    }
}

impl TimeSpec {
    pub fn start(&self) -> Time {
        match self {
            TimeSpec::Time(time) => *time,
            TimeSpec::Times(times) => times.0,
        }
    }

    pub fn end(&self) -> Option<Time> {
        match self {
            TimeSpec::Time(_time) => None,
            TimeSpec::Times(times) => Some(times.1),
        }
    }
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Date {
        NaiveDate::from_ymd(year.into(), month.into(), day.into()).into()
    }
}

impl Point {
    pub fn new(date: Date) -> Point {
        Point {
            active: Activity::Active,
            date,
            time: None,
            cookie: RepeaterAndDelay::default(),
        }
    }

    pub fn with_active(&self, active: Activity) -> Point {
        Point { active, ..*self }
    }

    pub fn with_time(&self, time: Option<Time>) -> Point {
        Point { time, ..*self }
    }

    pub fn with_date(&self, date: Date) -> Point {
        Point { date, ..*self }
    }

    pub fn with_cookie(&self, cookie: RepeaterAndDelay) -> Point {
        Point { cookie, ..*self }
    }

    pub fn with_repeater(&self, repeater: Option<Repeater>) -> Point {
        Point {
            cookie: self.cookie.with_repeater(repeater),
            ..*self
        }
    }

    pub fn with_delay(&self, delay: Option<Delay>) -> Point {
        Point {
            cookie: self.cookie.with_delay(delay),
            ..*self
        }
    }
}

impl Range {
    pub fn new(start: Point, mut end: Point) -> Range {
        end.active = start.active;
        Range { start, end }
    }

    // Note: This will NOT change the active/inactive status of the Range.
    pub fn with_start(&self, start: Point) -> Range {
        Range {
            start: start.with_active(self.start.active),
            ..*self
        }
    }

    // Note: This will NOT change the active/inactive status of the Range.
    pub fn with_end(&self, end: Point) -> Range {
        Range {
            end: end.with_active(self.end.active),
            ..*self
        }
    }

    pub fn with_active(&self, active: Activity) -> Range {
        Range {
            start: self.start.with_active(active),
            end: self.end.with_active(active),
        }
    }
}

impl TimeRange {
    pub fn new(start: Point, end_time: Time) -> TimeRange {
        TimeRange { start, end_time }
    }

    // Note: This WILL NOT change the active/inactive status of the Range.
    pub fn with_start(&self, start: Point) -> TimeRange {
        TimeRange {
            start: start.with_active(self.start.active),
            ..*self
        }
    }

    // Note: This will NOT change the active/inactive status of the Range.
    pub fn with_start_time(&self, start_time: Time) -> TimeRange {
        TimeRange {
            start: self.start.with_time(Some(start_time)),
            ..*self
        }
    }

    // Note: This will NOT change the active/inactive status of the Range.
    pub fn with_end_time(&self, end_time: Time) -> TimeRange {
        TimeRange { end_time, ..*self }
    }

    pub fn with_start_and_end(&self, start_time: Time, end_time: Time) -> TimeRange {
        TimeRange {
            start: self.start.with_time(Some(start_time)),
            end_time,
        }
    }

    pub fn with_times(&self, times: Times) -> TimeRange {
        self.with_start_and_end(times.0, times.1)
    }

    pub fn with_active(&self, active: Activity) -> TimeRange {
        TimeRange {
            start: self.start.with_active(active),
            ..*self
        }
    }
}

impl<'a> Diary<'a> {
    pub fn new<'b>(s: &'b str) -> Diary<'b> {
        Diary(Cow::Borrowed(s))
    }

    pub fn into_owned(self) -> Diary<'static> {
        match self.0 {
            Cow::Owned(x) => Diary(Cow::Owned(x)),
            Cow::Borrowed(..) => Diary(Cow::Owned(self.0.to_string())),
        }
    }

    pub fn to_borrowed(&self) -> Diary<'a> {
        match &self.0 {
            Cow::Owned(..) => self.clone(),
            Cow::Borrowed(x) => Diary(Cow::Borrowed(x)),
        }
    }
}

impl TimestampExt for Point {
    fn start_point(&self) -> Option<Point> {
        Some(*self)
    }

    fn end_point(&self) -> Option<Point> {
        None
    }

    fn start_time(&self) -> Option<Time> {
        self.time
    }

    fn end_time(&self) -> Option<Time> {
        None
    }

    fn active(&self) -> Activity {
        self.active
    }

    fn to_timestamp(&self) -> Timestamp {
        self.into()
    }

    fn date(&self) -> Option<Date> {
        Some(self.date)
    }

    fn time(&self) -> Option<Time> {
        self.time
    }

    fn cookie(&self) -> Option<RepeaterAndDelay> {
        Some(self.cookie)
    }
}

impl TimestampExt for Diary<'_> {
    fn start_point(&self) -> Option<Point> {
        None
    }

    fn end_point(&self) -> Option<Point> {
        None
    }

    fn start_time(&self) -> Option<Time> {
        None
    }

    fn end_time(&self) -> Option<Time> {
        None
    }

    fn active(&self) -> Activity {
        Activity::Active
    }

    fn to_timestamp<'a>(&'a self) -> Timestamp<'a> {
        Timestamp::Diary(self.clone())
    }

    fn date(&self) -> Option<Date> {
        None
    }

    fn time(&self) -> Option<Time> {
        None
    }

    fn cookie(&self) -> Option<RepeaterAndDelay> {
        None
    }
}

impl TimestampExt for Range {
    fn start_point(&self) -> Option<Point> {
        Some(self.start)
    }

    fn end_point(&self) -> Option<Point> {
        Some(self.end)
    }

    fn start_time(&self) -> Option<Time> {
        self.start.time
    }

    fn end_time(&self) -> Option<Time> {
        self.end.time
    }

    fn active(&self) -> Activity {
        self.start.active
    }

    fn to_timestamp(&self) -> Timestamp<'static> {
        Timestamp::Range(self.clone())
    }

    fn date(&self) -> Option<Date> {
        None
    }

    fn time(&self) -> Option<Time> {
        None
    }

    fn cookie(&self) -> Option<RepeaterAndDelay> {
        None
    }
}

impl TimestampExt for TimeRange {
    fn start_point(&self) -> Option<Point> {
        Some(self.start)
    }

    fn end_point(&self) -> Option<Point> {
        Some(Point {
            time: Some(self.end_time),
            ..self.start
        })
    }

    fn start_time(&self) -> Option<Time> {
        self.start.time
    }

    fn end_time(&self) -> Option<Time> {
        Some(self.end_time)
    }

    fn active(&self) -> Activity {
        self.start.active
    }

    fn to_timestamp(&self) -> Timestamp<'static> {
        Timestamp::TimeRange(self.clone())
    }

    fn date(&self) -> Option<Date> {
        Some(self.start.date)
    }

    fn time(&self) -> Option<Time> {
        None
    }

    fn cookie(&self) -> Option<RepeaterAndDelay> {
        Some(self.start.cookie)
    }
}

impl TimestampExt for Timestamp<'_> {
    fn start_point(&self) -> Option<Point> {
        match self {
            Timestamp::Diary(_diary) => None,
            Timestamp::Point(point) => Some(*point),
            Timestamp::Range(range) => Some(range.start),
            Timestamp::TimeRange(range) => Some(range.start),
        }
    }

    fn end_point(&self) -> Option<Point> {
        match self {
            Timestamp::Diary(..) | Timestamp::Point(..) => None,
            Timestamp::Range(range) => Some(range.end),
            Timestamp::TimeRange(range) => Some(Point {
                time: Some(range.end_time),
                ..range.start
            }),
        }
    }

    fn start_time(&self) -> Option<Time> {
        match self {
            Timestamp::Diary(_diary) => None,
            Timestamp::Point(point) => point.time,
            Timestamp::Range(range) => range.start.time,
            Timestamp::TimeRange(range) => range.start.time,
        }
    }

    fn end_time(&self) -> Option<Time> {
        match self {
            Timestamp::Diary(..) | Timestamp::Point(..) => None,
            Timestamp::Range(range) => range.end.time,
            Timestamp::TimeRange(range) => Some(range.end_time),
        }
    }

    fn active(&self) -> Activity {
        match self {
            Timestamp::Diary(_diary) => Activity::Active,
            Timestamp::Point(point) => point.active,
            Timestamp::Range(range) => range.start.active,
            Timestamp::TimeRange(range) => range.start.active,
        }
    }

    fn to_timestamp(&self) -> Timestamp {
        self.clone()
    }

    fn date(&self) -> Option<Date> {
        match self {
            Timestamp::Diary(_diary) => None,
            Timestamp::Point(point) => Some(point.date),
            Timestamp::Range(..) => None,
            Timestamp::TimeRange(range) => Some(range.start.date),
        }
    }

    fn time(&self) -> Option<Time> {
        match self {
            Timestamp::Diary(_diary) => None,
            Timestamp::Point(point) => point.time,
            Timestamp::Range(..) => None,
            Timestamp::TimeRange(..) => None,
        }
    }

    fn cookie(&self) -> Option<RepeaterAndDelay> {
        match self {
            Timestamp::Diary(_diary) => None,
            Timestamp::Point(point) => Some(point.cookie),
            Timestamp::Range(..) => None,
            Timestamp::TimeRange(time_range) => Some(time_range.start.cookie),
        }
    }
}

impl Default for Activity {
    fn default() -> Activity {
        Activity::Active
    }
}

impl Into<bool> for &Activity {
    fn into(self) -> bool {
        *self == Activity::Active
    }
}

impl Into<bool> for Activity {
    fn into(self) -> bool {
        (&self).into()
    }
}

impl From<bool> for Activity {
    fn from(active: bool) -> Activity {
        Activity::new(active)
    }
}

impl Default for Time {
    fn default() -> Time {
        Time(NaiveTime::from_num_seconds_from_midnight(0, 0))
    }
}

impl From<NaiveTime> for Time {
    fn from(time: NaiveTime) -> Time {
        (&time).into()
    }
}

impl From<&NaiveTime> for Time {
    fn from(time: &NaiveTime) -> Time {
        Time(NaiveTime::from_hms(time.hour(), time.minute(), 0))
    }
}

impl TryFrom<TimeSpec> for Time {
    type Error = ();
    fn try_from(time: TimeSpec) -> Result<Self, Self::Error> {
        match time {
            TimeSpec::Time(time) => Ok(time),
            _ => Err(()),
        }
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0.format("%H:%M"))
    }
}

impl TryFrom<TimeSpec> for Times {
    type Error = ();
    fn try_from(time: TimeSpec) -> Result<Self, Self::Error> {
        match time {
            TimeSpec::Times(times) => Ok(times),
            _ => Err(()),
        }
    }
}

impl Display for Times {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.0, self.1)
    }
}

impl Default for TimeSpec {
    fn default() -> TimeSpec {
        TimeSpec::Time(Time::default())
    }
}

impl From<Time> for TimeSpec {
    fn from(time: Time) -> Self {
        TimeSpec::Time(time)
    }
}

impl From<Times> for TimeSpec {
    fn from(times: Times) -> Self {
        TimeSpec::Times(times)
    }
}

impl Display for TimeSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TimeSpec::Times(times) => times.fmt(f),
            TimeSpec::Time(time) => time.fmt(f),
        }
    }
}

impl From<NaiveDate> for Date {
    fn from(date: NaiveDate) -> Date {
        (&date).into()
    }
}

impl From<&NaiveDate> for Date {
    fn from(date: &NaiveDate) -> Date {
        Date(*date)
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl AsRef<str> for TimeUnit {
    fn as_ref(&self) -> &str {
        match self {
            TimeUnit::Hour => "h",
            TimeUnit::Day => "d",
            TimeUnit::Week => "w",
            TimeUnit::Month => "m",
            TimeUnit::Year => "y",
        }
    }
}

impl Into<char> for &TimeUnit {
    fn into(self) -> char {
        self.as_ref().chars().next().unwrap()
    }
}

impl Into<char> for TimeUnit {
    fn into(self) -> char {
        (&self).into()
    }
}

impl fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_char((*self).into())
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.value, self.unit)
    }
}

impl AsRef<str> for RepeaterMark {
    fn as_ref(&self) -> &str {
        match self {
            RepeaterMark::CatchUp => "++",
            RepeaterMark::Cumulate => "+",
            RepeaterMark::Restart => ".+",
        }
    }
}

impl fmt::Display for RepeaterMark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl AsRef<str> for DelayMark {
    fn as_ref(&self) -> &str {
        match self {
            DelayMark::All => "-",
            DelayMark::First => "--",
        }
    }
}

impl fmt::Display for DelayMark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl fmt::Display for Repeater {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.mark, self.interval)
    }
}

impl fmt::Display for Delay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.mark, self.interval)
    }
}

impl fmt::Display for RepeaterAndDelay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.repeater.as_ref(), self.delay.as_ref()) {
            (Some(r), Some(d)) => write!(f, "{} {}", &r, &d),
            (Some(r), None) => r.fmt(f),
            (None, Some(d)) => d.fmt(f),
            (None, None) => Ok(()),
        }
    }
}

impl fmt::Display for Diary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<%%({})>", &self.0)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (open, close) = self.active.delimiters();
        write!(f, "{}{}", open, &self.date)?;
        if let Some(time) = self.time.as_ref() {
            write!(f, " {}", time)?;
        }
        if self.cookie != RepeaterAndDelay::default() {
            write!(f, " {}", &self.cookie)?;
        }
        f.write_char(close)
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}--{}", &self.start, &self.end)
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (open, close) = self.start.active.delimiters();
        write!(f, "{}{}", open, &self.start.date)?;
        let start = self.start.time.as_ref().expect("must have time");
        write!(f, " {}-{}", start, &self.end_time)?;
        if self.start.cookie != RepeaterAndDelay::default() {
            write!(f, " {}", &self.start.cookie)?;
        }
        f.write_char(close)
    }
}

impl fmt::Display for Timestamp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Timestamp::Point(timestamp) => timestamp.fmt(f),
            Timestamp::Range(timestamp) => timestamp.fmt(f),
            Timestamp::TimeRange(timestamp) => timestamp.fmt(f),
            Timestamp::Diary(timestamp) => timestamp.fmt(f),
        }
    }
}

mod chrono {
    use super::{Date, Time};
    use ::chrono::*;

    impl Into<NaiveTime> for Time {
        fn into(self) -> NaiveTime {
            (&self).into()
        }
    }

    impl Into<NaiveTime> for &Time {
        fn into(self) -> NaiveTime {
            self.0
        }
    }

    impl Into<NaiveDate> for Date {
        fn into(self) -> NaiveDate {
            (&self).into()
        }
    }

    impl Into<NaiveDate> for &Date {
        fn into(self) -> NaiveDate {
            self.0
        }
    }

    impl Into<NaiveDateTime> for Date {
        fn into(self) -> NaiveDateTime {
            (&self).into()
        }
    }

    impl Into<NaiveDateTime> for &Date {
        fn into(self) -> NaiveDateTime {
            NaiveDateTime::new(self.into(), NaiveTime::from_hms(0, 0, 0))
        }
    }
}

impl<'a> TryFrom<Timestamp<'a>> for Diary<'a> {
    type Error = ();
    fn try_from(timestamp: Timestamp<'a>) -> Result<Diary<'a>, Self::Error> {
        match timestamp {
            Timestamp::Diary(d) => Ok(d),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Timestamp<'a>> for Diary<'a> {
    type Error = ();
    fn try_from(timestamp: &'a Timestamp<'a>) -> Result<Diary<'a>, Self::Error> {
        match timestamp {
            Timestamp::Diary(d) => Ok(Diary(Cow::Borrowed(&d.0))),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Timestamp<'a>> for Point {
    type Error = ();
    fn try_from(timestamp: Timestamp<'a>) -> Result<Self, Self::Error> {
        (&timestamp).try_into()
    }
}

impl<'a> TryFrom<&'a Timestamp<'a>> for Point {
    type Error = ();
    fn try_from(timestamp: &'a Timestamp<'a>) -> Result<Self, Self::Error> {
        match timestamp {
            Timestamp::Point(d) => Ok(*d),
            _ => Err(()),
        }
    }
}

impl From<Date> for Point {
    fn from(date: Date) -> Point {
        (&date).into()
    }
}

impl From<&Date> for Point {
    fn from(date: &Date) -> Point {
        Point::new(*date)
    }
}

impl From<NaiveDate> for Point {
    fn from(date: NaiveDate) -> Point {
        (&date).into()
    }
}

impl From<&NaiveDate> for Point {
    fn from(date: &NaiveDate) -> Point {
        let date: Date = date.into();
        date.into()
    }
}

impl From<NaiveDateTime> for Point {
    fn from(date: NaiveDateTime) -> Point {
        (&date).into()
    }
}

impl From<&NaiveDateTime> for Point {
    fn from(date: &NaiveDateTime) -> Point {
        Point::new(date.date().into()).with_time(Some(date.time().into()))
    }
}

impl From<&NaiveDate> for Timestamp<'_> {
    fn from(date: &NaiveDate) -> Timestamp<'static> {
        let date: Point = date.into();
        date.into()
    }
}

impl From<Date> for Timestamp<'_> {
    fn from(date: Date) -> Timestamp<'static> {
        Timestamp::Point(Point::new(date))
    }
}

impl From<NaiveDate> for Timestamp<'_> {
    fn from(date: NaiveDate) -> Timestamp<'static> {
        let date: Date = date.into();
        date.into()
    }
}

impl From<&Point> for Timestamp<'_> {
    fn from(point: &Point) -> Timestamp<'static> {
        Timestamp::Point(*point)
    }
}

impl From<Point> for Timestamp<'_> {
    fn from(point: Point) -> Timestamp<'static> {
        Timestamp::Point(point)
    }
}

impl From<TimeRange> for Range {
    fn from(range: TimeRange) -> Self {
        (&range).into()
    }
}

impl From<&TimeRange> for Range {
    fn from(range: &TimeRange) -> Self {
        let start = range.start;
        let end = start.with_time(Some(range.end_time));
        Range { start, end }
    }
}

impl<'a> TryFrom<Timestamp<'a>> for Range {
    type Error = ();
    fn try_from(timestamp: Timestamp<'a>) -> Result<Self, Self::Error> {
        (&timestamp).try_into()
    }
}

impl<'a> TryFrom<&'a Timestamp<'a>> for Range {
    type Error = ();
    fn try_from(timestamp: &'a Timestamp<'a>) -> Result<Self, Self::Error> {
        match timestamp {
            Timestamp::Range(r) => Ok(*r),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Timestamp<'a>> for TimeRange {
    type Error = ();
    fn try_from(timestamp: Timestamp<'a>) -> Result<Self, Self::Error> {
        (&timestamp).try_into()
    }
}

impl<'a> TryFrom<&'a Timestamp<'a>> for TimeRange {
    type Error = ();
    fn try_from(timestamp: &'a Timestamp<'a>) -> Result<Self, Self::Error> {
        match timestamp {
            Timestamp::TimeRange(r) => Ok(*r),
            _ => Err(()),
        }
    }
}

impl TryFrom<&Range> for TimeRange {
    type Error = ();
    fn try_from(range: &Range) -> Result<Self, Self::Error> {
        if range.start.date == range.end.date
            && range.start.cookie == range.end.cookie
            && range.start.time.is_some()
        {
            if let Some(end_time) = range.end.time {
                return Ok(TimeRange {
                    start: range.start,
                    end_time,
                });
            }
        }
        Err(())
    }
}

impl<P: AsRef<Point>> From<P> for TimeRange {
    fn from(point: P) -> Self {
        TimeRange {
            start: *point.as_ref(),
            end_time: Time::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_point() {
        assert_eq!(
            Point::new(Date::new(1971, 11, 11)).to_string().as_str(),
            "<1971-11-11>"
        );
        assert_eq!(
            Point::new(Date::new(1971, 11, 11))
                .with_active(Activity::Inactive)
                .to_string()
                .as_str(),
            "[1971-11-11]"
        );
        assert_eq!(
            Point::new(Date::new(1971, 11, 11))
                .with_active(Activity::Inactive)
                .with_time(Some(Time::new(9, 18)))
                .to_string()
                .as_str(),
            "[1971-11-11 09:18]"
        );
        assert_eq!(
            Point::new(Date::new(1971, 11, 11))
                .with_active(Activity::Inactive)
                .with_time(Some(Time::new(9, 18)))
                .with_repeater(Some(Repeater::parse(".+7d").unwrap().1))
                .to_string()
                .as_str(),
            "[1971-11-11 09:18 .+7d]"
        );
        assert_eq!(
            Point::new(Date::new(2021, 1, 2))
                .with_active(Activity::Inactive)
                .with_delay(Some(Delay::parse("--7d").unwrap().1))
                .to_string()
                .as_str(),
            "[2021-01-02 --7d]"
        );
        assert_eq!(
            Point::new(Date::new(1971, 11, 11))
                .with_active(Activity::Inactive)
                .with_time(Some(Time::new(9, 18)))
                .with_delay(Some(Delay::parse("-8w").unwrap().1))
                .with_repeater(Some(Repeater::parse("+7d").unwrap().1))
                .to_string()
                .as_str(),
            "[1971-11-11 09:18 +7d -8w]"
        );
    }

    #[test]
    fn test_format_diary() {
        assert_eq!(Diary::new("hello").to_string().as_str(), "<%%(hello)>");
        assert_eq!(Diary::new("").to_string().as_str(), "<%%()>");
    }

    #[test]
    fn test_format_range() {
        let a = Point::new(Date::new(2018, 9, 2)).with_time(Some(Time::new(8, 17)));
        let b = Point::new(Date::new(2018, 9, 3)).with_time(Some(Time::new(8, 17)));
        assert_eq!(
            Range::new(a, b).to_string().as_str(),
            "<2018-09-02 08:17>--<2018-09-03 08:17>"
        );
        assert_eq!(
            Range::new(a.with_time(None), b).to_string().as_str(),
            "<2018-09-02>--<2018-09-03 08:17>"
        );
        assert_eq!(
            Range::new(a, b.with_repeater(Some(Repeater::parse("+3d").unwrap().1)))
                .to_string()
                .as_str(),
            "<2018-09-02 08:17>--<2018-09-03 08:17 +3d>"
        );
    }

    #[test]
    fn test_format_time_range() {
        let a = Point::new(Date::new(2018, 9, 2)).with_time(Some(Time::new(8, 17)));
        let b = Time::new(18, 3);
        assert_eq!(
            TimeRange::new(a, b).to_string().as_str(),
            "<2018-09-02 08:17-18:03>"
        );
        assert_eq!(
            TimeRange::new(
                a.with_repeater(Some(Repeater::new(
                    RepeaterMark::Cumulate,
                    Interval::new(3, TimeUnit::Year)
                ))),
                b
            )
            .to_string()
            .as_str(),
            "<2018-09-02 08:17-18:03 +3y>"
        );
    }
}
