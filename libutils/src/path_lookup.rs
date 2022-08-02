use std::env;

const ETC_SYSTEM_PATH: &str = "/etc/process1";
const RUN_SYSTEM_PATH: &str = "/run/process1";
const LIB_SYSTEM_PATH: &str = "/usr/lib/process1";

#[derive(Debug, Clone)]
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
        let devel_path = || {
            let out_dir = env::var("OUT_DIR").unwrap_or_else(|_x| {
                let ld_path = env::var("LD_LIBRARY_PATH").map_or("".to_string(), |_v| {
                    let _tmp = _v.split(":").collect::<Vec<_>>()[0];
                    let _tmp_path = _tmp.split("target").collect::<Vec<_>>()[0];
                    _tmp_path.to_string()
                });
                ld_path
            });
            out_dir
        };

        self.search_path.push(LIB_SYSTEM_PATH.to_string());
        self.search_path.push(RUN_SYSTEM_PATH.to_string());
        self.search_path.push(ETC_SYSTEM_PATH.to_string());
        let out_dir = devel_path();
        if !out_dir.is_empty() && out_dir.contains("build") {
            let tmp_str: Vec<_> = out_dir.split("build").collect();
            self.search_path.push(format!("{}", tmp_str[0]));
        }
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
