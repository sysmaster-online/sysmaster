//! # 一种基于epoll的事件调度框架 An event scheduling framework based on epoll
//!
//! 支持io/signal/pidfd/child/timer/defer/post/exit等事件.
//!
//! 当多个或多种事件注册后, events框架会按照注册时设置的触发条件来循环调度.
//!
//! 事件源需要implement Source trait才能被events框架管理.
//!
//! # Example:
//! ```rust
//! # use std::{
//! #     net::{TcpListener, TcpStream},
//! #     os::unix::io::{AsRawFd, RawFd},
//! #     rc::Rc};
//! # use libutils::Error;
//! #
//! # use std::thread;
//! # use std::time::Duration;
//! #
//! # use libevent::Events;
//! # use libevent::Source;
//! # use libevent::{EventState, EventType};
//! #
//! /// 定义一个类型, implement Source trait
//! #[derive(Debug)]
//! struct Io {
//!     t: TcpStream,
//! }
//!
//! impl Io {
//!     fn new(s: &'static str) -> Io {
//!         Io {
//!             /// 建立通信
//!             t: TcpStream::connect(s).unwrap(),
//!         }
//!     }
//! }
//!
//! impl Source for Io {
//!     /// 设置要监听的句柄
//!     fn fd(&self) -> RawFd {
//!         self.t.as_raw_fd()
//!     }
//!
//!     /// 定义当前事件类型
//!     fn event_type(&self) -> EventType {
//!         EventType::Io
//!     }
//!
//!     /// 设置要注册的事件类型
//!     fn epoll_event(&self) -> u32 {
//!         (libc::EPOLLIN) as u32
//!     }
//!
//!     /// 设置优先级, -127i8 ~ 128i8, 值越小, 优先级越高
//!     fn priority(&self) -> i8 {
//!         0i8
//!     }
//!
//!     /// 此为事件到达后, 开始调度的代码
//!     fn dispatch(&self, _: &Events) -> Result<i32, Error> {
//!         println!("Dispatching IO!");
//!         Ok(0)
//!     }
//!
//!     /// 除非你能保证所有类型的token分配, 否则建议使用这里的默认实现
//!     fn token(&self) -> u64 {
//!         let data: u64 = unsafe { std::mem::transmute(self) };
//!         data
//!     }
//! }
//!
//! fn main() {
//!     /// 模拟一次网络通信事件的监听
//!     thread::spawn(move || {
//!         let listener = TcpListener::bind("0.0.0.0:9098").unwrap();
//!         loop {
//!             let (_stream, addr) = listener.accept().unwrap();
//!             println!("Accepted a new connection: {}", addr);
//!         }
//!     });
//!
//!     thread::sleep(Duration::from_millis(100));
//!
//!     /// 创建event调度框架
//!     let mut e = Events::new().unwrap();
//!
//!     /// 创建事件类型
//!     let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9098"));
//!
//!     /// 添加到调度框架中
//!     e.add_source(s.clone()).unwrap();
//!
//!     /// 开启调度
//!     e.set_enabled(s.clone(), EventState::OneShot).unwrap();
//!
//!     /// 一次调度, 也可使用rloop()一直循环调度
//!     e.run(100).unwrap();
//!
//!     /// 删除事件后, 不再调度该事件
//!     e.del_source(s.clone()).unwrap();
//! }
//! ```
//!
#![deny(missing_docs)]
pub mod events;
pub mod poll;
mod signal;
pub mod source;
mod timer;

pub use crate::events::Events;
pub(crate) use crate::poll::Poll;
pub(crate) use crate::signal::Signals;
pub use crate::source::Source;

/// 支持添加到框架中的事件类型
/// An event scheduling framework based on epoll
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum EventType {
    /// Io类型
    Io,
    /// realtime定时器
    TimerRealtime,
    /// boottime定时器
    TimerBoottime,
    /// monotonic定时器
    TimerMonotonic,
    /// realtime alarm定时器
    TimerRealtimeAlarm,
    /// boottime alarm定时器
    TimerBoottimeAlarm,
    /// 信号
    Signal,
    /// 子进程
    Child,
    /// 进程
    Pidfd,
    /// Watchdog
    Watchdog,
    /// Inotify监控
    Inotify,
    /// Defer事件, 每一次LOOP执行一次
    Defer,
    /// Post事件
    Post,
    /// 退出事件
    Exit,
}

/// 事件的调度状态
/// The dispatch status of the event
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum EventState {
    /// 开启调度
    On,
    /// 关闭调度
    Off,
    /// 调度一次后停止
    OneShot,
}
