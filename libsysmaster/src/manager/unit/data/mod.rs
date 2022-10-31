pub(super) use d_table::DataManager;
pub(super) use unit_dep_conf::UnitDepConf;
pub(super) use unit_state::UnitState;
pub use unit_state::{UnitActiveState, UnitNotifyFlags};

// dependency: {unit_state | unit_dep_conf} -> d_table
mod d_table;
mod unit_dep_conf;
mod unit_state;
