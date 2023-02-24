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

#[cfg(test)]
mod test {
    use libevent::Events;
    use libevent::Source;
    use libevent::{EventState, EventType};
    use std::thread;
    use std::time::Duration;
    use std::{
        net::{TcpListener, TcpStream},
        os::unix::io::{AsRawFd, RawFd},
        rc::Rc,
    };

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

        fn event_type(&self) -> EventType {
            EventType::Io
        }

        fn epoll_event(&self) -> u32 {
            (libc::EPOLLIN) as u32
        }

        fn priority(&self) -> i8 {
            0i8
        }

        fn dispatch(&self, _: &Events) -> i32 {
            println!("Dispatching IO!");
            0
        }

        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    #[test]

    fn build() {}

    #[test]
    fn test_io() {
        thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:9097").unwrap();
            loop {
                let (_stream, addr) = listener.accept().unwrap();
                println!("Accepted a new connection: {addr}");
            }
        });

        thread::sleep(Duration::from_millis(100));
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9097"));
        let s2: Rc<dyn Source> = Rc::new(Io::new("127.0.0.1:9097"));
        e.add_source(s.clone()).unwrap();
        e.add_source(s2.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();
        e.set_enabled(s2.clone(), EventState::On).unwrap();

        e.run(100).unwrap();
        e.run(100).unwrap();
        e.run(100).unwrap();

        e.del_source(s.clone()).unwrap();
        e.del_source(s2.clone()).unwrap();
    }

    #[test]
    fn test_io_onshot() {
        thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:9098").unwrap();
            loop {
                let (_stream, addr) = listener.accept().unwrap();
                println!("Accepted a new connection: {addr}");
            }
        });

        thread::sleep(Duration::from_millis(100));
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9098"));
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::OneShot).unwrap();

        e.run(100).unwrap();
        e.run(100).unwrap();
        e.run(100).unwrap();

        e.del_source(s.clone()).unwrap();
    }
}
