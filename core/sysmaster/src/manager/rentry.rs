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

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ReliLastQue {
    Load = 0,
    TargetDeps,
    GcJob,
    GcUnit,
    Clean,
    CgRealize,
    StartWhenUpheld,
    StopWhenBound,
    StopWhenUnneeded,
    Dbus,
}

impl TryFrom<u32> for ReliLastQue {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReliLastQue::Load),
            1 => Ok(ReliLastQue::TargetDeps),
            2 => Ok(ReliLastQue::GcJob),
            3 => Ok(ReliLastQue::GcUnit),
            4 => Ok(ReliLastQue::Clean),
            5 => Ok(ReliLastQue::CgRealize),
            6 => Ok(ReliLastQue::StartWhenUpheld),
            7 => Ok(ReliLastQue::StopWhenBound),
            8 => Ok(ReliLastQue::StopWhenUnneeded),
            9 => Ok(ReliLastQue::Dbus),
            v => Err(format!("input {} is invalid", v)),
        }
    }
}

/* manager */
#[allow(dead_code)]
const RELI_DB_HMNG: &str = "manager";
/* job */
pub(crate) const RELI_DB_HJOB_TRIGGER: &str = "jtrigger";
pub(crate) const RELI_DB_HJOB_SUSPENDS: &str = "jsuspends";
/* unit */
pub(crate) const RELI_DB_HUNIT_BASE: &str = "ubase";
pub(crate) const RELI_DB_HUNIT_LOAD: &str = "uload";
pub(crate) const RELI_DB_HUNIT_CONFIG: &str = "uconfig";
pub(crate) const RELI_DB_HUNIT_CGROUP: &str = "ucgroup";
pub(crate) const RELI_DB_HUNIT_CHILD: &str = "uchild";
pub(crate) const RELI_DB_HUNIT_PPS: &str = "upps";
pub(crate) const RELI_DB_HUNIT_DEP: &str = "udep";
pub(crate) const RELI_DB_HUM_NOTIFY: &str = "um-notify";
/* service */
#[allow(dead_code)]
const RELI_DB_HSERVICE_CONF: &str = "svcconf";
#[allow(dead_code)]
const RELI_DB_HSERVICE_MNG: &str = "svcmng";
/* socket */
#[allow(dead_code)]
const RELI_DB_HSOCKET_CONF: &str = "sockconf";
#[allow(dead_code)]
const RELI_DB_HSOCKET_MNG: &str = "sockmng";
#[allow(dead_code)]
const RELI_DB_HSOCKETM_FRAME: &str = "sockm-frame";
/* mount */
#[allow(dead_code)]
const RELI_DB_HMOUNT_MNG: &str = "mntmng";
#[allow(dead_code)]
const RELI_DB_HMOUNTM_FRAME: &str = "mntm-frame";
#[allow(dead_code)]
/* target */
const RELI_DB_HTARGET_MNG: &str = "tarmng";

/* timer */
const RELI_DB_HTIMER_CONF: &str = "timerconf";
const RELI_DB_HTIMER_MNG: &str = "timermng";

/* path */
const RELI_DB_HPATH_CONF: &str = "pathconf";
const RELI_DB_HPATH_MNG: &str = "pathmng";

pub const RELI_HISTORY_MAX_DBS: u32 = 22;
#[allow(dead_code)]
static RELI_HISTORY_DB_NAME: [&str; RELI_HISTORY_MAX_DBS as usize] = [
    RELI_DB_HJOB_TRIGGER,
    RELI_DB_HJOB_SUSPENDS,
    RELI_DB_HUNIT_BASE,
    RELI_DB_HUNIT_LOAD,
    RELI_DB_HUNIT_CONFIG,
    RELI_DB_HUNIT_CGROUP,
    RELI_DB_HUNIT_CHILD,
    RELI_DB_HUNIT_PPS,
    RELI_DB_HUNIT_DEP,
    RELI_DB_HUM_NOTIFY,
    RELI_DB_HSERVICE_CONF,
    RELI_DB_HSERVICE_MNG,
    RELI_DB_HSOCKET_CONF,
    RELI_DB_HSOCKET_MNG,
    RELI_DB_HSOCKETM_FRAME,
    RELI_DB_HMOUNT_MNG,
    RELI_DB_HMOUNTM_FRAME,
    RELI_DB_HTARGET_MNG,
    RELI_DB_HTIMER_CONF,
    RELI_DB_HTIMER_MNG,
    RELI_DB_HPATH_CONF,
    RELI_DB_HPATH_MNG,
];
