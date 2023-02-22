pub(super) use dep_conf::UnitDepConf;
pub(super) use state::UnitState;
pub(crate) use table::DataManager;

// dependency: {unit_state | unit_dep_conf} -> d_table
mod dep_conf;
mod state;
mod table;
