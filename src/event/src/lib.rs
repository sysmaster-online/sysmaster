#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[warn(dead_code)]
#[derive(Debug)]
#[warn(unused_variables)]
struct Todo {
    unimplemented: i32,
}

type Type = Todo;
type RetType = Todo;


#[derive(Debug)]
enum EVENT_TYPE {
    EVENT_OFF = 0,
    EVENT_ON = 1,
    EVENT_ONESHOT = -1,
}

#[derive(Debug)]
enum EVENT_STATE {
    EVENT_INITIAL,
    EVENT_ARMED,
    EVENT_PENDING,
    EVENT_RUNNING,
    EVENT_EXITING,
    EVENT_FINISHED,
    EVENT_PREPARING,
}

#[derive(Debug)]
enum EVENT_PRIORITY {
    EVENT_PRIORITY_IMPORTANT = -100,
    EVENT_PRIORITY_NORMAL = 0,
    EVENT_PRIORITY_IDLE = 100, 
}

struct Event {
    n_ref: u32,
    epoll_fd: i32,
    wd_fd: i32,

}

impl Event {
    fn new(arg: Type) -> Self {
        unimplemented!();
    }

    fn default(arg: Type) -> RetType {
        unimplemented!();
    }

    fn for_loop(arg: Type) -> RetType {
        unimplemented!();
    }

    fn dispatch(arg: Type) -> RetType {
        unimplemented!();
    }

    fn run(arg: Type) -> RetType {
        unimplemented!();
    }

    fn wait(arg: Type) -> RetType {
        unimplemented!();
    }

    fn exit(arg: Type) -> RetType {
        unimplemented!();
    }

    fn refer(arg: Type) -> RetType {
        unimplemented!();
    }

    fn unrefer(arg: Type) -> RetType {
        unimplemented!();
    }

    fn now(arg: Type) -> RetType {
        unimplemented!();
    }

    ///set get functions
    fn get_fd(arg: Type) -> RetType {
        unimplemented!();
    }

    fn get_state(arg: Type) -> RetType {
        unimplemented!();
    }

    fn exit_code(arg: Type) -> RetType {
        unimplemented!();
    }

    fn set_watchdog(arg: Type) -> RetType {
        unimplemented!();
    }

    fn get_watchdog(arg: Type) -> RetType {
        unimplemented!();
    }

    fn get_iteration(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_io(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_time(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_signal(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_inotify(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_child(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_defer(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_post(arg: Type) -> RetType {
        unimplemented!();
    }

    fn add_exit(arg: Type) -> RetType {
        unimplemented!();
    }
}