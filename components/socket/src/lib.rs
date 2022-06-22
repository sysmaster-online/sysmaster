// dependency:
// socket_base -> {socket_comm | socket_config}
// {socket_pid | socket_spawn} ->
// {socket_mng | socket_load} -> socket_unit

mod socket_base;

mod socket_comm;
mod socket_config;

mod socket_mng;

mod socket_load;

mod socket_spawn;

mod socket_port;

mod socket_unit;
