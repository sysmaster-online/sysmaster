use super::unit;
use std::any::{TypeId, Any};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;

#[derive(PartialEq)]
struct ExitStatusSet {

}

#[derive(PartialEq)]
enum ServiceTimeoutFailureMode {
    ServiceTimeoutTerminate,
    ServiceTimeoutAbort,
    ServiceTimeoutKill,
    ServiceTimeoutFailureModeMax,
    ServiceTimeoutFailureModeInvalid = -1,
}

#[derive(PartialEq)]
enum ServiceRestart {
    ServiceRestartNo,
    ServiceRestartOnSuccess,
    ServiceRestartOnFailure,
    ServiceRestartOnAbnormal,
    ServiceRestartOnAbort,
    ServiceRestartAlways,
    ServiceRestartMax,
    ServiceRestartInvalid = -1
}

#[derive(PartialEq, Debug)]
enum ServiceType {
    ServiceSimple,
    SserviceForking,
    ServiceOneshot,
    ServiceDbus,
    ServiceNotify,
    SserviceIdle,
    ServiceExec,
    ServiceTypeMax,
    ServiceTypeInvalid = -1,
}

/*
impl Default for ServiceType {
    fn default() => Self {ServiceType::ServiceTypeInvalid}
}
*/

#[derive(PartialEq)]
struct DualTimestamp {

}

#[derive(PartialEq)]
pub struct ServiceUnit {
    service_unit: unit::Unit,
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
    // TODO

}   

impl ServiceUnit {
    pub fn new(unit: unit::Unit) -> Self {
        Self {
            service_unit: unit,
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
        }
    }

    pub fn unit_service_load_and_parse(&mut self) -> bool {
        println!("load frament and dropin");
        return self.service_unit.unit_load_and_parse(true);
    }

    pub fn service_add_extras(&mut self) -> Result<isize, io::Error> {
        if self.service_type == ServiceType::ServiceTypeInvalid {
            if !self.bus_name.is_empty() {
                self.service_type = ServiceType::ServiceDbus;
            }
        }
        Ok(0)
    }

    pub fn service_verify(&self) -> bool {
        true
    }
}

impl unit::UnitObj for ServiceUnit {
    fn init(&self) {
         todo!() 
    }
    fn done(&self) { todo!() }
    fn load(&mut self) -> bool { 
        if !self.unit_service_load_and_parse() {
            return false;
        }

        self.service_add_extras();

        return self.service_verify()

     }
    fn coldplug(&self) { todo!() }
    fn start(&self) { todo!() }
    fn dump(&self) { todo!() }
    fn stop(&self) { todo!() }
    fn reload(&self) { todo!() }
    fn kill(&self) { todo!() }
    fn check_gc(&self) -> bool { todo!() }
    fn release_resources(&self) { todo!() }
    fn check_snapshot(&self) { todo!() }
    fn sigchld_events(&self, _: u64, _: i32, _: i32) { todo!() }
    fn reset_failed(&self) { todo!() }

    fn eq(&self, other: &dyn unit::UnitObj) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<ServiceUnit>() {
            return self == other;
        }
        false
    }

    fn hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        Hash::hash(&(TypeId::of::<ServiceUnit>()), &mut h);
        h.write(self.service_unit.id.as_bytes());
        h.finish()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
