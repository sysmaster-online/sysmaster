use super::{UnitTrait, UnitType};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Socket(i32);
impl Socket {
    pub fn new() -> Self {
        todo!()
    }
}

impl UnitTrait for Socket {
    fn start(&self) -> bool {
        todo!()
    }

    fn stop(&self) -> bool {
        todo!()
    }

    fn load(&self) -> bool {
        todo!()
    }

    fn kind(&self) -> UnitType {
        UnitType::Socket
    }
}
