use std::{fs, io, os::linux::fs::MetadataExt};

pub const SYSTEM_DATA_UNIT_DIR: &str = "/lib/systemd/system";
pub const RC_LOCAL_PATH: &str = "/etc/rc.local";
pub const B_EXEC: u32 = 0o100; /*判断文件是否可执行 */

pub fn mkdir_parents_lable(path: &str) -> io::Result<()> {
    if path.is_empty() {
        let e = Result::Err(io::ErrorKind::NotFound);
        return e?;
    }

    let size = path.rfind("/").unwrap_or(0);

    /*没有解析出目录说明不用创建，直接返回成功*/
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
    match e {
        Err(a) => {
            if a.kind() == io::ErrorKind::AlreadyExists {
                log::debug!("symlink already exists");
                return Ok(());
            }

            log::debug!("Failed to create symlink {}", to);
            return Err(a);
        }

        _ => {}
    }

    return Ok(());
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
    use std::ffi::CString;
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
        let str_from = "/tmp".to_string() + &"/".to_string() + &"multi-user.target".to_string();
        let e = add_symlink("rc-local.service", &str_from);
        match e {
            Err(a) => {
                if a.kind() == io::ErrorKind::NotFound {
                    panic!("{} does not exist!", RC_LOCAL_PATH);
                }

                panic!(
                    "Couldn't determine if {} exists and is executable, skipping",
                    RC_LOCAL_PATH
                );
            }
            Ok(_) => {}
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
        let file = fs::File::create("test.exec").unwrap();
        unsafe {
            libc::chmod(CString::new("test.exec").unwrap().as_ptr(), 0o777);
        }
        let is_exec = match check_executable("test.exec") {
            Ok(()) => true,
            Err(_) => false,
        };
        assert!(is_exec);
        fs::remove_file("test.exec").unwrap();
    }
}
