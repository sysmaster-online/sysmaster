use crate::service_base::ServiceConf;

use super::service::ServiceUnit;
use super::service_base::ServiceCommand;
use super::service_base::{CommandLine, ServiceType};
use std::collections::LinkedList;
use std::error::Error;
use std::path::Path;
use std::str::FromStr;
use std::{cell::RefCell, rc::Rc};
use utils::unit_conf::{Conf, ConfValue, Section};

impl ServiceUnit {
    pub(super) fn parse(&mut self, section: &Section<Conf>) -> Result<(), Box<dyn Error>> {
        //self.unit.upgrade().as_ref().cloned().unwrap().get_id();
        let confs = section.get_confs();
        for conf in confs.iter() {
            let key = conf.get_key();
            match key.to_string() {
                _ if key == ServiceConf::ExecCondition.to_string() => {
                    let values = conf.get_values();
                    self.exec_commands[ServiceCommand::ServiceCondition as usize] =
                        LinkedList::new();
                    prepare_command(
                        &values,
                        &mut self.exec_commands[ServiceCommand::ServiceCondition as usize],
                    );
                }
                _ if key == ServiceConf::ExecStart.to_string() => {
                    let values = conf.get_values();
                    self.exec_commands[ServiceCommand::ServiceStart as usize] = LinkedList::new();
                    prepare_command(
                        &values,
                        &mut self.exec_commands[ServiceCommand::ServiceStart as usize],
                    );
                }
                _ if key == ServiceConf::ExecReload.to_string() => {
                    let values = conf.get_values();
                    self.exec_commands[ServiceCommand::ServiceReload as usize] = LinkedList::new();
                    prepare_command(
                        &values,
                        &mut self.exec_commands[ServiceCommand::ServiceReload as usize],
                    );
                }
                _ if key == ServiceConf::Type.to_string() => {
                    let values = conf.get_values();
                    for value in values.iter() {
                        if let ConfValue::String(v) = value {
                            self.service_type = ServiceType::from_str(v)?;
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn prepare_command(
    commands: &Vec<ConfValue>,
    command_list: &mut LinkedList<Rc<RefCell<CommandLine>>>,
) -> Result<(), Box<dyn Error>> {
    if commands.len() == 0 {
        return Ok(());
    }
    let mut i = 0;
    for exec in commands.iter() {
        let mut cmd = "";
        let mut t_args: Vec<String> = Vec::new();
        if let ConfValue::String(t_cmd) = exec {
            if i == 0 {
                cmd = t_cmd;
                i = i + 1;
            } else {
                t_args.push(t_cmd.to_string());
            }
        } else {
            return Err(format!(
                "service config  format is error, command {:?} is error",
                exec
            )
            .into());
        }

        if cmd.is_empty() {
            return Ok(());
        }
        let path = Path::new(&cmd);
        if !path.exists() || !path.is_file() {
            return Err(format!("{:?} is not exist or commad is not a file", path).into());
        }

        let new_command = Rc::new(RefCell::new(CommandLine {
            cmd: path.to_str().unwrap().to_string(),
            args: t_args,
            next: None,
        }));
        match command_list.back() {
            Some(command) => {
                command.borrow_mut().next = Some(new_command.clone());
            }
            None => {}
        }

        command_list.push_back(new_command.clone());
    }

    Ok(())
}
