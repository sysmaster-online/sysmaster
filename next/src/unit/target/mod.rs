use async_trait::async_trait;
use futures::future::pending;

use super::{RtMsg, Unit, UnitDeps, UnitHandle, UnitImpl, UnitKind};
use crate::Rc;

pub(crate) mod loader;

#[derive(Debug)]
pub(crate) struct Impl;
pub(super) struct Handle;
#[async_trait]
impl super::Handle for Handle {
    async fn stop(self: Box<Self>) -> Result<(), UnitHandle> {
        Ok(())
    }
    async fn wait(&mut self) -> RtMsg {
        pending().await
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
        UnitKind::Service
    }

    fn deps(&self) -> Rc<UnitDeps> {
        self.common.deps.clone()
    }

    async fn start(&self) -> Result<UnitHandle, ()> {
        Ok(Box::new(Handle))
    }

    async fn stop(&self, handle: UnitHandle) -> Result<(), ()> {
        Ok(())
    }

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()> {
        Ok(Box::new(Handle))
    }
}
