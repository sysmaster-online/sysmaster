use std::{fs::File, io::Read};

use process1::manager::UnitObj;
use utils::{
    config_parser::{ConfigParse, ConfigParser},
    unit_conf::{ConfFactory, Confs, Section, SectionType},
};

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

    struct MockDefaultFatory(String);
    impl ConfFactory for MockDefaultFatory {
        fn product_confs(&self) -> utils::unit_conf::Confs {
            let mut confs = Confs::new("service".to_string());
            let unit_section = Section::new("Unit".to_string(), SectionType::PUB);
            let service_section = Section::new("Service".to_string(), SectionType::PRIVATE);
            let install_section = Section::new("Install".to_string(), SectionType::PUB);
            confs.add_section(unit_section);
            confs.add_section(service_section);
            confs.add_section(install_section);
            confs
        }
    }
    let default_factory = MockDefaultFatory("Service".to_string());

    let config_parse = ConfigParser::new("service".to_string(), default_factory);
    let confs = config_parse.toml_file_parse(&buf).unwrap();

    let section = confs.get_section_by_name("Service");
    let mut service_unit = ServiceUnit::new();

    let _result = section.map(|s| service_unit.load(s));

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
