use std::env;

use crate::Error;

pub fn env_path() -> Result<String, Error> {
    let devel_path = || {
        let out_dir = env::var("OUT_DIR");
        out_dir
    };

    let _tmp_lib_path = devel_path();
    let out_dir = match _tmp_lib_path {
        Ok(v) => v,
        Err(_e) => {
            let ld_path = env::var("LD_LIBRARY_PATH");
            if ld_path.is_err() {
                return Err(Error::Other {
                    msg: "LD_LIBRARY_PATH env is not set",
                });
            }
            let ld_path = ld_path.unwrap();
            let _tmp = ld_path.split(":").collect::<Vec<_>>()[0];
            let _tmp_path = _tmp.split("target").collect::<Vec<_>>()[0];
            _tmp_path.to_string()
        }
    };

    let tmp_str: Vec<_> = out_dir.split("build").collect();
    if tmp_str.len() < 1 {
        return Err(Error::Other {
            msg: "not running with cargo",
        });
    }

    Ok(tmp_str[0].to_string())
}
