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

use nix::errno::Errno;
use nix::sys::socket::{self, AddressFamily, MsgFlags, SockFlag, SockType, UnixAddr};
use nix::unistd;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Arc;

const INIT_SOCKET: &str = "/run/sysmaster/init";

static INSTANCE: Lazy<Arc<Result<KeepAlive, Errno>>> = Lazy::new(|| {
    let keep_alive = KeepAlive::new();
    Arc::new(keep_alive)
});

pub struct KeepAlive {
    connect_fd: i32,
}

impl KeepAlive {
    fn new() -> Result<Self, Errno> {
        let sock_path = PathBuf::from(INIT_SOCKET);
        let connect_fd = socket::socket(
            AddressFamily::Unix,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )?;

        let unix_addr = UnixAddr::new(&sock_path);
        match unix_addr {
            Ok(addr) => {
                if let Err(err) = socket::connect(connect_fd, &addr) {
                    println!("Failed to connect:{:?}", err);
                    unistd::close(connect_fd)?;
                    return Err(err);
                }
            }
            Err(err) => {
                println!("unix_addr invalid:{:?}", err);
                unistd::close(connect_fd)?;
                return Err(err);
            }
        }

        loop {
            if let Err(err) = socket::send(connect_fd, b"MANAGEABLE", MsgFlags::MSG_WAITALL) {
                if Errno::EINTR == err {
                    continue;
                }
                println!("Failed to write MANAGEABLE:{:?}", err);
                unistd::close(connect_fd)?;
                return Err(err);
            }
            return Ok(KeepAlive { connect_fd });
        }
    }

    pub fn get_fd(&self) -> i32 {
        self.connect_fd
    }

    pub fn send_unmanageable(&self) -> Result<(), Errno> {
        loop {
            if let Err(err) = socket::send(
                self.connect_fd,
                b"UNMANAGEABLE",
                socket::MsgFlags::MSG_WAITALL,
            ) {
                if Errno::EINTR == err {
                    continue;
                }
                log::error!("Failed to write UNMANAGEABLE:{:?}", err);
                unistd::close(self.connect_fd)?;
                return Err(err);
            }
            return Ok(());
        }
    }

    pub fn get_instance() -> Arc<Result<KeepAlive, Errno>> {
        INSTANCE.clone()
    }
}
