use std::io::BufRead;
use std::io::BufReader;
use std::io::Error;
use std::path::Path;

pub fn read_first_line(path: &Path) -> Result<String, Error> {
    let file = std::fs::File::open(path)?;

    let mut buffer = BufReader::new(file);
    let mut first_line = String::with_capacity(1024);

    let _length = buffer.read_line(&mut first_line);

    Ok(first_line)
}
