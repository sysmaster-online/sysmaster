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
fn docker_config_test_seq_001() {
    common::run_script("docker_config_test", "docker_config_test_seq_001", "1");
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
