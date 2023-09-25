//! Parsers of systemd-style durations.
use crate::config::UnitEntry;
use chrono::Duration;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, multispace0, space0},
    combinator::{opt, value},
    multi::many0,
    sequence::delimited,
    IResult,
};

/// Implementation of [chrono::Duration] parsing.
impl UnitEntry for Duration {
    type Error = &'static str;
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        if let Ok((i, result)) = duration(input.as_ref()) {
            if i.is_empty() {
                Ok(result)
            } else {
                // TODO: log what went wrong
                Err("Unexpected characters at the end of duration expression.")
            }
        } else {
            Err("Failed to parse duration expression.")
        }
    }
}

/// Parses a [chrono::Duration], eating up whitespaces after it.
pub(crate) fn duration(i: &str) -> IResult<&str, Duration> {
    let (i, segments) = many0(time_segment)(i)?;
    let mut result = segments
        .into_iter()
        .reduce(|a, b| a + b)
        .unwrap_or_else(Duration::zero);
    // captures a integer without unit
    let (i, opt_int) = opt(digit1)(i)?;
    if let Some(int) = opt_int {
        let int = int.parse().unwrap();
        result = result + Duration::seconds(int);
    }
    let (i, _) = space0(i)?;
    Ok((i, result))
}

fn time_segment(i: &str) -> IResult<&str, Duration> {
    let (i, int) = delimited(multispace0, digit1, multispace0)(i)?;
    let int = int.parse().unwrap();
    alt((
        value(Duration::microseconds(int), usec),
        value(Duration::milliseconds(int), msec),
        value(Duration::seconds(int), sec),
        value(Duration::minutes(int), min),
        value(Duration::hours(int), hr),
        value(Duration::days(int), day),
        value(Duration::weeks(int), week),
        value(
            Duration::seconds(/* 30.44 * 24 * 60 * 60 = */ 2630016 * int),
            month,
        ),
        value(Duration::hours(/* 365.25 * 24 = */ 8766 * int), year),
    ))(i)
}

fn usec(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("us"), tag("μs"), tag("usec")))(i)?;
    Ok((i, ()))
}

fn msec(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("ms"), tag("msec")))(i)?;
    Ok((i, ()))
}

fn sec(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("seconds"), tag("second"), tag("sec"), tag("s")))(i)?;
    Ok((i, ()))
}

fn min(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("minutes"), tag("minute"), tag("min"), tag("m")))(i)?;
    Ok((i, ()))
}

fn hr(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("hours"), tag("hour"), tag("hr"), tag("h")))(i)?;
    Ok((i, ()))
}

fn day(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("days"), tag("day"), tag("d")))(i)?;
    Ok((i, ()))
}

fn week(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("weeks"), tag("week"), tag("w")))(i)?;
    Ok((i, ()))
}

fn month(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("months"), tag("month"), tag("M")))(i)?;
    Ok((i, ()))
}

fn year(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((tag("years"), tag("year"), tag("y")))(i)?;
    Ok((i, ()))
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use crate::config::UnitEntry;

    fn test_pairs(pair: &[(&str, Duration)]) {
        for each in pair {
            let parse = Duration::parse_from_str(each.0).unwrap();
            assert_eq!(parse, each.1);
        }
    }

    #[test]
    fn single() {
        let pairs = vec![
            ("1us", Duration::microseconds(1)),
            ("2ms", Duration::milliseconds(2)),
            ("3s", Duration::seconds(3)),
            ("4m", Duration::minutes(4)),
            ("5h", Duration::hours(5)),
            ("6d", Duration::days(6)),
        ];
        test_pairs(&pairs);
    }

    #[test]
    fn complex() {
        let pairs = vec![
            ("5s400ms", Duration::milliseconds(5400)),
            ("3w 5d", Duration::days(26)),
        ];
        test_pairs(&pairs);
    }

    #[test]
    fn unitless() {
        let parse = Duration::parse_from_str("114").unwrap();
        let target = Duration::seconds(114);
        assert_eq!(parse, target);
    }
}
