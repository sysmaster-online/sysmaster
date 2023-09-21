//! Parser struct definitions.
use crate::specifiers::{resolve, SpecifierContext};
use nom::{
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take_till},
    character::complete::{alphanumeric1, anychar, char, multispace0, multispace1, space0},
    combinator::value,
    multi::many_till,
    sequence::{delimited, separated_pair, tuple},
    IResult,
};
use std::{fs::read_dir, path::PathBuf, rc::Rc};

pub struct UnitParser<'a> {
    paths: Rc<Vec<PathBuf>>,
    // the shared parsing cursor
    inner: &'a str,
    context: SpecifierContext<'a>,
}

// use a progress function to update inner cursor
// when a section parser finishes
impl<'a> UnitParser<'a> {
    pub fn new(input: &'a str, paths: Rc<Vec<PathBuf>>, context: SpecifierContext<'a>) -> Self {
        UnitParser {
            paths,
            inner: input,
            context,
        }
    }

    pub fn progress(&mut self, i: &'a str) {
        self.inner = i;
    }

    pub fn next(&mut self) -> Option<SectionParser<'a>> {
        if let Ok((i, name)) = section_header(self.inner) {
            dbg!(name);
            self.inner = i;
            Some(SectionParser {
                paths: Rc::clone(&self.paths),
                name,
                inner: self.inner,
                context: self.context,
            })
        } else {
            None
        }
    }
}

fn section_header(i: &str) -> IResult<&str, &str> {
    let (i, result) = delimited(char('['), alphanumeric1, char(']'))(i)?;
    let (i, _) = multispace1(i)?;
    Ok((i, result))
}

pub struct SectionParser<'a> {
    paths: Rc<Vec<PathBuf>>,
    pub name: &'a str,
    // the shared parsing cursor
    inner: &'a str,
    context: SpecifierContext<'a>,
}

impl<'a> SectionParser<'a> {
    pub fn finish(self) -> &'a str {
        dbg!(self.inner);
        self.inner
    }

    pub fn next(&mut self) -> Option<(&str, String)> {
        if let Ok((i, result)) = entry(self.inner.as_ref(), self.context) {
            dbg!(&result);
            self.inner = i;
            Some(result)
        } else {
            None
        }
    }
}

impl<'a> SectionParser<'a> {
    /// Parses subdirs from paths.
    pub fn __parse_subdir(&self, subdir: &str) -> Vec<String> {
        let mut result = Vec::new();
        for dir in (*self.paths).iter() {
            let mut path = dir.to_owned();
            let path_end = format!("{}.{}", self.context.1, subdir);
            path.push(path_end.as_str());
            if let Ok(read_res) = read_dir(path) {
                for item in read_res {
                    if let Ok(entry) = item {
                        // only look for symlinks
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.file_type().is_symlink() {
                                result.push(entry.file_name().to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
        result
    }
}

// returns (key, value) pair
// specifiers are resolved in the process, leading to string copies
fn entry<'a>(i: &'a str, context: SpecifierContext<'a>) -> IResult<&'a str, (&'a str, String)> {
    let (i, result) = separated_pair(
        alphanumeric1,
        delimited(space0, char('='), space0),
        entry_value(context),
    )(i)?;
    let (i, _) = multispace0(i)?;
    Ok((i, result))
}

fn entry_value<'a>(
    context: SpecifierContext<'a>,
) -> impl FnMut(&'a str) -> IResult<&'a str, String> {
    move |i| {
        let mut result = String::new();
        let mut i = i;
        loop {
            let (new_i, (segments, terminator)) =
                many_till(value_segment(context), alt((tag("\\\n"), tag("\n"))))(i)?;
            result.extend(segments.into_iter());
            i = new_i;

            if terminator == "\n" {
                break;
            }
        }

        Ok((i, result))
    }
}

fn value_segment<'a>(
    context: SpecifierContext<'a>,
) -> impl FnMut(&'a str) -> IResult<&'a str, String> {
    move |i| {
        let (i, segment) = take_till(|x| x == '\\' || x == '\n' || x == '%')(i)?;
        if let Ok((i, spec)) = specifier(i) {
            let mut result = segment.to_string();
            if let Ok(_) = resolve(&mut result, spec, context) {
                Ok((i, result))
            } else {
                Err(nom::Err::Failure(nom::error::Error::new(
                    i,
                    nom::error::ErrorKind::Fail,
                )))
            }
        } else {
            Ok((i, segment.to_string()))
        }
    }
}

fn specifier(i: &str) -> IResult<&str, char> {
    let (i, _) = char('%')(i)?;
    anychar(i)
}

fn comment(i: &str) -> IResult<&str, ()> {
    value((), tuple((is_a("#;"), is_not("\n\r"), is_a("\n\r"))))(i)
}
