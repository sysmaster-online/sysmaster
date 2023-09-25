//! Definitions for parsing-related traits.
use crate::{
    error::ReadFileSnafu,
    internal::Error,
    parser::{SectionParser, UnitParser},
    template::{unit_type, UnitType},
};
use snafu::ResultExt;
use std::{
    ffi::OsString,
    fs::{canonicalize, read_dir, File},
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

/// Result of a [UnitParser].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The trait that needs to be implemented on the most-outer struct,
/// representing a type of unit.
pub trait UnitConfig: Sized {
    /// The suffix of a type of unit, parsed from an attribute.
    const SUFFIX: &'static str;
    /// Parses the unit from a [UnitParser].
    fn __parse_unit(__source: UnitParser) -> Result<Self>;
    /// Parses the unit from a [UnitParser], but only patches the supplied entries onto the given struct.
    fn __patch_unit(__source: UnitParser, __from: &mut Self) -> Result<()>;

    /// A convenient function that opens the file that needs to be loaded.
    fn __load<S: AsRef<Path>>(
        path: S,
        paths: Rc<Vec<PathBuf>>,
        filename: &str,
        root: bool,
    ) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path).context(ReadFileSnafu {
            path: path.to_string_lossy().to_string(),
        })?;
        let mut content = String::new();
        file.read_to_string(&mut content).context(ReadFileSnafu {
            path: path.to_string_lossy().to_string(),
        })?;
        let canonical_path = canonicalize(path).unwrap_or_else(|_| path.into());
        let parser = crate::parser::UnitParser::new(
            content.as_ref(),
            paths,
            (root, filename, &canonical_path),
        );
        Self::__parse_unit(parser)
    }

    /// A convenient function that opens the file that needs to be patched.
    fn __patch<S: AsRef<Path>>(
        path: S,
        paths: Rc<Vec<PathBuf>>,
        filename: &str,
        from: &mut Self,
        root: bool,
    ) -> Result<()> {
        let path = path.as_ref();
        let mut file = File::open(path).context(ReadFileSnafu {
            path: path.to_string_lossy().to_string(),
        })?;
        let mut content = String::new();
        file.read_to_string(&mut content).context(ReadFileSnafu {
            path: path.to_string_lossy().to_string(),
        })?;
        let parser =
            crate::parser::UnitParser::new(content.as_ref(), paths, (root, filename, path));
        Self::__patch_unit(parser, from)
    }

    /// Loads a unit with the given name and search paths.
    /// If no suffix is given, the unit's defined suffix will be added to the end.
    fn load_named<S: AsRef<str>, P: AsRef<Path>>(
        paths: Vec<P>,
        name: S,
        root: bool,
    ) -> Result<Self> {
        // return when first one is found?
        let paths: Vec<PathBuf> = paths.iter().map(|x| x.as_ref().to_path_buf()).collect();
        let paths_rc = Rc::new(paths);
        let name = name.as_ref();
        let fullname = if name.ends_with(Self::SUFFIX) {
            name.to_string()
        } else {
            format!("{}.{}", name, Self::SUFFIX)
        };
        let actual_file_name = match unit_type(fullname.as_str())? {
            UnitType::Template(_) => {
                return Err(Error::LoadTemplateError {
                    name: fullname.to_owned(),
                });
            }
            UnitType::Instance(_, template_filename) => template_filename,
            UnitType::Regular(_) => fullname.to_owned(),
        };
        let mut result = None;

        // load itself
        let paths = Rc::clone(&paths_rc);
        for dir in (*paths).iter() {
            let mut path = dir.to_owned();
            path.push(actual_file_name.as_str());
            if let Ok(res) = Self::__load(path, Rc::clone(&paths_rc), fullname.as_str(), root) {
                result = Some(res);
                break;
            }
        }

        let mut result = if let Some(result) = result {
            result
        } else {
            return Err(Error::NoUnitFoundError {
                name: name.to_string(),
            });
        };

        // load drop-ins
        let mut dropin_dir_names = vec![
            format!("{}.d", Self::SUFFIX),
            format!("{}.d", fullname.as_str()),
        ];
        let segments: Vec<&str> = fullname.split('-').collect();
        for i in (1..segments.len()).rev() {
            let segmented = segments[0..i].join("-");
            let dir_name = format!("{}-.{}.d", segmented, Self::SUFFIX);
            dropin_dir_names.push(dir_name);
        }

        for dir_name in dropin_dir_names.iter() {
            for dir in (*paths).iter() {
                let mut path = dir.to_owned();
                path.push(dir_name.as_str());
                if !path.is_dir() {
                    continue;
                }
                if let Ok(dir_entries) = read_dir(&path) {
                    for entry in dir_entries.flatten() {
                        if let (Ok(filetype), Some(extension)) =
                            (entry.file_type(), entry.path().extension())
                        {
                            if filetype.is_file() && extension == "conf" {
                                let paths = Rc::clone(&paths_rc);
                                if let Err(err) = Self::__patch(
                                    entry.path(),
                                    paths,
                                    fullname.as_str(),
                                    &mut result,
                                    root,
                                ) {
                                    log::warn!("Failed to patch unit {}: {})", name, err);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// The trait that needs to be implemented on each section of the unit.
pub trait UnitSection: Sized {
    /// Parses the section from a [SectionParser].
    fn __parse_section(__source: &mut SectionParser) -> Result<Option<Self>>;
    /// Parses the section from a [SectionParser], but only patches the supplied entries onto the given struct.
    fn __patch_section(__source: &mut SectionParser, __from: &mut Self) -> Result<()>;
}

/// The trait that needs to be implemented on each entry of the unit.
/// The crate has already implemented this trait for most common types, see Readme for more information.
/// To add support for a custom type, implement the [UnitEntry::parse_from_str] function similar to [std::str::FromStr].
pub trait UnitEntry: Sized {
    /// Possible parsing error.
    type Error;
    /// Parse the type from [str].
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error>;
}

/// Implement [UnitEntry] for types that also implements [std::str::FromStr].
macro_rules! impl_for_types {
    ($typ:ty) => {
        impl UnitEntry for $typ {
            type Error = <$typ as FromStr>::Err;
            fn parse_from_str<S: AsRef<str>>(
                input: S,
            ) -> std::result::Result<Self, Self::Error> {
                Self::from_str(input.as_ref())
            }
        }
    };
    ($x:ty, $($y:ty),+) => {
        impl_for_types!($x);
        impl_for_types!($($y),+);
    };
}

impl_for_types!(
    IpAddr,
    SocketAddr,
    char,
    f32,
    f64,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    OsString,
    Ipv4Addr,
    Ipv6Addr,
    SocketAddrV4,
    SocketAddrV6,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroIsize,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128,
    NonZeroUsize,
    PathBuf,
    String
);

/// Implement [UnitEntry] for [bool] according to systemd specifications.
impl UnitEntry for bool {
    type Error = ();
    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        match input.as_ref() {
            "1" | "yes" | "true" | "on" => Ok(true),
            "0" | "no" | "false" | "off" => Ok(false),
            _ => Err(()),
        }
    }
}
