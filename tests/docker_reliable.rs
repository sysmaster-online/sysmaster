mod common;

#[test]
#[ignore]
fn docker_reliable_random_kill_service_001() {
    common::run_script("docker_reliable", "docker_reliable_random_kill_service_001");
}
