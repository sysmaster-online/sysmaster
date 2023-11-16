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

use crate::{config::*, error::*, framework::*, rules::*};
use event::*;
use log::init_log;
use std::{
    cell::RefCell,
    fs::create_dir_all,
    rc::Rc,
    sync::{Arc, RwLock},
};

/// encapsulate all submanagers
pub struct Devmaster {
    /// reference to events
    pub(crate) events: Rc<Events>,

    /// reference to worker manager
    pub(crate) worker_manager: Option<Rc<WorkerManager>>,
    /// reference to control manager
    pub(crate) control_manager: Option<Rc<ControlManager>>,
    /// reference to monitor
    pub(crate) monitor: Option<Rc<UeventMonitor>>,
    /// job queue
    pub(crate) job_queue: Option<Rc<JobQueue>>,
    /// post event source for garbage collection
    pub(crate) gc: Option<Rc<GarbageCollect>>,

    /// Shared by workers
    /// .0 rules
    /// .1 netif configurations
    pub(crate) cache: Arc<RwLock<Cache>>,
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
    pub fn new(rules_d: Vec<String>, network_d: Vec<String>) -> Cache {
        let rules = Rules::load_rules(rules_d, ResolveNameTime::Early);

        let mut netif_cfg_ctx = NetifConfigCtx::new();
        netif_cfg_ctx.load(network_d);

        Cache {
            rules,
            netif_cfg_ctx,
        }
    }
}

impl Devmaster {
    /// generate a devmaster object
    pub fn new(config_path: &str, events: Rc<Events>) -> Rc<RefCell<Devmaster>> {
        let config = DevmasterConfig::new();

        config.load(config_path);

        init_log(
            "devmaster",
            config.get_log_level(),
            config
                .get_log_targets()
                .iter()
                .map(|s| s.as_ref())
                .collect(),
            "/var/log/devmaster/devmaster.log",
            10240,
            10,
            false,
        );

        let _ = create_dir_all("/run/devmaster/");

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

        let devmaster = Rc::new(RefCell::new(Devmaster {
            events: events.clone(),
            worker_manager: None,
            control_manager: None,
            monitor: None,
            job_queue: None,
            gc: None,
            cache: Arc::new(RwLock::new(cache)),
        }));

        // initialize submanagers
        let worker_manager = Rc::new(WorkerManager::new(
            config.get_max_workers(),
            String::from(WORKER_MANAGER_LISTEN_ADDR),
            Rc::downgrade(&devmaster),
        ));
        let job_queue = Rc::new(JobQueue::new(devmaster.clone()));
        let control_manager = Rc::new(ControlManager::new(
            String::from(CONTROL_MANAGER_LISTEN_ADDR),
            worker_manager.clone(),
            job_queue.clone(),
            events.clone(),
        ));
        let monitor = Rc::new(UeventMonitor::new(job_queue.clone()));
        let post = Rc::new(GarbageCollect::new(&devmaster));

        // configure worker manager and monitor
        worker_manager.set_job_queue(&job_queue);
        monitor.set_receive_buffer(1024 * 1024 * 128);

        events.add_source(worker_manager.clone()).unwrap();
        events.add_source(control_manager.clone()).unwrap();
        events.add_source(monitor.clone()).unwrap();

        events.add_source(post.clone()).unwrap();

        events
            .set_enabled(worker_manager.clone(), EventState::On)
            .unwrap();
        events
            .set_enabled(control_manager.clone(), EventState::On)
            .unwrap();
        events.set_enabled(monitor.clone(), EventState::On).unwrap();
        events.set_enabled(post.clone(), EventState::On).unwrap();

        devmaster.borrow_mut().worker_manager = Some(worker_manager);
        devmaster.borrow_mut().control_manager = Some(control_manager);
        devmaster.borrow_mut().monitor = Some(monitor);
        devmaster.borrow_mut().job_queue = Some(job_queue);
        devmaster.borrow_mut().gc = Some(post);

        devmaster
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
        self.events.del_source(self.gc.clone().unwrap()).unwrap();
    }

    /// get shared cache
    pub(crate) fn get_cache(&self) -> Arc<RwLock<Cache>> {
        self.cache.clone()
    }
}
