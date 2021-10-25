#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}



pub mod front_of_hourse {
    pub mod hosting {
        pub fn add_to_waitlist(name:String) -> Vec<String> {
            let mut result = Vec::new();
            result.push(name);
            return result;
        }
    }
}

pub mod manager;

pub mod plugin;