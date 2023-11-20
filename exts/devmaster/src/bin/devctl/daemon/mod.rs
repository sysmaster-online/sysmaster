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

//! devmaster daemon

use event::Events;
use libdevmaster::framework::devmaster::Devmaster;
use std::{env, os::unix::net::UnixDatagram, rc::Rc};

fn notify(unset_env: bool, msg: String) -> std::io::Result<()> {
    let socket_path = match env::var_os("NOTIFY_SOCKET") {
        Some(p) => p,
        None => return Ok(()),
    };

    if unset_env {
        env::remove_var("NOTIFY_SOCKET");
    }

    let sock = UnixDatagram::unbound()?;
    let len = sock.send_to(msg.as_bytes(), socket_path)?;
    if len != msg.len() {
        Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "incomplete write",
        ))
    } else {
        Ok(())
    }
}

pub fn run_daemon(config_path: &str) {
    let events = Rc::new(Events::new().unwrap());

    let devmaster = Devmaster::new(config_path, events);

    if let Err(e) = notify(false, "READY=1\n".to_string()) {
        log::warn!("Failed to notify pid 1: {}", e);
    }

    devmaster.as_ref().borrow().run();

    devmaster.as_ref().borrow().exit();
}

#[cfg(test)]
mod test {
    use super::*;
    use device::{Device, DeviceAction};
    use libdevmaster::framework::control_manager::CONTROL_MANAGER_LISTEN_ADDR;
    use std::io::Write;

    #[test]
    fn test_run_daemon() {
        /* Require root privilege, skip in ci environment.
         *
         * In addition, this test case will fail when devmaster daemon is running.
         */
        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        if dev.trigger(DeviceAction::Change).is_err() {
            return;
        }

        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let dev = Device::from_subsystem_sysname("net", "lo").unwrap();

            /* Trigger more than the number of workers. */
            dev.trigger(DeviceAction::Remove).unwrap();
            dev.trigger(DeviceAction::Add).unwrap();
            dev.trigger(DeviceAction::Add).unwrap();
            dev.trigger(DeviceAction::Change).unwrap();
            dev.trigger(DeviceAction::Change).unwrap();
            dev.trigger(DeviceAction::Change).unwrap();

            /* Sleep more than 3 seconds to wait for the workers being recycled. */
            std::thread::sleep(std::time::Duration::from_secs(4));

            let mut stream =
                std::os::unix::net::UnixStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
            stream.write_all(b"exit ").unwrap();
        });

        run_daemon("none");
    }
}
