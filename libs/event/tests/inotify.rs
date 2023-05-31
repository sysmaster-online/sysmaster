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
    use event::Events;
    use event::Source;
    use event::{EventState, EventType};
    use nix::sys::inotify::AddWatchFlags;
    use std::fs::File;
    use std::os::unix::prelude::RawFd;
    use std::path::Path;
    use std::rc::Rc;
    use std::thread;

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

        fn dispatch(&self, e: &Events) -> i32 {
            e.set_exit();
            0
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
