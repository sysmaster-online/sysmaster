//! Parser struct definitions.
use crate::specifiers::{resolve, SpecifierContext};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_a, is_not, tag, take_till, take_until},
    character::complete::{alphanumeric1, anychar, char, multispace1, none_of, space0},
    combinator::value,
    multi::{many0, many_till},
    sequence::{delimited, separated_pair},
    IResult,
};
use std::{fs::read_dir, path::PathBuf, rc::Rc};

// TODO: error callsite marking

/// A parser for parsing a whole unit.
pub struct UnitParser<'a> {
    /// Search paths
    paths: Rc<Vec<PathBuf>>,
    /// Parsing cursor
    inner: &'a str,
    /// Specifier resolve context
    context: SpecifierContext<'a>,
}

impl<'a> UnitParser<'a> {
    /// Creates a new [UnitParser] with input, scan paths and specifier resolve context.
    pub fn new(input: &'a str, paths: Rc<Vec<PathBuf>>, context: SpecifierContext<'a>) -> Self {
        UnitParser {
            paths,
            inner: input,
            context,
        }
    }

    /// Moves the inner cursor forward by updating the `inner` field.
    pub fn progress(&mut self, i: &'a str) {
        self.inner = i;
    }
}

/// [UnitParser] is a [std::iter::Iterator] that yields [SectionParser].
impl<'a> Iterator for UnitParser<'a> {
    type Item = SectionParser<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Ok((i, name)) = section_header(self.inner) {
                self.inner = i;
                return Some(SectionParser {
                    paths: Rc::clone(&self.paths),
                    name,
                    inner: self.inner,
                    context: self.context,
                });
            } else {
                let temp: IResult<&str, &str> = take_until("\n")(self.inner);
                match temp {
                    Ok((i, _)) => {
                        self.inner = i.trim_start_matches('\n');
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        }
    }
}

/// Parses a section header.
fn section_header(i: &str) -> IResult<&str, &str> {
    let (i, _) = gaps(i)?;
    let (i, result) = delimited(char('['), alphanumeric1, char(']'))(i)?;
    let (i, _) = multispace1(i)?;
    let (i, _) = gaps(i)?;
    Ok((i, result))
}

/// A parser for parsing a section.
pub struct SectionParser<'a> {
    /// Specifier resolve context
    paths: Rc<Vec<PathBuf>>,
    /// Section name
    pub name: &'a str,
    /// Parsing cursor
    inner: &'a str,
    /// Specifier resolve context
    context: SpecifierContext<'a>,
}

impl<'a> SectionParser<'a> {
    /// Returns the inner cursor when a section reaches it end.
    pub fn finish(self) -> &'a str {
        self.inner
    }
}

/// [SectionParser] is a [std::iter::Iterator] that yields [(&str, String)], which represents a key-value pair.
impl<'a> Iterator for SectionParser<'a> {
    type Item = (&'a str, String);
    fn next(&mut self) -> Option<Self::Item> {
        if let Ok((i, result)) = entry(self.inner, self.context) {
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
                for entry in read_res.flatten() {
                    // only look for symlinks
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.file_type().is_symlink() {
                            result.push(entry.file_name().to_string_lossy().to_string());
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
/// Parses an entry.
fn entry<'a>(i: &'a str, context: SpecifierContext<'a>) -> IResult<&'a str, (&'a str, String)> {
    let (i, result) = separated_pair(
        alphanumeric1,
        delimited(space0, char('='), space0),
        entry_value(context),
    )(i)?;
    let (i, _) = gaps(i)?;
    Ok((i, result))
}

/// Parses the value of an entry.
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

/// Parses a segment of a value, ending with an optional specifier.
fn value_segment<'a>(
    context: SpecifierContext<'a>,
) -> impl FnMut(&'a str) -> IResult<&'a str, String> {
    move |i| {
        let (i, segment) = take_till(|x| x == '\n' || x == '%')(i)?;
        /* escape the '\\' in segment to '\' */
        let excaped_segment: IResult<&str, String> =
            escaped_transform(none_of("\\"), '\\', value("\\", tag("\\")))(segment);
        let segment = match excaped_segment {
            Err(_) => segment.to_string(),
            Ok(v) => v.1,
        };
        if let Ok((i, spec)) = specifier(i) {
            let mut result = segment;
            if resolve(&mut result, spec, context).is_ok() {
                Ok((i, result))
            } else {
                Err(nom::Err::Failure(nom::error::Error::new(
                    i,
                    nom::error::ErrorKind::Fail,
                )))
            }
        } else {
            Ok((i, segment))
        }
    }
}

/// Parses a specifier.
fn specifier(i: &str) -> IResult<&str, char> {
    let (i, _) = char('%')(i)?;
    anychar(i)
}

/// Parses spaces, newlines and comments.
fn gaps(i: &str) -> IResult<&str, ()> {
    let comment = delimited(is_a("#;"), is_not("\n\r"), is_a("\n\r"));
    value((), many0(alt((multispace1, comment))))(i)
}
