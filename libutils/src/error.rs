use snafu::prelude::*;
use std::{io, path::PathBuf};

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Unable to read configuration from {}: {}", path.display(), source))]
    ReadConfiguration { source: io::Error, path: PathBuf },

    #[snafu(display("Unable to write result to {}: {}", path.display(), source))]
    WriteResult { source: io::Error, path: PathBuf },
}

#[cfg(test)]

mod tests {
    use super::*;
    use std::fs;

    type Result<T, E = Error> = std::result::Result<T, E>;

    #[test]
    fn process_data() -> Result<()> {
        let path = "../Cargo.toml";
        //ReadConfigurationSnafu must add Snafu suffix.
        let _ = fs::read_to_string(path).context(ReadConfigurationSnafu { path })?;
        Ok(())
    }
}
