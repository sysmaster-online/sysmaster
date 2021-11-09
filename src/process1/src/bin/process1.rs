mod util;
fn main() {
    util::logging::init_log_with_console("process1", 0);
    log::info!("process one test");
}
