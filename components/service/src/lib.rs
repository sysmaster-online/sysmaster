#[macro_use]
extern crate strum;

// dependency:
// service_base -> {service_comm | service_config}
// {service_pid | service_spawn} ->
// {service_mng | service_load} ->
// {service_monitor} -> service_unit
#[allow(dead_code)]
mod service_base;
mod service_comm;
#[allow(dead_code)]
mod service_config;
mod service_load;
#[allow(dead_code)]
mod service_mng;
#[allow(dead_code)]
mod service_monitor;
mod service_pid;
#[allow(dead_code)]
mod service_spawn;
#[allow(dead_code)]
mod service_unit;
