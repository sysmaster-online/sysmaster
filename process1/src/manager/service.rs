use super::unit::{ConfigParser, UnitManager,UnitObj,Unit};
use std::any::{TypeId, Any};
use std::collections::LinkedList;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fmt;
use std::io::{Error as IOError, ErrorKind};
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use crate::watchdog;

#[derive(PartialEq,Default)]
struct ExitStatusSet {

}

#[derive(PartialEq, EnumString, Display, Debug)]
enum ServiceTimeoutFailureMode {
    #[strum(serialize = "terminate")]
    ServiceTimeoutTerminate,
    #[strum(serialize = "abort")]
    ServiceTimeoutAbort,
    #[strum(serialize = "kill")]
    ServiceTimeoutKill,
    ServiceTimeoutFailureModeMax,
    ServiceTimeoutFailureModeInvalid = -1,
}

impl Default for ServiceTimeoutFailureMode{
    fn default() -> Self {
        ServiceTimeoutFailureMode::ServiceTimeoutTerminate
    }
}


#[derive(PartialEq, EnumString, Display, Debug)]
enum ServiceRestart {
    #[strum(serialize = "no")]
    ServiceRestartNo,
    #[strum(serialize = "on-success")]
    ServiceRestartOnSuccess,
    #[strum(serialize = "on-failure")]
    ServiceRestartOnFailure,
    #[strum(serialize = "on-abnormal")]
    ServiceRestartOnAbnormal,
    #[strum(serialize = "on-abort")]
    ServiceRestartOnAbort,
    #[strum(serialize = "always")]
    ServiceRestartAlways,
    ServiceRestartMax,
    ServiceRestartInvalid = -1
}


impl Default for ServiceRestart {
    fn default() -> Self { ServiceRestart::ServiceRestartNo}
}
#[derive(PartialEq, EnumString, Display, Debug)]
enum ServiceType {
    #[strum(serialize = "simple")]
    ServiceSimple,
    #[strum(serialize = "forking")]
    SserviceForking,
    #[strum(serialize = "oneshot")]
    ServiceOneshot,
    #[strum(serialize = "dbus")]
    ServiceDbus,
    #[strum(serialize = "notify")]
    ServiceNotify,
    #[strum(serialize = "idle")]
    SserviceIdle,
    #[strum(serialize = "exec")]
    ServiceExec,
    ServiceTypeMax,
    ServiceTypeInvalid = -1,
}


impl Default for ServiceType {
     fn default() -> Self { ServiceType::ServiceSimple}
}
enum ServiceCommand {
    ServiceCondition,
    ServiceStartPre,
    ServiceStart,
    ServiceStartPost,
    ServiceReload,
    ServiceStop,
    ServiceStopPost,
    ServiceCommandMax,
}

#[derive(PartialEq,Default)]
struct DualTimestamp {

}

#[derive(PartialEq)]
struct CommandLine {
    cmd: String,
    args: Vec<String>,
}

impl fmt::Display for CommandLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Display: {}", self.cmd)
    }
}

#[derive(PartialEq,Default)]
pub struct ServiceUnit {
    unit: Unit,
    service_type: ServiceType,
    restart: ServiceRestart,
    restart_prevent_status: ExitStatusSet,
    restart_force_status: ExitStatusSet,
    success_status: ExitStatusSet,
    pid_file: String,
    restart_usec:u64,
    timeout_start_usec:u64,
    timeout_stop_usec:u64,
    timeout_abort_usec:u64,
    timeout_abort_set:bool,
    runtime_max_usec:u64,
    timeout_start_failure_mode: ServiceTimeoutFailureMode,
    timeout_stop_failure_mode: ServiceTimeoutFailureMode,
    watchdog_timestamp: DualTimestamp,
    watchdog_usec: u64,
    watchdog_original_usec:u64,
    watchdog_override_usec:u64,
    watchdog_override_enable:bool,
    socket_fd: isize,
    bus_name:String,
    exec_commands: [LinkedList<CommandLine>; ServiceCommand::ServiceCommandMax as usize],
    // TODO

}   

impl ServiceUnit {
    pub fn new(unit: Unit) -> Self {
        Self {
            unit,
            service_type: ServiceType::ServiceTypeInvalid,
            restart: ServiceRestart::ServiceRestartInvalid,
            restart_prevent_status: ExitStatusSet{},
            restart_force_status:ExitStatusSet{},
            success_status:ExitStatusSet{},
            pid_file: String::from(""),
            restart_usec: 0,
            timeout_start_usec:0,
            timeout_stop_usec:0,
            timeout_abort_usec:0,
            timeout_abort_set:false,
            runtime_max_usec:u64::MAX,
            timeout_start_failure_mode:ServiceTimeoutFailureMode::ServiceTimeoutFailureModeInvalid,
            timeout_stop_failure_mode:ServiceTimeoutFailureMode::ServiceTimeoutFailureModeInvalid,
            watchdog_timestamp:DualTimestamp{},
            watchdog_usec:0,
            watchdog_original_usec:u64::MAX,
            watchdog_override_usec:0,
            watchdog_override_enable:false,
            socket_fd:-1,
            bus_name:String::from(""),
            exec_commands: Default::default(),
        }
    }

    pub fn unit_service_load(&mut self, manager: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        return self.unit.load(manager);
    }

    pub fn service_add_extras(&mut self) -> bool {
        if self.service_type == ServiceType::ServiceTypeInvalid {
            if !self.bus_name.is_empty() {
                self.service_type = ServiceType::ServiceDbus;
            }
        }
        true
    }

    pub fn service_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub fn get_unit_name(&self) -> String {
        self.unit.id.to_string()
    }
}

impl ServiceUnit {
    pub fn start_watchdog(self) {
        let watchdog_usec = if self.watchdog_override_enable {
            self.watchdog_override_usec
        } else { self.watchdog_original_usec };
        if watchdog_usec == 0 || watchdog_usec == u64::MAX {
            self.stop_watchdog()
        }
        watchdog::register_timer();
        watchdog::event_source_set_enabled(true);
    }

    pub fn stop_watchdog(self) {
        watchdog::event_source_set_enabled(false);
    }
}

impl UnitObj for ServiceUnit {
    fn init(&self) {
         todo!() 
    }
    fn done(&self) { todo!() }
    fn load(&mut self,m: &mut UnitManager) -> Result<(), Box<dyn Error>> { 
        self.unit_service_load(m)?;

        self.parse(m)?;

        self.service_add_extras();

        return self.service_verify()

     }
    fn coldplug(&self) { todo!() }
    fn start(&self) {
        let commands = &self.exec_commands[ServiceCommand::ServiceStart as usize];

        for command in commands.iter() { 
            println!("{}", command);
        }
        
    }
    fn dump(&self) { todo!() }
    fn stop(&self) { todo!() }
    fn reload(&self) { todo!() }
    fn kill(&self) { todo!() }
    fn check_gc(&self) -> bool { todo!() }
    fn release_resources(&self) { todo!() }
    fn check_snapshot(&self) { todo!() }
    fn sigchld_events(&self, _: u64, _: i32, _: i32) { todo!() }
    fn reset_failed(&self) { todo!() }

    fn eq(&self, other: &dyn UnitObj) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<ServiceUnit>() {
            return self == other;
        }
        false
    }

    fn hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        Hash::hash(&(TypeId::of::<ServiceUnit>()), &mut h);
        h.write(self.unit.id.as_bytes());
        h.finish()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

use crate::declure_unitobj_plugin;
declure_unitobj_plugin!(ServiceUnit,ServiceUnit::default);

impl ConfigParser for ServiceUnit {
    fn parse(&mut self, manager: &mut UnitManager)  -> Result<(), Box<dyn Error>> {
        self.unit.parse(manager)?;
        let conf = self.unit.conf.as_ref().ok_or_else(|| IOError::new(ErrorKind::Other, "config file not loaded"))?;
        
        let service = conf.service.as_ref().unwrap();
        match &service.exec_start {
            None => {self.exec_commands[ServiceCommand::ServiceStart as usize] = LinkedList::new();},
            Some(exec_start) => {
                let commands = &mut self.exec_commands[ServiceCommand::ServiceStart as usize];
                commands.push_back(CommandLine{cmd: exec_start.to_string(), args: Vec::new()});
            }
        }

        match &service.restart {
            None => {self.restart = ServiceRestart::ServiceRestartNo;},
            Some(restart) => {self.restart = ServiceRestart::from_str(restart)?;}
        }
        
        match &service.service_type {
            None => {self.service_type = ServiceType::ServiceTypeInvalid;},
            Some(service_type) => {self.service_type = ServiceType::from_str(service_type)?;}
        }

        Ok(())
    }
}
