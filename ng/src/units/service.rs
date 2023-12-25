use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::{UnitTrait, UnitType};

enum _State {
    _Running,
    _Stopped,
}

impl _State {
    fn _is_running(&self) -> bool {
        match self {
            _State::_Running => true,
            _State::_Stopped => false,
        }
    }
}

struct Config {
    _service: ServiceConfig,
}

struct ServiceConfig {
    _first_config: i32,
}

pub struct Service {
    _state: (Sender<_State>, Receiver<_State>),
    _config: Config,
}

impl Service {
    pub fn new() -> Self {
        Self {
            _state: channel::<_State>(1),
            _config: Config {
                _service: ServiceConfig { _first_config: 0 },
            },
        }
    }

    async fn _set_state(&mut self, state: _State) {
        self._state.0.send(state).await.unwrap();
    }
}

impl UnitTrait for Service {
    fn start(&self) -> bool {
        println!("Service::start");
        true
    }

    fn stop(&self) -> bool {
        todo!()
    }

    fn load(&self) -> bool {
        todo!()
    }

    fn kind(&self) -> UnitType {
        UnitType::Service
    }
}
