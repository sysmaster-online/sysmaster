use std::env;

use crate::env_cargo;

const ETC_SYSTEM_PATH: &str = "/etc/process1/system";
const LIB_SYSTEM_PATH: &str = "/usr/lib/process1/system";

#[derive(Debug)]
pub struct LookupPaths {
    pub search_path: Vec<String>,
    pub generator: String,
    pub generator_early: String,
    pub generator_late: String,
    pub transient: String,
}

impl LookupPaths {
    pub fn new() -> Self {
        LookupPaths {
            generator: String::from(""),
            generator_early: String::from(""),
            generator_late: String::from(""),
            transient: String::from(""),
            search_path: Vec::new(),
        }
    }

    pub fn init_lookup_paths(&mut self) {
        self.search_path.push(ETC_SYSTEM_PATH.to_string());
        self.search_path.push(LIB_SYSTEM_PATH.to_string());

        let env_path = env_cargo::env_path();
        if env_path.is_err() {
            log::debug!("{:?}", env_path);
            return;
        }
        self.search_path.push(format!("{}", env_path.unwrap()));
    }
}

impl Default for LookupPaths {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use std::env;

    use crate::logger;

    use super::LookupPaths;
    #[test]
    fn test_init_lookup_paths() {
        logger::init_log_with_console("test_init_lookup_paths", 4);
        let mut _lp = LookupPaths::default();
        _lp.init_lookup_paths();
        for item in _lp.search_path.iter() {
            log::info!("lookup path is{:?}", item);
        }
        let tmp_dir = env::var("OUT_DIR").unwrap();
        let tmp_dir_v: Vec<_> = tmp_dir.split("build").collect();
        assert_eq!(
            _lp.search_path.last().unwrap().to_string(),
            tmp_dir_v[0].to_string()
        );
    }
}
