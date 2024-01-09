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

//! mount unit is entry of mount type of unit，need impl
//! UnitObj,UnitMngUtil, UnitSubClass trait

use super::comm::PathUnitComm;
use super::mng::PathMng;
use crate::bus::PathBus;
use crate::config::PathConfig;
use crate::mng::PathInotify;
use crate::rentry::PathState;
use basic::fs::{path_length_is_valid, path_name_is_safe, path_simplify};
use basic::{IN_SET, PATHS_TARGET, SHUTDOWN_TARGET, SYSINIT_TARGET};
use core::error::*;
use core::rel::{ReStation, Reliability};
use core::unit::unit_name_to_type;
use core::unit::PathSpec;
use core::unit::{
    PathType, SubUnit, UmIf, UnitActiveState, UnitBase, UnitDependencyMask, UnitMngUtil,
    UnitRelations, UnitType,
};
use nix::sys::wait::WaitStatus;
use nix::NixPath;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
struct PathUnit {
    comm: Rc<PathUnitComm>,
    mng: Rc<PathMng>,
    config: Rc<PathConfig>,
    bus: PathBus,
}

impl ReStation for PathUnit {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        self.config.db_map(reload);
        if !reload {
            self.build_mspecs().unwrap();
        }
        self.mng.db_map(reload);
    }

    fn db_insert(&self) {
        self.config.db_insert();
        self.mng.db_insert();
    }

    // reload: no external connections, entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        self.mng.entry_coldplug();
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        self.mng.entry_clear();
    }
}

impl PathUnit {
    fn new(_um: Rc<dyn UmIf>) -> PathUnit {
        let comm = Rc::new(PathUnitComm::new());
        let config = Rc::new(PathConfig::new(&comm));
        PathUnit {
            comm: Rc::clone(&comm),
            mng: Rc::new(PathMng::new(&comm, &config)),
            config: Rc::clone(&config),
            bus: PathBus::new(&comm, &config),
        }
    }

    fn build_mspecs(&self) -> Result<()> {
        self.parse_specs(
            &self.config.config_data().borrow().Path.PathExists,
            PathType::Exists,
        )?;

        self.parse_specs(
            &self.config.config_data().borrow().Path.PathExistsGlob,
            PathType::ExistsGlob,
        )?;

        self.parse_specs(
            &self.config.config_data().borrow().Path.DirectoryNotEmpty,
            PathType::DirectoryNotEmpty,
        )?;

        self.parse_specs(
            &self.config.config_data().borrow().Path.PathChanged,
            PathType::Changed,
        )?;

        self.parse_specs(
            &self.config.config_data().borrow().Path.PathModified,
            PathType::Modified,
        )
    }

    fn parse_specs(&self, paths: &[PathBuf], path_type: PathType) -> Result<()> {
        for path in paths {
            if path.is_empty() {
                continue;
            }

            let s = path.to_str().unwrap();

            if !path_name_is_safe(s) {
                log::error!("{:?} contains invalid character: {}", path_type, s);
                return Err(Error::Nix {
                    source: nix::Error::EINVAL,
                });
            }

            if !path_length_is_valid(s) {
                log::error!("{:?} is too long: {}", path_type, s);
                return Err(Error::Nix {
                    source: nix::Error::EINVAL,
                });
            }

            let s = match path_simplify(s) {
                None => {
                    log::error!("{:?} is not valid: {}", path_type, s);
                    return Err(Error::Nix {
                        source: nix::Error::EINVAL,
                    });
                }
                Some(v) => v,
            };

            if !path.is_absolute() {
                log::error!("{:?} path is not absolute, ignoring: {}", path_type, s);
                return Err(Error::Nix {
                    source: nix::Error::EINVAL,
                });
            }

            let spec = Rc::new(PathSpec::new(path.to_path_buf(), path_type));
            let inotify = Rc::new(PathInotify::new(&self.mng, spec));
            self.mng.push_inotify(inotify);
        }

        Ok(())
    }

    fn verify(&self) -> Result<()> {
        if self.mng.all_inotify().is_empty() {
            log::error!("Path unit lacks path setting. Refusing.");
            return Err(Error::Nix {
                source: nix::Error::ENOEXEC,
            });
        }

        Ok(())
    }

    fn add_extras(&self) -> Result<()> {
        let um = self.comm.um();
        let u = match self.comm.owner() {
            None => {
                return Ok(());
            }
            Some(v) => v,
        };

        self.add_trigger_dependencies(&um, &u)?;

        // TODO: add_mount_dependencies()

        self.add_default_dependencies(&um, &u)
    }

    fn add_trigger_dependencies(&self, um: &Rc<dyn UmIf>, u: &Rc<dyn UnitBase>) -> Result<()> {
        if !um.unit_get_trigger(&u.id()).is_empty() {
            return Ok(());
        }

        let name = self.load_related_unit(UnitType::UnitService, u)?;
        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitBefore,
            UnitRelations::UnitTriggers,
            &name,
            true,
            UnitDependencyMask::Implicit,
        )
    }

    fn load_related_unit(&self, related_type: UnitType, u: &Rc<dyn UnitBase>) -> Result<String> {
        let path_unit = self.config.config_data().borrow().Path.Unit.clone();
        if path_unit.is_empty() {
            let unit_name = u.id();
            let suffix = String::from(related_type);
            if suffix.is_empty() {
                return Err(format!("failed to load related unit {}", suffix).into());
            }
            if unit_name.is_empty() {
                return Err(
                    format!("failed to load related unit {} unit name is empty", suffix).into(),
                );
            }
            let u_name = unit_name;
            let stem_name = Path::new(&u_name).file_stem().unwrap().to_str().unwrap();
            let relate_name = format!("{}.{}", stem_name, suffix);
            return Ok(relate_name);
        }

        let path_unit_type = unit_name_to_type(&path_unit);
        if path_unit_type == UnitType::UnitTypeInvalid {
            return Err(format!("Unit {} type not valid, ignoring", path_unit).into());
        }
        if path_unit == u.id() {
            return Err(format!("Units {} cannot trigger themselves, ignoring", path_unit).into());
        }

        Ok(path_unit)
    }

    fn add_default_dependencies(&self, um: &Rc<dyn UmIf>, u: &Rc<dyn UnitBase>) -> Result<()> {
        if !u.default_dependencies() {
            return Ok(());
        }

        um.unit_add_dependency(
            &u.id(),
            UnitRelations::UnitBefore,
            PATHS_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;

        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitAfter,
            UnitRelations::UnitRequires,
            SYSINIT_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;

        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitBefore,
            UnitRelations::UnitConflicts,
            SHUTDOWN_TARGET,
            true,
            UnitDependencyMask::Default,
        )
    }
}

impl SubUnit for PathUnit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn load(&self, paths: Vec<PathBuf>) -> Result<()> {
        let unit_name = self.comm.get_owner_id();
        self.config.load(paths, &unit_name, true)?;

        self.add_extras()?;

        self.build_mspecs()?;

        self.verify()
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.current_active_state()
    }

    fn get_subunit_state(&self) -> String {
        self.mng.get_state()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> Result<()> {
        log::info!("Path start {:?}", self.comm.get_owner_id());

        self.mng.start_action();
        Ok(())
    }

    fn stop(&self, _force: bool) -> Result<()> {
        self.mng.stop_action();
        Ok(())
    }

    fn trigger(&self, other: &str) {
        /* Invoked whenever the unit we trigger changes state or gains or loses a job */

        /* Don't propagate state changes from the triggered unit if we are already down */
        if !IN_SET!(self.mng.state(), PathState::Waiting, PathState::Running) {
            return;
        }

        /* TODO: Propagate start limit hit state */

        /* Don't propagate anything if there's still a job queued */
        let um = self.comm.um();
        if um.has_job(other) {
            return;
        }

        if self.mng.state() == PathState::Running
            && um.current_active_state(other).is_inactive_or_failed()
        {
            log::debug!(
                "{}: Got notified about unit deactivation.",
                self.comm.get_owner_id()
            );
            self.mng.enter_waiting(false, true);
        } else if self.mng.state() == PathState::Waiting
            && um.current_active_state(other).is_inactive_or_failed()
        {
            log::debug!(
                "{}: Got notified about unit activation.",
                self.comm.get_owner_id()
            );
            self.mng.enter_waiting(false, true);
        }
    }

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _wait_status: WaitStatus) {}

    fn reset_failed(&self) {
        self.mng.reset_failed()
    }

    fn unit_set_property(
        &self,
        key: &str,
        value: &str,
        flags: core::unit::UnitWriteFlags,
    ) -> Result<()> {
        self.bus.unit_set_property(key, value, flags)
    }
}

impl UnitMngUtil for PathUnit {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

/*
impl Default for PathUnit {
    fn default() -> Self {
        PathUnit::new()
    }
}
*/

use core::declare_unitobj_plugin_with_param;
declare_unitobj_plugin_with_param!(PathUnit, PathUnit::new);
