use serde::{Deserializer, Deserialize};

pub trait DeserializeWith: Sized {
    type Item;
    ///
    fn deserialize_with<'de, D>(de: D) -> Result<Self::Item, D::Error>
    where
        D: Deserializer<'de>;
}



impl DeserializeWith for Vec<String> {
    type Item = Self;
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        let mut vec = Vec::new();

        for l in s.split_terminator(';') {
            vec.push(l.trim().to_string());
        }

        Ok(vec)
    }
}
