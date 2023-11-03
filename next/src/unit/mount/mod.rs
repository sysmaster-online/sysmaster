use async_trait::async_trait;
use futures::future::pending;
use rustix::fs::{MountFlags, UnmountFlags};

use crate::{
    fstab::{FsEntry, MountInfo},
    unit::{Unit, UnitKind},
    util::{
        loader::{empty_dep, empty_str},
        mount::{mount, mount_point_to_unit_name, unmount},
    },
    Rc,
};

use super::{RtMsg, State, UnitCommon, UnitDeps, UnitHandle, UnitImpl};

pub(crate) type Impl = Rc<MountInfo>;
pub(super) struct Handle;

#[async_trait]
impl super::Handle for Handle {
    async fn stop(self: Box<Self>) -> Result<(), UnitHandle> {
        // noop: all info in unit
        Ok(())
    }
    async fn wait(&mut self) -> RtMsg {
        pending().await // never return
    }
}

impl From<FsEntry> for Impl {
    fn from(value: FsEntry) -> Self {
        value.mount_info.clone()
    }
}

impl From<Impl> for UnitImpl<Impl> {
    fn from(value: Impl) -> Self {
        let name = value.mount_point.to_str().unwrap();
        let name = mount_point_to_unit_name(name).into();
        let common = UnitCommon {
            name,
            description: empty_str(),
            documentation: empty_str(),
            deps: empty_dep(),
        };
        Self { common, sub: value }
    }
}

impl From<FsEntry> for UnitImpl<Impl> {
    fn from(value: FsEntry) -> Self {
        let mount_impl: Impl = value.into();
        mount_impl.into()
    }
}

#[async_trait]
impl Unit for UnitImpl<Impl> {
    fn name(&self) -> Rc<str> {
        Rc::clone(&self.common.name)
    }

    fn description(&self) -> Rc<str> {
        Rc::clone(&self.common.description)
    }

    fn documentation(&self) -> Rc<str> {
        Rc::clone(&self.common.documentation)
    }

    fn kind(&self) -> UnitKind {
        UnitKind::Mount
    }

    async fn start(&self) -> Result<UnitHandle, ()> {
        let Self {
            common: _,
            sub: mount_info,
        } = self;
        let mount_info = mount_info.clone();
        match tokio::task::block_in_place(|| mount(mount_info, MountFlags::empty())) {
            Ok(_) => Ok(Box::new(Handle)),
            Err(_) => Err(()),
        }
    }

    async fn stop(&self, _: UnitHandle) -> Result<(), ()> {
        let Self {
            common: _,
            sub: mount_info,
        } = self;
        let mount_info = mount_info.clone();
        match tokio::task::block_in_place(|| unmount(mount_info, UnmountFlags::empty())) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()> {
        self.stop(handle).await?;
        self.start().await
    }

    fn deps(&self) -> Rc<UnitDeps> {
        todo!()
    }
}
