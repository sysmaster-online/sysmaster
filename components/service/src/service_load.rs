use super::service_base::{ServiceCommand, ServiceType};
use super::service_config::{ServiceConf, ServiceConfig, ServiceConfigItem};
use process1::manager::ExecCommand;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;

pub(super) struct ServiceLoad {
    config: Rc<ServiceConfig>,
}

impl ServiceLoad {
    pub(super) fn new(configr: &Rc<ServiceConfig>) -> ServiceLoad {
        ServiceLoad {
            config: Rc::clone(configr),
        }
    }

    pub(super) fn service_add_extras(&self) -> bool {
        if let ServiceConfigItem::ScItemType(ServiceType::TypeInvalid) = self
            .config
            .get(&ServiceConfigItem::ScItemType(ServiceType::Simple))
        {
            if let ServiceConfigItem::ScItemBusName(bus_name) =
                self.config.get(&ServiceConfigItem::ScItemBusName(None))
            {
                if bus_name.is_some() && !bus_name.unwrap().is_empty() {
                    self.config
                        .set(ServiceConfigItem::ScItemType(ServiceType::Dbus));
                }
            }
        }
        true
    }

    pub(super) fn service_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub(super) fn parse(&self, service_conf: ServiceConf) -> Result<(), Box<dyn Error>> {
        let update_exec_command = |command_type: ServiceCommand| {
            let commands: Option<Vec<String>> = match command_type {
                ServiceCommand::Condition => service_conf.get_exec_condition(),
                ServiceCommand::StartPre => todo!(),
                ServiceCommand::Start => service_conf.get_exec_start(),
                ServiceCommand::StartPost => todo!(),
                ServiceCommand::Reload => service_conf.get_exec_reload(),
                ServiceCommand::Stop => service_conf.get_exec_stop(),
                ServiceCommand::StopPost => todo!(),
                ServiceCommand::CommandMax => todo!(),
            };
            if commands.is_some() {
                match self.prepare_command(command_type, &commands.unwrap()) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                return Err(format!("config opton is error, value cannot be null").into());
            }
        };

        update_exec_command(ServiceCommand::Condition)?;
        update_exec_command(ServiceCommand::Start)?;
        update_exec_command(ServiceCommand::Stop)?;
        update_exec_command(ServiceCommand::Reload)?;

        let s_type: ServiceType = service_conf.get_service_type();
        self.config.set(ServiceConfigItem::ScItemType(s_type));

        Ok(())
    }

    fn prepare_command(
        &self,
        cmd_type: ServiceCommand,
        commands: &Vec<String>,
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

            let cmd = path.to_str().unwrap().to_string();
            let new_command = Rc::new(ExecCommand::new(cmd, command));

            self.config.insert_exec_cmds(cmd_type, new_command);
        }

        if set_command {
            Ok(())
        } else {
            return Err(format!("config opton is error, value cannot be null").into());
        }
    }
}
