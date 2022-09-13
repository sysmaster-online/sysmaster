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

#[cfg(test)]
mod tests {
    use crate::get_project_root;

    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("libutils/examples/config.service.toml");

        println!("{:?}", file_path);
        assert_eq!(file_path, file_path);
    }
}
