use crate::service_base::ServiceConf;

use super::service::ServiceUnit;
use super::service_base::ServiceCommand;
use super::service_base::ServiceType;
use process1::manager::CommandLine;
use std::collections::LinkedList;
use std::error::Error;
use std::path::Path;
use std::{cell::RefCell, rc::Rc};

impl ServiceUnit {
    pub(super) fn parse(&mut self, service_conf: ServiceConf) -> Result<(), Box<dyn Error>> {
        let mut update_exec_command = |command_type: ServiceCommand, ps: usize| {
            let commands: Option<Vec<String>> = match command_type {
                ServiceCommand::ServiceCondition => service_conf.get_exec_condition(),
                ServiceCommand::ServiceStartPre => todo!(),
                ServiceCommand::ServiceStart => service_conf.get_exec_start(),
                ServiceCommand::ServiceStartPost => todo!(),
                ServiceCommand::ServiceReload => service_conf.get_exec_reload(),
                ServiceCommand::ServiceStop => service_conf.get_exec_stop(),
                ServiceCommand::ServiceStopPost => todo!(),
                ServiceCommand::ServiceCommandMax => todo!(),
            };
            if commands.is_some() {
                self.exec_commands[ps] = LinkedList::new();
                match prepare_command(commands.unwrap(), &mut self.exec_commands[ps]) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                return Err(format!("config opton is error, value cannot be null").into());
            }
        };

        if let Err(e) = update_exec_command(
            ServiceCommand::ServiceCondition,
            ServiceCommand::ServiceCondition as usize,
        ) {
            return Err(e);
        }

        if let Err(e) = update_exec_command(
            ServiceCommand::ServiceStart,
            ServiceCommand::ServiceStart as usize,
        ) {
            return Err(e);
        }
        if let Err(e) = update_exec_command(
            ServiceCommand::ServiceStop,
            ServiceCommand::ServiceStop as usize,
        ) {
            return Err(e);
        }

        if let Err(e) = update_exec_command(
            ServiceCommand::ServiceReload,
            ServiceCommand::ServiceReload as usize,
        ) {
            return Err(e);
        }

        let s_type: ServiceType = service_conf.get_service_type();
        self.service_type = s_type;
        Ok(())
    }
}

fn prepare_command(
    commands: Vec<String>,
    command_list: &mut LinkedList<Rc<RefCell<CommandLine>>>,
) -> Result<(), Box<dyn Error>> {
    if commands.len() == 0 {
        return Err(format!("config opton is error, value cannot be null").into());
    }

    let mut set_command = false;
    for cmd in commands.iter() {
        if cmd.is_empty() {
            continue;
        }

        set_command = true;
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

    if set_command {
        Ok(())
    } else {
        return Err(format!("config opton is error, value cannot be null").into());
    }
}
