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
        let confs = section.get_confs();
        for conf in confs.iter() {
            let key = conf.get_key();

            if key == ServiceConf::ExecCondition.to_string() {
                let values = conf.get_values();
                self.exec_commands[ServiceCommand::ServiceCondition as usize] = LinkedList::new();
                match prepare_command(
                    &values,
                    &mut self.exec_commands[ServiceCommand::ServiceCondition as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            if key == ServiceConf::ExecStart.to_string() {
                let values = conf.get_values();
                self.exec_commands[ServiceCommand::ServiceStart as usize] = LinkedList::new();
                match prepare_command(
                    &values,
                    &mut self.exec_commands[ServiceCommand::ServiceStart as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("prepare command return err: {}", e.to_string());
                        return Err(e);
                    }
                }
            }

            if key == ServiceConf::ExecStop.to_string() {
                let values = conf.get_values();
                self.exec_commands[ServiceCommand::ServiceStop as usize] = LinkedList::new();
                match prepare_command(
                    &values,
                    &mut self.exec_commands[ServiceCommand::ServiceStop as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("prepare command return err: {}", e.to_string());
                        return Err(e);
                    }
                }
            }

            if key == ServiceConf::ExecReload.to_string() {
                let values = conf.get_values();
                self.exec_commands[ServiceCommand::ServiceReload as usize] = LinkedList::new();
                match prepare_command(
                    &values,
                    &mut self.exec_commands[ServiceCommand::ServiceReload as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            if key == ServiceConf::Type.to_string() {
                let values = conf.get_values();
                for value in values.iter() {
                    if let ConfValue::String(v) = value {
                        self.service_type = ServiceType::from_str(v)?;
                        break;
                    }
                }
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

    for exec in commands.iter() {
        let cmd = match exec {
            ConfValue::String(s) => s,
            _ => {
                return Err(format!(
                    "service config  format is error, command {:?} is error",
                    exec
                )
                .into())
            }
        };

        if cmd.is_empty() {
            return Ok(());
        }

        let mut command: Vec<String> = cmd
            .trim_end()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // get the command and leave the command args
        let exec_cmd = command.remove(0);
        let path = Path::new(&exec_cmd);

        if path.is_absolute() && !path.exists() {
            log::debug!("{:?} is not exist in parse!", path);
            return Err(format!("{:?} is not exist!", path).into());
        }

        let new_command = Rc::new(RefCell::new(CommandLine {
            cmd: path.to_str().unwrap().to_string(),
            args: command,
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
