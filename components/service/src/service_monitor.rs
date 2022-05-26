use super::service_config::{ServiceConfig, ServiceConfigItem};
use process1::watchdog;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct ServiceMonitor {
    data: RefCell<ServiceMonitorData>,
}

impl ServiceMonitor {
    pub(super) fn new(configr: &Rc<ServiceConfig>) -> ServiceMonitor {
        ServiceMonitor {
            data: RefCell::new(ServiceMonitorData::new(configr)),
        }
    }

    pub(super) fn start_action(&self) {
        self.data.borrow_mut().start_action()
    }
}

struct ServiceMonitorData {
    // associated objects
    config: Rc<ServiceConfig>,

    // owned objects
    watchdog_original_usec: u64,
    watchdog_override_usec: u64,
    watchdog_override_enable: bool,
}

// the declaration "pub(self)" is for identification only.
impl ServiceMonitorData {
    pub(self) fn new(configr: &Rc<ServiceConfig>) -> ServiceMonitorData {
        ServiceMonitorData {
            config: Rc::clone(configr),
            watchdog_original_usec: u64::MAX,
            watchdog_override_usec: 0,
            watchdog_override_enable: false,
        }
    }

    pub(self) fn start_action(&mut self) {
        if let ServiceConfigItem::ScItemWatchdogSec(Some(wd_sec)) =
            self.config.get(&ServiceConfigItem::ScItemWatchdogSec(None))
        {
            self.watchdog_original_usec = wd_sec;
        }
        self.watchdog_override_enable = false;
        self.watchdog_override_usec = u64::MAX;
    }

    /// 软件看门狗，在service中的watchdog主要是定期接收服务进程发来的READY=1的消息，如果没收到则执行杀死或重启操作。
    /// 打开看门狗，需要比较原有的超时时间和复写的超时时间，并判断如果是非法值则要关闭看门狗
    /// 直接调用recvmsg系统调用从socket文件中读取字符串，再判断是否是看门狗相关的字段，如READY=1
    /// 功能未完全实现，依赖timer sd-event的实现
    fn start_watchdog(self) {
        // 允许覆盖timeout则使用覆盖值
        let watchdog_usec = if self.watchdog_override_enable {
            self.watchdog_override_usec
        } else {
            self.watchdog_original_usec
        };
        // timeout为0则关闭看门狗
        if watchdog_usec == 0 || watchdog_usec == u64::MAX {
            self.stop_watchdog()
        }
        watchdog::register_timer();
        watchdog::event_source_set_enabled(true);
    }

    fn stop_watchdog(self) {
        watchdog::event_source_set_enabled(false);
    }
}
