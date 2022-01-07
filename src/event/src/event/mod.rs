pub mod events;
pub mod source;

pub use events::Events;
pub use source::Source;

#[cfg(test)]
mod test {
    use super::Source;
    use std::{
        net::{TcpListener, TcpStream},
        os::unix::io::{AsRawFd, RawFd},
        rc::Rc,
    };

    #[derive(Debug)]
    struct Io {
        t: TcpStream,
        d: &'static str,
    }

    impl Io {
        fn new(s: &'static str) -> Io {
            Io {
                t: TcpStream::connect(s).unwrap(),
                d: s,
            }
        }
    }

    impl Source for Io {
        fn fd(&self) -> RawFd {
            self.t.as_raw_fd()
        }

        fn description(&self) -> &'static str {
            self.d
        }
    }

    #[test]
    fn test_run() {
        use super::Events;
        use std::thread;
        use std::time::Duration;

        thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:9097").unwrap();
            loop {
                let (_stream, addr) = listener.accept().unwrap();
                println!("Accepted a new connection: {}", addr);
            }
        });

        thread::sleep(Duration::from_millis(100));
        let mut e = Events::new();
        let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9097"));
        let s2: Rc<dyn Source> = Rc::new(Io::new("127.0.0.1:9097"));
        e.add_source(s.clone());
        e.add_source(s2.clone());
        e.run(100);
        e.run(100);
        e.run(100);
        // e.rloop();
    }
}
