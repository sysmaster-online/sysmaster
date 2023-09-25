//! Parsers of systemd-style datetimes.
use crate::{config::UnitEntry, duration::duration};
use chrono::{prelude::*, Duration};
use chrono_tz::Tz;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take},
    character::complete::{digit1, not_line_ending, space0},
    combinator::{opt, value},
    sequence::{preceded, terminated},
    IResult,
};

fn datetime(i: &str) -> IResult<&str, DateTime<Utc>> {
    alt((special, absolute, relative, full_len))(i)
}

fn weekday(i: &str) -> IResult<&str, Weekday> {
    fn mon(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("mon"), tag_no_case("monday")))(i)?;
        Ok((i, ()))
    }

    fn tue(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("tue"), tag_no_case("tuesday")))(i)?;
        Ok((i, ()))
    }

    fn wed(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("wed"), tag_no_case("wednesday")))(i)?;
        Ok((i, ()))
    }

    fn thu(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("thu"), tag_no_case("thu")))(i)?;
        Ok((i, ()))
    }

    fn fri(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("fri"), tag_no_case("friday")))(i)?;
        Ok((i, ()))
    }

    fn sat(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("sat"), tag_no_case("saturday")))(i)?;
        Ok((i, ()))
    }

    fn sun(i: &str) -> IResult<&str, ()> {
        let (i, _) = alt((tag_no_case("sun"), tag_no_case("sunday")))(i)?;
        Ok((i, ()))
    }

    alt((
        value(Weekday::Mon, mon),
        value(Weekday::Tue, tue),
        value(Weekday::Wed, wed),
        value(Weekday::Thu, thu),
        value(Weekday::Fri, fri),
        value(Weekday::Sat, sat),
        value(Weekday::Sun, sun),
    ))(i)
}

fn absolute(i: &str) -> IResult<&str, DateTime<Utc>> {
    let (i, _) = tag("@")(i)?;
    let (i, int) = digit1(i)?;
    let offset = int.parse().unwrap();
    if let Some(naive) = NaiveDateTime::from_timestamp_opt(offset, 0) {
        Ok((i, Utc.from_utc_datetime(&naive)))
    } else {
        Err(nom::Err::Failure(nom::error::Error::new(
            i,
            nom::error::ErrorKind::Fail,
        )))
    }
}

fn full_len(i: &str) -> IResult<&str, DateTime<Utc>> {
    let now = Utc::now();
    let now_date = now.date_naive();

    fn digit4(i: &str) -> IResult<&str, u32> {
        let (i, int) = take(4usize)(i)?;
        if let Ok(int) = int.parse() {
            Ok((i, int))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                i,
                nom::error::ErrorKind::Digit,
            )))
        }
    }

    fn digit2(i: &str) -> IResult<&str, u32> {
        let (i, int) = take(2usize)(i)?;
        if let Ok(int) = int.parse() {
            Ok((i, int))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                i,
                nom::error::ErrorKind::Digit,
            )))
        }
    }

    fn digit2_year(i: &str) -> IResult<&str, u32> {
        let (i, int) = digit2(i)?;
        Ok((i, int + 2000))
    }

    fn date(i: &str) -> IResult<&str, (i32, u32, u32)> {
        let (i, year) = alt((digit4, digit2_year))(i)?;
        let (i, _) = tag("-")(i)?;
        let (i, month) = digit2(i)?;
        let (i, _) = tag("-")(i)?;
        let (i, day) = digit2(i)?;
        Ok((i, (year as i32, month, day)))
    }

    fn date_without_year(i: &str) -> IResult<&str, (i32, u32, u32)> {
        let (i, month) = digit2(i)?;
        let (i, _) = tag("-")(i)?;
        let (i, day) = digit2(i)?;

        let now_date = Utc::now().date_naive();
        Ok((i, (now_date.year(), month, day)))
    }

    fn time(i: &str) -> IResult<&str, (u32, u32, u32, i64)> {
        let (i, hour) = digit2(i)?;
        let (i, _) = tag(":")(i)?;
        let (i, minute) = digit2(i)?;
        let (i, second, microsecond) =
            if let (i, Some(second)) = opt(preceded(tag(":"), digit2))(i)? {
                if let (i, Some(microsecond)) = opt(preceded(tag("."), digit1))(i)? {
                    let microsecond = microsecond.parse().unwrap();
                    (i, second, microsecond)
                } else {
                    (i, second, 0)
                }
            } else {
                (i, 0, 0)
            };
        Ok((i, (hour, minute, second, microsecond)))
    }

    let (i, weekday) = opt(terminated(weekday, space0))(i)?;
    let (i, date) = opt(terminated(alt((date, date_without_year)), space0))(i)?;
    let (i, time) = opt(terminated(time, space0))(i)?;
    let (i, timezone) = opt(not_line_ending)(i)?;

    if date.is_none() && time.is_none() {
        return Err(nom::Err::Failure(nom::error::Error::new(
            i,
            nom::error::ErrorKind::Fail,
        )));
    }

    let (year, month, day) =
        date.unwrap_or_else(|| (now_date.year(), now_date.month(), now_date.day()));
    let (hour, minute, second, microsecond) = time.unwrap_or((0, 0, 0, 0));

    let result = match timezone {
        Some(timezone) if !timezone.is_empty() => {
            let timezone: Tz = timezone.parse().map_err(|_| {
                nom::Err::Failure(nom::error::Error::new(i, nom::error::ErrorKind::Fail))
            })?;
            timezone
                .with_ymd_and_hms(year, month, day, hour, minute, second)
                .single()
                .ok_or_else(|| {
                    nom::Err::Failure(nom::error::Error::new(i, nom::error::ErrorKind::Fail))
                })?
                .with_timezone(&Utc)
        }
        _ => Utc
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| {
                nom::Err::Failure(nom::error::Error::new(i, nom::error::ErrorKind::Fail))
            })?,
    };

    let result = result + Duration::microseconds(microsecond);

    // weekday validation
    if let Some(weekday) = weekday {
        if weekday != result.weekday() {
            return Err(nom::Err::Failure(nom::error::Error::new(
                i,
                nom::error::ErrorKind::Fail,
            )));
        }
    }

    Ok((i, result))
}

fn relative(i: &str) -> IResult<&str, DateTime<Utc>> {
    fn prefix_plus(i: &str) -> IResult<&str, DateTime<Utc>> {
        let (i, _) = tag("+")(i)?;
        let (i, duration) = duration(i)?;
        Ok((i, Utc::now() + duration))
    }

    fn prefix_minus(i: &str) -> IResult<&str, DateTime<Utc>> {
        let (i, _) = tag("-")(i)?;
        let (i, duration) = duration(i)?;
        Ok((i, Utc::now() - duration))
    }

    fn suffix_ago(i: &str) -> IResult<&str, DateTime<Utc>> {
        let (i, duration) = duration(i)?;
        let (i, _) = tag("ago")(i)?;
        Ok((i, Utc::now() - duration))
    }

    fn suffix_left(i: &str) -> IResult<&str, DateTime<Utc>> {
        let (i, duration) = duration(i)?;
        let (i, _) = tag("left")(i)?;
        Ok((i, Utc::now() + duration))
    }

    alt((prefix_minus, prefix_plus, suffix_ago, suffix_left))(i)
}

fn special(i: &str) -> IResult<&str, DateTime<Utc>> {
    fn now(i: &str) -> IResult<&str, DateTime<Utc>> {
        let (i, _) = tag("now")(i)?;
        Ok((i, Utc::now()))
    }

    fn today(i: &str) -> IResult<&str, &str> {
        tag("today")(i)
    }

    fn tomorrow(i: &str) -> IResult<&str, &str> {
        tag("tomorrow")(i)
    }

    fn yesturday(i: &str) -> IResult<&str, &str> {
        tag("yesturday")(i)
    }

    fn relative_day(i: &str) -> IResult<&str, DateTime<Utc>> {
        let now_date = Utc::now().date_naive();
        dbg!(i);
        let (i, direction) = alt((value(0, today), value(-1, yesturday), value(1, tomorrow)))(i)?;

        let (i, _) = space0(i)?;

        let (i, timezone) = opt(not_line_ending)(i)?;

        dbg!(timezone);

        let mut result = match timezone {
            Some(timezone) if !timezone.is_empty() => {
                let timezone: Tz = timezone.parse().map_err(|_| {
                    nom::Err::Failure(nom::error::Error::new(i, nom::error::ErrorKind::Fail))
                })?;
                timezone
                    .with_ymd_and_hms(now_date.year(), now_date.month(), now_date.day(), 0, 0, 0)
                    .single()
                    .ok_or_else(|| {
                        nom::Err::Failure(nom::error::Error::new(i, nom::error::ErrorKind::Fail))
                    })?
                    .with_timezone(&Utc)
            }
            _ => Utc
                .with_ymd_and_hms(now_date.year(), now_date.month(), now_date.day(), 0, 0, 0)
                .single()
                .ok_or_else(|| {
                    nom::Err::Failure(nom::error::Error::new(i, nom::error::ErrorKind::Fail))
                })?,
        };

        result = match direction {
            0 => result,
            1 => result + Duration::days(1),
            -1 => result - Duration::days(1),
            _ => unreachable!(),
        };

        Ok((i, result))
    }

    alt((now, relative_day))(i)
}

/// Implementation of [chrono::DateTime] parsing.
/// The output is converted to [chrono::Utc] as timezone.
impl UnitEntry for chrono::DateTime<Utc> {
    type Error = &'static str;
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        if let Ok((_, result)) = datetime(input.as_ref()) {
            Ok(result)
        } else {
            Err("Failed to parse timestamp")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::UnitEntry;
    use chrono::{DateTime, TimeZone, Timelike, Utc};

    fn test_pairs(pair: &[(&str, DateTime<Utc>)]) {
        for each in pair {
            let parse: DateTime<Utc> = UnitEntry::parse_from_str(each.0).unwrap();
            assert_eq!(parse, each.1);
            println!("{} passed.", each.0);
        }
    }

    #[test]
    fn test_datetime() {
        let pairs = vec![
            (
                "Fri 2012-11-23 11:12:13",
                Utc.with_ymd_and_hms(2012, 11, 23, 11, 12, 13).unwrap(),
            ),
            (
                "2012-11-23 11:12:13",
                Utc.with_ymd_and_hms(2012, 11, 23, 11, 12, 13).unwrap(),
            ),
            (
                "2012-11-23 11:12:13 UTC",
                Utc.with_ymd_and_hms(2012, 11, 23, 11, 12, 13).unwrap(),
            ),
            (
                "2012-11-23",
                Utc.with_ymd_and_hms(2012, 11, 23, 0, 0, 0).unwrap(),
            ),
            (
                "12-11-23",
                Utc.with_ymd_and_hms(2012, 11, 23, 0, 0, 0).unwrap(),
            ),
            (
                "11:12:13",
                Utc::now()
                    .with_hour(11)
                    .unwrap()
                    .with_minute(12)
                    .unwrap()
                    .with_second(13)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap(),
            ),
            (
                "11:12",
                Utc::now()
                    .with_hour(11)
                    .unwrap()
                    .with_minute(12)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap(),
            ),
            // cannot be tested due to processing time
            //             ("now", Utc::now()),
            (
                "today",
                Utc::now()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap(),
            ),
            // cannot be tested due to processing time
            //            ("+3h30min", Utc::now() + Duration::minutes(3 * 60 + 30)),
            //            ("-5s", Utc::now() - Duration::seconds(5)),
            //            ("11min ago", Utc::now() - Duration::minutes(11)),
            (
                "@1395716396",
                // different from systemd examples
                Utc.with_ymd_and_hms(2014, 3, 25, 2, 59, 56).unwrap(),
            ),
        ];

        test_pairs(&pairs);
    }
}
