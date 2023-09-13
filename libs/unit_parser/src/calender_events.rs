use std::fmt::Display;

use chrono::prelude::*;

use crate::{config::UnitEntry, datetime::Rule};

#[derive(Debug)]
pub enum CalenderEvent {
    Once(DateTime<Utc>),
    Repetitive(Schedule),
}

impl UnitEntry for CalenderEvent {
    type Error = pest::error::Error<Rule>;
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        todo!()
    }
}

impl CalenderEvent {
    pub fn validate(&self, input: DateTime<Utc>) -> bool {
        todo!()
    }

    pub fn upcoming(&self) -> CalenderEventIterator {
        todo!()
    }
}

pub struct CalenderEventIterator {
    inner: CalenderEvent,
    current: DateTime<Utc>,
}

impl Iterator for CalenderEventIterator {
    type Item = DateTime<Utc>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Schedule {}

#[derive(Debug)]
enum ValidType<T: PartialEq> {
    All,
    AnyOf(Vec<T>),
}

type IntRepitition = Option<u64>;
type FracRepitition = Option<f64>;
type LastDays = Option<u64>;

impl<T: PartialEq> ValidType<T> {
    fn is_valid(&self, input: &T) -> bool {
        match self {
            ValidType::All => true,
            ValidType::AnyOf(inner) => inner.contains(input),
        }
    }
}

impl<T: PartialEq + Display> Display for ValidType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidType::All => f.write_str("*"),
            ValidType::AnyOf(inner) => {
                let res: String = inner
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                f.write_str(res.as_str())
            }
        }
    }
}
