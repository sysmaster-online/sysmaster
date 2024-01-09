// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//!
pub use base::{unit_name_is_valid, SubUnit, UnitBase, UnitNameFlags};
pub use deps::{
    unit_name_to_type, unit_write_flags_is_noop, UnitDependencyMask, UnitRelationAtom,
    UnitRelations, UnitType, UnitWriteFlags,
};
pub use kill::{KillContext, KillMode, KillOperation};
pub use path_spec::{PathSpec, PathType};
pub use state::{UnitActiveState, UnitNotifyFlags, UnitStatus};
pub use umif::{UmIf, UnitManagerObj, UnitMngUtil};
pub use write::{unit_write_setting, unit_write_settingf};

mod base;
mod deps;
mod kill;
mod path_spec;
mod state;
mod umif;
mod write;
