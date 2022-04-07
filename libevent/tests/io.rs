use std::{
    cell::RefCell,
    net::{TcpListener, TcpStream},
    os::unix::io::{AsRawFd, RawFd},
    rc::Rc,
};

use event::EventType;
use event::Events;
use event::Source;
use utils::{Error, Result};

use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct Io {
    t: TcpStream,
}

impl Io {
    fn new(s: &'static str) -> Io {
        Io {
            t: TcpStream::connect(s).unwrap(),
        }
    }
}

impl Source for Io {
    fn fd(&self) -> RawFd {
        self.t.as_raw_fd()
    }

    fn event_type(&self) -> crate::EventType {
        crate::EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN | libc::EPOLLONESHOT) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _: &mut Events) -> Result<i32, Error> {
        println!("Dispatching IO!");
        Ok(0)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

#[test]
fn test_io() {
    thread::spawn(move || {
        let listener = TcpListener::bind("0.0.0.0:9097").unwrap();
        loop {
            let (_stream, addr) = listener.accept().unwrap();
            println!("Accepted a new connection: {}", addr);
        }
    });

    thread::sleep(Duration::from_millis(100));
    let mut e = Events::new().unwrap();
    let s: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(Io::new("0.0.0.0:9097")));
    let s2: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(Io::new("127.0.0.1:9097")));
    e.add_source(s.clone());
    e.add_source(s2.clone());
    e.run(100);
    e.run(100);
    e.run(100);
    // e.rloop();
}
