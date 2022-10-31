use std::{fs, io, os::linux::fs::MetadataExt};

const SYSTEM_DATA_UNIT_DIR: &str = "/etc/sysmaster/";
pub const RC_LOCAL_PATH: &str = "/etc/rc.local";
const B_EXEC: u32 = 0o100; /*judge whether file can be executed */

fn mkdir_parents_lable(path: &str) -> io::Result<()> {
    if path.is_empty() {
        let e = Result::Err(io::ErrorKind::NotFound);
        return e?;
    }

    let size = path.rfind('/').unwrap_or(0);

    if 0 == size {
        return Ok(());
    }

    let s = &path[..size];
    println!("{}", s);
    fs::create_dir_all(s)?;

    Ok(())
}

pub fn add_symlink(from_service: &str, to_where: &str) -> io::Result<()> {
    if from_service.is_empty() || to_where.is_empty() {
        let e = Err(io::ErrorKind::NotFound);
        return e?;
    }

    let from = SYSTEM_DATA_UNIT_DIR.to_string() + "/" + from_service;
    let to = to_where.to_string() + ".wants/" + from_service;

    let _ = mkdir_parents_lable(&to);

    let e = std::os::unix::fs::symlink(&from, &to);
    if let Err(a) = e {
        if a.kind() == io::ErrorKind::AlreadyExists {
            log::debug!("symlink already exists");
            return Ok(());
        }

        log::debug!("Failed to create symlink {}", to);
        return Err(a);
    }

    Ok(())
}

pub fn check_executable(file: &str) -> io::Result<()> {
    let e = fs::metadata(file);
    match e {
        Err(a) => {
            if a.kind() == io::ErrorKind::NotFound {
                log::debug!("{} does not exist, skipping", file);
                return Err(a);
            }

            log::debug!(
                "Couldn't determine if {} exists and is executable, skipping",
                file
            );
            return Err(a);
        }
        Ok(a) => {
            let mode = a.st_mode();
            if 0 == B_EXEC & mode {
                log::debug!("{} is not marked executable, skipping", RC_LOCAL_PATH);
                return Err(Err(io::ErrorKind::PermissionDenied)?);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::prelude::PermissionsExt;
    #[test]
    fn mkdir_parents_lable_test() {
        let path = "/tmp/a/";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a/";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a/b/b/";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a/c";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a////d/e//";
        mkdir_parents_lable(path).unwrap();
    }

    #[test]
    #[should_panic]
    fn mkdir_empty_parents_lable_test() {
        let path = "";
        mkdir_parents_lable(path).unwrap();
    }

    #[test]
    fn add_symlink_test() {
        let str_from = "/tmp".to_string() + "/" + "multi-user.target";
        if let Err(e) = add_symlink("rc-local.service", &str_from) {
            if e.kind() == io::ErrorKind::NotFound {
                panic!("{} does not exist!", RC_LOCAL_PATH);
            }

            panic!(
                "Couldn't determine if {} exists and is executable, skipping",
                RC_LOCAL_PATH
            );
        }
    }

    #[test]
    #[should_panic]
    fn add_empty_symlink_test() {
        add_symlink("", "").unwrap();
        /*
        let from_service = "/tmp";
        add_symlink(from_service, "").unwrap();

        let to_where = "/tmp";
        add_symlink("", to_where).unwrap();

        let from_service = "no_exit_file";
        let to_where = "/tmp/tolink";
        add_symlink(from_service, to_where).unwrap();
        */
    }

    #[test]
    fn check_executable_test() {
        fs::File::create("test.exec").unwrap();
        if let Err(e) =
            std::fs::set_permissions("test.exec", std::fs::Permissions::from_mode(0o777))
        {
            println!("Failed to chmod: {}", e);
        }
        let is_exec = match check_executable("test.exec") {
            Ok(()) => true,
            Err(_) => false,
        };
        assert!(is_exec);
        fs::remove_file("test.exec").unwrap();
    }
}
