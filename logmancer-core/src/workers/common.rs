use std::time::Duration;

pub fn wait(millis: u64) {
    let wait_duration = Duration::from_millis(millis);
    let now = std::time::Instant::now();
    while now.elapsed() < wait_duration {
        std::thread::sleep(wait_duration);
    }
}
