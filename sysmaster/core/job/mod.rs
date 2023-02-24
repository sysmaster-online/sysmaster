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

pub(super) use entry::JobConf;
#[allow(unused_imports)]
pub(super) use entry::{JobInfo, JobResult, JobStage};
pub(super) use manager::{JobAffect, JobManager};
pub(super) use rentry::JobKind;
// dependency:
// job_rentry -> job_entry ->
// {job_unit_entry | job_alloc} -> job_table ->
// {job_transaction | job_notify | job_stat} -> job_manager
mod alloc;
mod entry;
mod junit;
mod manager;
mod notify;
mod rentry;
mod stat;
mod table;
mod transaction;
