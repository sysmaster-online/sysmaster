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

#[rustfmt::skip]
mod common;

#[test]
#[ignore]
fn docker_config_test_dependency_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_002() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_002",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_003() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_003",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_004() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_004",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_005() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_005",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_condition_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_condition_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_condition_002() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_condition_002",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_condition_003() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_condition_003",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_assert_001() {
    common::run_script("docker_config_test", "docker_config_test_assert_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_startlimit_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_startlimit_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_exec_001() {
    common::run_script("docker_config_test", "docker_config_test_exec_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_action_001() {
    common::run_script("docker_config_test", "docker_config_test_action_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_timeout_001() {
    common::run_script("docker_config_test", "docker_config_test_timeout_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_timeout_002() {
    common::run_script("docker_config_test", "docker_config_test_timeout_002", "1");
}

#[test]
#[ignore]
fn docker_config_test_restart_001() {
    common::run_script("docker_config_test", "docker_config_test_restart_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_refuse_001() {
    common::run_script("docker_config_test", "docker_config_test_refuse_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_service_001() {
    common::run_script("docker_config_test", "docker_config_test_service_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_service_002() {
    common::run_script("docker_config_test", "docker_config_test_service_002", "1");
}

#[test]
#[ignore]
fn docker_config_test_service_003() {
    common::run_script("docker_config_test", "docker_config_test_service_003", "1");
}

#[test]
#[ignore]
fn docker_config_test_service_004() {
    common::run_script("docker_config_test", "docker_config_test_service_004", "1");
}

#[test]
#[ignore]
fn docker_config_test_env_001() {
    common::run_script("docker_config_test", "docker_config_test_env_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_kill_001() {
    common::run_script("docker_config_test", "docker_config_test_kill_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_listen_001() {
    common::run_script("docker_config_test", "docker_config_test_listen_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_socket_001() {
    common::run_script("docker_config_test", "docker_config_test_socket_001", "1");
}

/* sctl isolate cmd not implemented yet, can not run
#[test]
#[ignore]
fn docker_config_test_isolate_001() {
    common::run_script("docker_config_test", "docker_config_test_isolate_001", "1");
}
*/
