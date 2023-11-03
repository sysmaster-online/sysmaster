use std::process::Stdio;

use async_trait::async_trait;
use futures::future::pending;
use tokio::{io, process::Child};

use super::{RtMsg, State, Unit, UnitDeps, UnitHandle, UnitImpl, UnitKind};
use crate::Rc;

pub(crate) mod loader;

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) enum Kind {
    Simple,
    Forking,
    Oneshot,
    Notify,
}

pub(crate) enum Handle {
    Process(tokio::process::Child),
    Empty,
}

#[async_trait]
impl super::Handle for Handle {
    async fn stop(mut self: Box<Self>) -> Result<(), UnitHandle> {
        match self.as_mut() {
            Handle::Process(child) => child.kill().await.or(Err(self)),
            Handle::Empty => Ok(()),
        }
    }
    async fn wait(&mut self) -> RtMsg {
        match self {
            Handle::Process(child) => match child.wait().await {
                Ok(exitcode) => {
                    if exitcode.success() {
                        RtMsg::Exit(State::Stopped)
                    } else {
                        RtMsg::Exit(State::Failed)
                    }
                }
                Err(_) => todo!(),
            },
            Handle::Empty => pending().await,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Impl {
    kind: Kind,
    exec_start: Rc<str>,
    exec_stop: Rc<str>,
    exec_restart: Rc<str>,
}

impl Impl {
    pub fn new(kind: Kind, start: Rc<str>, stop: Rc<str>, restart: Rc<str>) -> Self {
        Self {
            kind,
            exec_start: start,
            exec_stop: stop,
            exec_restart: restart,
        }
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
        let kind = self.sub.kind;
        match kind {
            Kind::Simple => {
                let exec_start = self.sub.exec_start.clone();
                match run_cmd(&exec_start) {
                    Ok(child) => Ok(Box::new(Handle::Process(child))),
                    Err(e) => {
                        println!("{}: {}", self.name(), e);
                        Err(())
                    }
                }
            }
            Kind::Forking => todo!(),
            Kind::Oneshot => {
                if self.sub.exec_start.is_empty() {
                    todo!()
                } else {
                    match run_cmd(&self.sub.exec_start).unwrap().wait().await {
                        Ok(exitcode) => {
                            if exitcode.success() {
                                Ok(Box::new(Handle::Empty))
                            } else {
                                Err(())
                            }
                        }
                        Err(_) => todo!(),
                    }
                }
            }
            Kind::Notify => todo!(),
        }
    }

    async fn stop(&self, handle: UnitHandle) -> Result<(), ()> {
        match self.sub.kind {
            Kind::Simple => handle.stop().await.or(Err(())),
            Kind::Forking => todo!(),
            Kind::Oneshot => {
                if self.sub.exec_stop.is_empty() {
                    Ok(())
                } else {
                    match run_cmd(&self.sub.exec_stop).unwrap().wait().await {
                        Ok(exitcode) => {
                            if exitcode.success() {
                                Ok(())
                            } else {
                                Err(())
                            }
                        }
                        Err(_) => todo!(),
                    }
                }
            }
            Kind::Notify => todo!(),
        }
    }

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()> {
        self.stop(handle).await?;
        self.start().await
    }
}

fn run_cmd(cmd: &str) -> Result<Child, io::Error> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty command!",
        ));
    }
    let mut s = cmd.split_whitespace();
    tokio::process::Command::new(s.next().unwrap())
        .args(s)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}
