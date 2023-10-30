use std::fmt::{Debug, Display};

use async_trait::async_trait;
use tokio::{
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::oneshot,
};

use crate::Rc;

pub(crate) mod mount;
pub(crate) mod service;
pub(crate) mod socket;
pub(crate) mod target;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Uninit = 0,
    Stopped,
    Failed,
    Starting,
    Active,
    Stopping,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            State::Uninit => "Uninit",
            State::Stopped => "Stopped",
            State::Failed => "Failed",
            State::Starting => "Starting",
            State::Active => "Active",
            State::Stopping => "Stopping",
        };
        write!(f, "{}", s)
    }
}

impl State {
    pub(crate) fn is_active(&self) -> bool {
        matches!(self, State::Active)
    }
    pub(crate) fn is_dead(&self) -> bool {
        matches!(self, State::Uninit | State::Stopped | State::Failed)
    }

    pub(crate) fn is_stopped(&self) -> bool {
        matches!(self, State::Stopped)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum UnitKind {
    Service,
    Timer,
    Mount,
    Target,
    Socket,
}

impl Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UnitKind::Service => "service",
            UnitKind::Timer => "timer",
            UnitKind::Mount => "mount",
            UnitKind::Target => "target",
            UnitKind::Socket => "socket",
        };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub(crate) struct UnitCommon {
    name: Rc<str>,
    description: Rc<str>,
    documentation: Rc<str>,
    deps: Rc<UnitDeps>, // todo
}

#[derive(Debug, Default)]
pub(crate) struct UnitDeps {
    pub requires: Box<[UnitId]>,
    pub wants: Box<[UnitId]>,
    pub after: Box<[UnitId]>,
    pub before: Box<[UnitId]>,
    pub conflicts: Box<[UnitId]>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct UnitId {
    name: Rc<str>,
}
impl Display for UnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<&str> for UnitId {
    fn from(value: &str) -> Self {
        Self { name: value.into() }
    }
}

impl<T: Unit + ?Sized> From<&T> for UnitId {
    fn from(value: &T) -> Self {
        Self { name: value.name() }
    }
}

impl UnitId {
    pub(crate) fn kind(&self) -> UnitKind {
        match self.name.rsplit_once('.').unwrap().1 {
            "service" => UnitKind::Service,
            "timer" => UnitKind::Timer,
            "mount" => UnitKind::Mount,
            "target" => UnitKind::Target,
            "socket" => UnitKind::Socket,
            _ => unreachable!(),
        }
    }
}

type ChildStdio = (ChildStdin, ChildStdout, ChildStderr);
pub(crate) struct Extra {
    pub basic_io: Option<oneshot::Sender<ChildStdio>>,
}

pub(crate) enum RtMsg {
    Yield,
    Exit(State),
    TriggerStart(UnitId, Extra),
}

#[async_trait]
pub(crate) trait Handle: Send {
    /// use runtime info to stop the running things
    async fn stop(self: Box<Self>) -> Result<(), UnitHandle>;

    /// monitor runtime state, and return messages including rt notice or exit state...
    async fn wait(&mut self) -> RtMsg;
}
type UnitHandle = Box<dyn Handle>;

#[async_trait]
pub(crate) trait Unit: Debug {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> Rc<UnitDeps>;

    /// start the unit, return a handle which
    /// contains runtime info needed for monitor and stop/kill
    async fn start(&self) -> Result<UnitHandle, ()>; // todo: error type

    /// do things needed to stop the unit
    async fn stop(&self, handle: UnitHandle) -> Result<(), ()>;

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()>;
}

pub(crate) type UnitObj = Rc<dyn Unit + Send + Sync + 'static>;

#[derive(Debug)]
pub(crate) struct UnitImpl<KindImpl> {
    pub common: UnitCommon,
    pub sub: KindImpl,
}
