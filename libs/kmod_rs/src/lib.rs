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

//! innner lib of kmod
use basic::cmdline;
use nix::errno;
use std::collections::HashSet;
use std::ffi::{CStr, CString, OsStr};
use std::os::raw::{c_char, c_uint};
use std::os::unix::ffi::OsStrExt;
use std::ptr;

type Result<T> = std::result::Result<T, nix::Error>;

/// kmod resource's status
#[allow(missing_docs)]
#[derive(PartialEq)]
pub enum KmodResources {
    KmodResourceOk = 0,
    KmodResourceMustReload = 1,
    KmodResourceMustRecreate = 2,
}

/// Iterator over a kmod_list of kmod
pub struct KmodListIter {
    /// Current kernel module
    pub cur: *mut kmod_sys::kmod_list,
    /// List head
    pub head: *mut kmod_sys::kmod_list,
}

impl Iterator for KmodListIter {
    type Item = KmodListIter;
    /// Get the current Iterator, internal Iterator points to the next
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_null() {
            None
        } else {
            let ret = Some(KmodListIter {
                head: self.head,
                cur: self.cur,
            });
            self.cur = unsafe { kmod_sys::kmod_list_next(self.head, self.cur) };
            ret
        }
    }
}

/// struct for kmod, contain context, kmod list, module
#[allow(missing_docs)]
pub struct LibKmod {
    ctx: *mut kmod_sys::kmod_ctx,
    kmod_list_head: *mut kmod_sys::kmod_list,
    module: *mut kmod_sys::kmod_module,
}

impl Drop for LibKmod {
    fn drop(&mut self) {
        unsafe {
            kmod_sys::kmod_unref(self.ctx);
            kmod_sys::kmod_module_unref(self.module);
            kmod_sys::kmod_module_unref_list(self.kmod_list_head);
        };
    }
}

impl LibKmod {
    /// Create libkmod
    pub fn new() -> Option<LibKmod> {
        let c = unsafe { kmod_sys::kmod_new(std::ptr::null(), std::ptr::null()) };

        if c.is_null() {
            return None;
        }

        Some(LibKmod {
            ctx: c,
            kmod_list_head: ptr::null::<kmod_sys::kmod_list>() as *mut kmod_sys::kmod_list,
            module: ptr::null::<kmod_sys::kmod_module>() as *mut kmod_sys::kmod_module,
        })
    }

    /// Create KmodListIter with internal members
    pub fn iter(&self) -> KmodListIter {
        KmodListIter {
            cur: self.kmod_list_head,
            head: self.kmod_list_head,
        }
    }

    /// Load resources
    pub fn load_resources(&mut self) -> Result<()> {
        if (unsafe { kmod_sys::kmod_load_resources(self.ctx) }) == 0 {
            Ok(())
        } else {
            Err(errno::from_i32(errno::errno()))
        }
    }

    /// Get kmod_list from lookup
    pub fn module_new_from_lookup<S: AsRef<OsStr>>(&mut self, lookup: S) -> Result<()> {
        if let Ok(lookup) = CString::new(lookup.as_ref().as_bytes()) {
            self.kmod_list_head = ptr::null::<kmod_sys::kmod_list>() as *mut kmod_sys::kmod_list;
            if (unsafe {
                kmod_sys::kmod_module_new_from_lookup(
                    self.ctx,
                    lookup.as_ptr(),
                    &mut self.kmod_list_head,
                )
            }) >= 0
            {
                return Ok(());
            }
        }
        Err(errno::from_i32(errno::errno()))
    }

    /// Check if kmod_list is null
    pub fn is_kmod_list_null(&self) -> bool {
        self.kmod_list_head.is_null()
    }

    /// Check if context is null
    pub fn is_ctx_null(&self) -> bool {
        self.ctx.is_null()
    }

    /// Get initstate
    pub fn get_initstate(&self) -> Result<kmod_sys::kmod_module_initstate> {
        if self.module.is_null() {
            return Err(errno::Errno::EINVAL);
        }

        Ok(unsafe { kmod_sys::kmod_module_get_initstate(self.module) }
            as kmod_sys::kmod_module_initstate)
    }

    /// Get module's name
    pub fn get_module_name(&self) -> Option<String> {
        if self.module.is_null() {
            return None;
        }

        let ret: *const c_char = unsafe { kmod_sys::kmod_module_get_name(self.module) };

        if let Ok(ret) = unsafe { CStr::from_ptr(ret) }.to_str() {
            Some(ret.to_owned())
        } else {
            None
        }
    }

    /// Probe insert module
    pub fn probe_insert_module_simple(&mut self, flag: u32) -> Result<u32> {
        if self.module.is_null() {
            return Err(errno::Errno::EINVAL);
        }
        match unsafe {
            kmod_sys::kmod_module_probe_insert_module(
                self.module,
                flag as c_uint,
                std::ptr::null(),
                None,
                std::ptr::null(),
                None,
            )
        } {
            e if e < 0 => Err(errno::from_i32(e)),
            ret => Ok(ret as u32),
        }
    }

    /// Set the value of the module
    pub fn set_module(&mut self, kmodlst: &KmodListIter) -> Result<()> {
        if kmodlst.cur.is_null() {
            return Err(errno::Errno::EINVAL);
        }

        self.module = unsafe { kmod_sys::kmod_module_get_module(kmodlst.cur) };
        if self.module.is_null() {
            return Err(errno::from_i32(errno::errno()));
        }
        Ok(())
    }

    /// Load module
    pub fn module_load_and_warn(&mut self, module: &str, verbose: bool) -> Result<()> {
        let level = if verbose {
            log::Level::Error
        } else {
            log::Level::Debug
        };

        log::debug!("Loading module: {}", module);

        let mut denylisy_parsed = false;
        let mut denylist: HashSet<String> = HashSet::new();

        if let Err(e) = self.module_new_from_lookup(module) {
            log::log!(level, "Failed to look up module alias {}", module);
            return Err(e);
        }

        if self.is_kmod_list_null() {
            log::log!(level, "Failed to find module {}", module);
            return Err(errno::Errno::EINVAL);
        }

        for iter in self.iter() {
            if let Err(e) = self.set_module(&iter) {
                log::log!(level, "Set module failed : {}", e);
                break;
            }

            let name = match self.get_module_name() {
                None => break,
                Some(name) => name,
            };

            match self.get_initstate() {
                Err(_) => log::error!("Module {} get initstate failed!", name),
                Ok(kmod_sys::kmod_module_initstate_KMOD_MODULE_BUILTIN) => {
                    log::debug!("Module {} is built in", name)
                }
                Ok(kmod_sys::kmod_module_initstate_KMOD_MODULE_LIVE) => {
                    log::debug!("Module {} is already loaded", name)
                }
                Ok(_) => {
                    match self
                        .probe_insert_module_simple(kmod_sys::kmod_probe_KMOD_PROBE_APPLY_BLACKLIST)
                    {
                        Ok(0) => log::info!("Inserted module {}", name),
                        Ok(kmod_sys::kmod_probe_KMOD_PROBE_APPLY_BLACKLIST) => {
                            log::debug!("Module {} is deny-listed", name)
                        }
                        Err(e) => {
                            if e == nix::Error::EPERM {
                                if !denylisy_parsed {
                                    cmdline::Cmdline::parse(
                                        cmdline::Cmdline::cmdline_item,
                                        &mut denylist,
                                    );
                                    denylisy_parsed = true;
                                }

                                if denylist.contains(&name) {
                                    log::info!("Module {} is deny-listed (by kernel)", name);
                                    continue;
                                }
                            }

                            let fail_level = if !verbose {
                                log::Level::Debug
                            } else if [nix::Error::ENODEV, nix::Error::ENOENT].contains(&e) {
                                log::Level::Warn
                            } else {
                                log::Level::Error
                            };

                            log::log!(fail_level, "Failed to insert module {}", name);

                            if ![nix::Error::ENODEV, nix::Error::ENOENT].contains(&e) {
                                return Err(e);
                            }
                        }
                        _ => {
                            log::error!("Reached unknown state.");
                        }
                    }
                }
            };
        }
        Ok(())
    }

    /// Get data of resource
    pub fn validate_resources(&mut self) -> Result<KmodResources> {
        if self.is_ctx_null() {
            return Err(errno::Errno::EINVAL);
        }
        match unsafe { kmod_sys::kmod_validate_resources(self.ctx) } {
            0 => Ok(KmodResources::KmodResourceOk),
            1 => Ok(KmodResources::KmodResourceMustReload),
            2 => Ok(KmodResources::KmodResourceMustRecreate),
            _ => Err(errno::Errno::UnknownErrno),
        }
    }

    /// Unref context
    pub fn unref_context(&mut self) {
        if self.is_ctx_null() {
            return;
        }
        unsafe { kmod_sys::kmod_unref(self.ctx) };
    }
}
