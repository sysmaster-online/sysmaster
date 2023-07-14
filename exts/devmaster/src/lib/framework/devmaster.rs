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

//! encapsulate all sub-managers of framework
//!

use crate::{
    config::*,
    error::*,
    framework::{control_manager::*, job_queue::*, uevent_monitor::*, worker_manager::*},
    rules::*,
};
use basic::logger::init_log_to_console;
use event::{EventState, Events};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

/// encapsulate all submanagers
#[derive(Debug)]
pub struct Devmaster {
    /// reference to events
    events: Rc<Events>,

    /// reference to worker manager
    worker_manager: Option<Rc<WorkerManager>>,
    /// reference to control manager
    control_manager: Option<Rc<ControlManager>>,
    /// reference to monitor
    monitor: Option<Rc<UeventMonitor>>,

    /// Shared by workers
    /// .0 rules
    /// .1 netif configurations
    cache: Arc<RwLock<Cache>>,
}

/// Shared by workers
#[derive(Debug)]
pub struct Cache {
    /// shared rules
    pub(crate) rules: Arc<RwLock<Rules>>,
    /// shared network interface configuration context
    pub(crate) netif_cfg_ctx: NetifConfigCtx,
}

impl Cache {
    /// generate the shared cache
    pub fn new(rules_d: Vec<String>, netif_cfg_d: Vec<String>) -> Cache {
        let rules = Rules::load_rules(rules_d, ResolveNameTime::Early);

        let mut netif_cfg_ctx = NetifConfigCtx::new();
        netif_cfg_ctx.load(netif_cfg_d);

        Cache {
            rules,
            netif_cfg_ctx,
        }
    }
}

impl Devmaster {
    /// generate a devmaster object
    pub fn new(events: Rc<Events>) -> Rc<RefCell<Devmaster>> {
        let config = DevmasterConfig::new();

        config.load(DEFAULT_CONFIG);

        init_log_to_console("devmaster", config.get_log_level());
        log::info!("daemon start");

        let cache = Cache::new(config.get_rules_d(), config.get_netif_cfg_d());

        log::info!("rules loaded");

        let _ = cache
            .rules
            .as_ref()
            .read()
            .unwrap()
            .apply_static_dev_permission()
            .log_error("failed to apply permissions on static devnode");

        let ret = Rc::new(RefCell::new(Devmaster {
            events: events.clone(),
            worker_manager: None,
            control_manager: None,
            monitor: None,
            cache: Arc::new(RwLock::new(cache)),
        }));

        // initialize submanagers
        let worker_manager = Rc::new(WorkerManager::new(
            config.get_max_workers(),
            String::from(WORKER_MANAGER_LISTEN_ADDR),
            events.clone(),
            Rc::downgrade(&ret),
        ));
        let job_queue = Rc::new(JobQueue::new(worker_manager.clone()));
        let control_manager = Rc::new(ControlManager::new(
            String::from(CONTROL_MANAGER_LISTEN_ADDR),
            worker_manager.clone(),
            job_queue.clone(),
        ));
        let monitor = Rc::new(UeventMonitor::new(job_queue.clone()));

        // configure worker manager and monitor
        worker_manager.set_job_queue(&job_queue);
        worker_manager.set_kill_workers_timer();
        monitor.set_receive_buffer(1024 * 1024 * 128);

        events
            .add_source(worker_manager.get_kill_workers_timer().unwrap())
            .unwrap();
        events.add_source(worker_manager.clone()).unwrap();
        events.add_source(control_manager.clone()).unwrap();
        events.add_source(monitor.clone()).unwrap();

        events
            .set_enabled(worker_manager.clone(), EventState::On)
            .unwrap();
        events
            .set_enabled(control_manager.clone(), EventState::On)
            .unwrap();
        events.set_enabled(monitor.clone(), EventState::On).unwrap();

        ret.as_ref().borrow_mut().worker_manager = Some(worker_manager);
        ret.as_ref().borrow_mut().control_manager = Some(control_manager);
        ret.as_ref().borrow_mut().monitor = Some(monitor);

        ret
    }

    /// run the events loop
    pub fn run(&self) {
        self.events.rloop().unwrap();
    }

    /// clean up the events source
    pub fn exit(&self) {
        self.events
            .del_source(self.worker_manager.clone().unwrap())
            .unwrap();
        self.events
            .del_source(self.control_manager.clone().unwrap())
            .unwrap();
        self.events
            .del_source(self.monitor.clone().unwrap())
            .unwrap();
    }

    /// get shared cache
    pub(crate) fn get_cache(&self) -> Arc<RwLock<Cache>> {
        self.cache.clone()
    }
}
