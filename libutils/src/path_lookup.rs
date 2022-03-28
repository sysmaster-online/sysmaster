const ETC_SYSTEM_PATH: &'static str = "/etc/process1/system";
const LIB_SYSTEM_PATH: &'static str = "/usr/lib/process1/system";

#[derive(Debug)]
pub struct LookupPaths {
    pub search_path: Vec<&'static str>,
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
        self.search_path.push(ETC_SYSTEM_PATH);
        self.search_path.push(LIB_SYSTEM_PATH);
    }
}
