use std::{fs::File, io::Read};

use process1::manager::UnitObj;

use crate::{service_base::ServiceCommand, ServiceUnit};

#[test]
fn test_service_parse() {
    let file_path = "../../libutils/examples/config.service";
    let mut file = File::open(file_path).unwrap();
    let mut buf = String::new();
    match file.read_to_string(&mut buf) {
        Ok(s) => s,
        Err(_e) => {
            return;
        }
    };

    let mut service_unit = ServiceUnit::new();
    let _result = service_unit.load(buf.as_str());

    assert_ne!(
        service_unit.exec_commands[ServiceCommand::ServiceStart as usize].len(),
        0
    );

    for command in &service_unit.exec_commands[ServiceCommand::ServiceStart as usize] {
        println!(
            "cmd: {}, args: {:?}",
            command.borrow().cmd,
            command.borrow().args
        );
    }
}
