pub mod events;
pub mod poll;
mod signal;
pub mod source;

pub use crate::events::Events;
pub(crate) use crate::poll::Poll;
pub(crate) use crate::signal::Signals;
pub use crate::source::Source;

#[derive(PartialEq, Debug)]
pub enum EventType {
    Io,
    Timer,
    TimerRelative,
    Signal,
    Child,
    Pidfd,
    Inotify,
    Defer,
    Post,
    Exit,
}

#[derive(PartialEq, Debug)]
pub enum EventState {
    On,
    Off,
    OneShot,
}
