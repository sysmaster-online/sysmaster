use std::io::Error;

pub(crate) struct Config(i32);
impl Config {
    pub fn new() -> Self {
        Self(0)
    }

    pub async fn load(&self) -> Result<(), Error> {
        Ok(())
    }
}
