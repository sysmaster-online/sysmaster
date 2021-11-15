
pub struct LookupPaths {
    search_path: Vec<&'static str>,
    generator: String,
    generator_early: String,
    generator_late: String,
    transient: String,
}

impl LookupPaths{
    pub fn new() -> Self {
        LookupPaths {
            generator: String::from(""),
            generator_early: String::from(""),
            generator_late: String::from(""),
            transient: String::from(""),
            search_path: Vec::new(),
        }
    }
}
