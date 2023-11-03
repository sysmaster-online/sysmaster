use std::{collections::HashSet, time::Duration};

use notify::{Config, PollWatcher, RecursiveMode, Watcher};
use rustix::fs::UnmountFlags;
use tap::Pipe;
use tokio::{
    fs, select,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    unit::UnitId,
    util::mount::{mount_point_to_unit_name, unmount, ProcMountInfoLine},
};

use super::guard;

#[derive(Debug)]
pub(crate) enum Message {
    Register(UnitId),
    Remove(UnitId),
}

pub(crate) struct MountMonitorStore {
    map: HashSet<UnitId>,
    guard: Sender<guard::Message>,
}

impl MountMonitorStore {
    pub(crate) fn new(guard: Sender<guard::Message>) -> Self {
        Self {
            map: Default::default(),
            guard,
        }
    }
    pub(crate) fn run(mut self, mut receiver: Receiver<Message>) -> JoinHandle<()> {
        let (tx, mut rx) = mpsc::channel(1);
        let mut watcher = PollWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            Config::default()
                .with_compare_contents(true)
                .with_poll_interval(Duration::from_secs_f64(0.5)),
        )
        .unwrap();
        watcher
            .watch("/proc/self/mountinfo".as_ref(), RecursiveMode::NonRecursive)
            .unwrap();
        tokio::spawn(async move {
            loop {
                select! {
                    Some(msg) = receiver.recv() => {
                        match msg {
                            Message::Register(id) => {
                                self.map.insert(id);
                            }
                            Message::Remove(id) => {
                                self.map.remove(&id);
                            }
                        }
                    }
                    Some(msg) = rx.recv() => {
                        let _ = msg.unwrap();
                        let mount_info = fs::read_to_string("/proc/self/mountinfo").await.unwrap();
                        let mount_info = mount_info
                            .lines()
                            .map(|line| ProcMountInfoLine::parse(line).mount_point.pipe(|m| UnitId::from(mount_point_to_unit_name(&m).as_str())))
                            .collect::<HashSet<_>>();
                        let dead = self.map.difference(&mount_info).cloned().collect::<Vec<_>>();
                        for unit_id in dead {
                            self.map.remove(&unit_id);
                            self.guard.send(guard::Message::NotifyDead(unit_id)).await.unwrap();
                        }
                    }
                }
            }
        })
    }
}

#[test]
fn test() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (gtx, mut grx) = mpsc::channel(4);
            let (tx, rx) = mpsc::channel(4);
            MountMonitorStore::new(gtx).run(rx);
            tx.send(Message::Register(UnitId::from("tmp-sysrstest.mount")))
                .await
                .unwrap();
            let test_mount = crate::Rc::new(crate::fstab::MountInfo {
                fs_spec: "tmpfs".into(),
                mount_point: (&"/tmp/sysrstest" as &dyn AsRef<std::path::Path>)
                    .as_ref()
                    .into(),
                vfs_type: "tmpfs".into(),
                mount_options: "".into(),
            });
            crate::util::mount::mount(test_mount.clone(), rustix::fs::MountFlags::empty()).unwrap();
            println!("{:?}", grx.recv().await.unwrap());
            unmount(test_mount.clone(), UnmountFlags::empty()).unwrap();
            println!("{:?}", grx.recv().await.unwrap());
        });
}
