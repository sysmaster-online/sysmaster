#[macro_use]
extern crate strum;

// dependency:
// service_base -> {service_comm | service_config}
// {service_pid | service_spawn} ->
// {service_mng | service_load} ->
// {service_monitor} -> service_unit
mod service_base;
mod service_comm;
mod service_config;
mod service_mng;
mod service_monitor;
mod service_pid;
mod service_spawn;
mod service_unit;
