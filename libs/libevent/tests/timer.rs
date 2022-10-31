#[cfg(test)]
mod test {
    use libutils::Error;
    use std::os::unix::prelude::RawFd;
    use std::rc::Rc;

    use libevent::Events;
    use libevent::Source;
    use libevent::{EventState, EventType};

    #[derive(Debug)]
    struct Timer();

    impl Timer {
        fn new() -> Timer {
            Self {}
        }
    }

    impl Source for Timer {
        fn fd(&self) -> RawFd {
            0
        }

        fn event_type(&self) -> EventType {
            EventType::TimerRealtime
        }

        fn epoll_event(&self) -> u32 {
            (libc::EPOLLIN) as u32
        }

        fn priority(&self) -> i8 {
            0i8
        }

        fn time_relative(&self) -> u64 {
            100000
        }

        fn dispatch(&self, e: &Events) -> Result<i32, Error> {
            println!("Dispatching timer!");
            e.set_exit();
            Ok(0)
        }

        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    #[test]
    fn test_timer() {
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Timer::new());
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }
}
