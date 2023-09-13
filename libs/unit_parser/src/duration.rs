//! Parsers of systemd-style durations.
use crate::{
    config::UnitEntry,
    datetime::{DatetimeParser, Rule},
};
use chrono::Duration;
use pest::{iterators::Pairs, Parser};

/// Implementation of [chrono::Duration] parsing.
impl UnitEntry for Duration {
    type Error = pest::error::Error<Rule>;
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        let parse = DatetimeParser::parse(Rule::timespan, input.as_ref())?;
        duration_from_parser(parse)
    }
}

/// Parses [chrono::Duration] from a [crate::datetime::DatetimeParser].
pub(crate) fn duration_from_parser(
    mut parse: Pairs<'_, Rule>,
) -> std::result::Result<Duration, pest::error::Error<Rule>> {
    let timespan = parse.next().unwrap().into_inner();
    let mut result = Duration::zero();
    for segment in timespan {
        if segment.as_rule() == Rule::segment {
            let mut inner = segment.into_inner();
            let number: i64 = inner.next().unwrap().as_str().parse().unwrap();
            let unit = inner.next().unwrap();
            let addition = match unit.as_rule() {
                Rule::usec => Duration::microseconds(number),
                Rule::msec => Duration::milliseconds(number),
                Rule::seconds => Duration::seconds(number),
                Rule::minutes => Duration::minutes(number),
                Rule::hours => Duration::hours(number),
                Rule::days => Duration::days(number),
                Rule::weeks => Duration::weeks(number),
                Rule::months => {
                    Duration::seconds(/* 30.44 * 24 * 60 * 60 = */ 2630016 * number)
                }
                Rule::years => Duration::hours(/* 365.25 * 24 = */ 8766 * number),
                _ => unreachable!(),
            };
            result = result + addition;
        } else {
            let number: i64 = segment.as_str().parse().unwrap();
            result = result + Duration::seconds(number);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use crate::config::UnitEntry;

    fn test_pairs(pair: &Vec<(&str, Duration)>) {
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
