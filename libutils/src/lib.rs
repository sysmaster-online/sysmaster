pub mod conf_parser;
pub mod config_parser;
pub mod error;
pub mod io_util;
pub mod logger;
pub mod macros;
pub mod path_lookup;

pub mod env_cargo;
pub mod fd_util;
pub mod file_util;
pub mod fs_util;
pub mod mount_util;
pub mod path_util;
pub mod proc_cmdline;
pub mod process_util;
pub mod socket_util;
pub mod time_util;

pub mod condition;
pub use anyhow::*;
pub use error::Error;
pub use error::Result;

#[cfg(test)]
pub mod test_util {
    use std::{
        env,
        ffi::OsString,
        fs::read_dir,
        io::{self, ErrorKind},
        path::PathBuf,
    };

    pub fn get_project_root() -> io::Result<PathBuf> {
        let path = env::current_dir()?;
        let mut path_ancestors = path.as_path().ancestors();

        while let Some(p) = path_ancestors.next() {
            let has_cargo = read_dir(p)?
                .into_iter()
                .any(|p| p.unwrap().file_name() == OsString::from("Cargo.lock"));
            if has_cargo {
                return Ok(PathBuf::from(p));
            }
        }
        Err(io::Error::new(
            ErrorKind::NotFound,
            "Ran out of places to find Cargo.toml",
        ))
    }
}
