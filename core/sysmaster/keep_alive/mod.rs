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

use nix::sys::socket::{self, AddressFamily, SockFlag, SockType, UnixAddr};
use once_cell::sync::Lazy;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::sync::Arc;

use constants::INIT_SOCKET;

static INSTANCE: Lazy<Arc<KeepAlive>> = Lazy::new(|| {
    let keep_alive = KeepAlive::new();
    Arc::new(keep_alive)
});

pub struct KeepAlive {
    connect_fd: RawFd,
}

impl KeepAlive {
    fn new() -> Self {
        let sock_path = PathBuf::from(INIT_SOCKET);
        let connect_fd = socket::socket(
            AddressFamily::Unix,
            SockType::Stream,
            SockFlag::SOCK_CLOEXEC,
            None,
        )
        .expect("Failed to create socket");

        let addr = UnixAddr::new(&sock_path).expect("Failed to new UnixAddr");
        socket::connect(connect_fd, &addr).expect("Failed to connect");

        KeepAlive { connect_fd }
    }

    pub fn get_fd(&self) -> i32 {
        self.connect_fd
    }

    pub fn init() {
        let _ = INSTANCE.clone();
    }

    pub fn get_instance() -> Arc<KeepAlive> {
        INSTANCE.clone()
    }
}
