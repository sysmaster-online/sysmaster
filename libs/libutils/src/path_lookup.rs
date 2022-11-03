//! the management of the unit file lookup path
use std::env;

const ETC_SYSTEM_PATH: &str = "/etc/sysmaster";
const RUN_SYSTEM_PATH: &str = "/run/sysmaster";
const LIB_SYSTEM_PATH: &str = "/usr/lib/sysmaster";

/// struct LookupPaths
#[derive(Debug, Clone)]
pub struct LookupPaths {
    /// Used to search fragment, dropin, updated
    pub search_path: Vec<String>,
    /// Used to search preset file
    pub preset_path: Vec<String>,
    /// generator paths
    pub generator: String,
    /// generator early paths
    pub generator_early: String,
    /// generator late paths
    pub generator_late: String,
    /// transient paths
    pub transient: String,
    /// transient paths
    pub persistent_path: String,
}

impl LookupPaths {
    /// new
    pub fn new() -> Self {
        LookupPaths {
            generator: String::from(""),
            generator_early: String::from(""),
            generator_late: String::from(""),
            transient: String::from(""),
            search_path: Vec::new(),
            persistent_path: String::from(""),
            preset_path: Vec::new(),
        }
    }

    /// init lookup paths
    pub fn init_lookup_paths(&mut self) {
        let devel_path = || {
            let out_dir = env::var("OUT_DIR").unwrap_or_else(|_x| {
                let _tmp_str: Option<&'static str> = option_env!("OUT_DIR");
                _tmp_str.unwrap_or("").to_string()
            });
            if out_dir.is_empty() {
                env::var("LD_LIBRARY_PATH").map_or("".to_string(), |_v| {
                    let _tmp = _v.split(':').collect::<Vec<_>>()[0];
                    let _tmp_path = _tmp.split("target").collect::<Vec<_>>()[0];
                    _tmp_path.to_string()
                })
            } else {
                out_dir
            }
        };

        let out_dir = devel_path();
        if !out_dir.is_empty() && out_dir.contains("build") {
            let tmp_str: Vec<_> = out_dir.split("build").collect();
            self.search_path.push(tmp_str[0].to_string());
            self.preset_path.push(tmp_str[0].to_string());
        }
        self.search_path.push(LIB_SYSTEM_PATH.to_string());
        self.search_path.push(RUN_SYSTEM_PATH.to_string());
        self.search_path.push(ETC_SYSTEM_PATH.to_string());

        self.preset_path
            .push(format!("{}/{}", ETC_SYSTEM_PATH, "system-preset"));
        self.preset_path
            .push(format!("{}/{}", LIB_SYSTEM_PATH, "system-preset"));

        self.persistent_path = ETC_SYSTEM_PATH.to_string();
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
        let tmp_dir = env::var("OUT_DIR");
        if tmp_dir.is_err() {
            return;
        }
        let tmp = tmp_dir.unwrap();
        let tmp_dir_v: Vec<_> = tmp.split("build").collect();
        assert_eq!(
            _lp.search_path.first().unwrap().to_string(),
            tmp_dir_v[0].to_string()
        );
    }
}
