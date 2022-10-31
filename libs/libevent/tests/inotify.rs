#[cfg(test)]
mod test {
    use libutils::Error;
    use nix::sys::inotify::AddWatchFlags;
    use std::fs::File;
    use std::os::unix::prelude::RawFd;
    use std::path::Path;
    use std::rc::Rc;
    use std::thread;

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
            EventType::Inotify
        }

        fn epoll_event(&self) -> u32 {
            (libc::EPOLLIN) as u32
        }

        fn priority(&self) -> i8 {
            0i8
        }

        fn dispatch(&self, e: &Events) -> Result<i32, Error> {
            println!("Dispatching inotify!");
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
        thread::spawn(move || loop {
            let _ = File::create("/tmp/xxxxxxfoo.txt").unwrap();
        });

        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Timer::new());
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();

        let watch = Path::new("/tmp");
        let wd = e.add_watch(watch, AddWatchFlags::IN_ALL_EVENTS);

        e.rloop().unwrap();

        e.rm_watch(wd);

        e.del_source(s.clone()).unwrap();
    }
}
