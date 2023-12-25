//! Definitions for parsing-related traits.
use crate::{
    error::ReadFileSnafu,
    internal::Error,
    parser::{SectionParser, UnitParser},
};
use snafu::ResultExt;
use std::{
    ffi::OsString,
    fs::File,
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
pub trait UnitConfig: Sized + Default {
    /// The suffix of a type of unit, parsed from an attribute.
    const SUFFIX: &'static str;

    /// Parses the unit from a [UnitParser].
    fn __parse_unit(__source: UnitParser, res: &mut Self) -> Result<()>;

    /// Load the default value
    fn __load_default(__res: &mut Self);

    /// A convenient function that opens the file that needs to be loaded.
    fn __load<S: AsRef<Path>>(path: S, unit_name: &str, res: &mut Self) -> Result<()> {
        let path = path.as_ref();
        let mut file = File::open(path).context(ReadFileSnafu { path })?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .context(ReadFileSnafu { path })?;
        let parser = crate::parser::UnitParser::new(content.as_ref(), (unit_name,));
        Self::__parse_unit(parser, res)
    }

    /// Loads a unit with the given config file list and unit name
    ///
    /// paths: full paths of the given config file
    ///
    /// unit_name: the full unit name
    fn load_config<P: AsRef<Path>>(paths: Vec<P>, unit_name: &str) -> Result<Self> {
        // return when first one is found?
        let paths: Vec<PathBuf> = paths.iter().map(|x| x.as_ref().to_path_buf()).collect();
        let paths_rc = Rc::new(paths);
        let fullname = unit_name.to_string();

        let mut result = Self::default();
        Self::__load_default(&mut result);

        // load itself
        let paths = Rc::clone(&paths_rc);
        for dir in (*paths).iter() {
            let _ = Self::__load(dir, &fullname, &mut result);
        }

        Ok(result)
    }
}

/// The trait that needs to be implemented on each section of the unit.
pub trait UnitSection: Sized {
    /// Parses the section from a [SectionParser].
    fn __parse_section(__source: &mut SectionParser, res: &mut Self) -> Result<()>;
    /// Load the default value
    fn __load_default(__res: &mut Self);
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
