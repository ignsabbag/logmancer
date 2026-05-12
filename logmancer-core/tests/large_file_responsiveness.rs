use logmancer_core::LogReader;
use std::env;
use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};

const LARGE_LOG_PATH_ENV: &str = "LOGMANCER_LARGE_LOG_PATH";
const FILTER_PATTERN_ENV: &str = "LOGMANCER_FILTER_PATTERN";
const MAX_READ_LATENCY_MS_ENV: &str = "LOGMANCER_MAX_READ_LATENCY_MS";
const POLL_ITERATIONS_ENV: &str = "LOGMANCER_POLL_ITERATIONS";

const DEFAULT_FILTER_PATTERN: &str = "ERROR";
const DEFAULT_MAX_READ_LATENCY_MS: u64 = 2_000;
const DEFAULT_POLL_ITERATIONS: usize = 10;

#[test]
#[ignore = "requires LOGMANCER_LARGE_LOG_PATH pointing to a large local log file"]
fn filter_during_large_file_indexing_keeps_reads_responsive() -> io::Result<()> {
    let path = env::var(LARGE_LOG_PATH_ENV).unwrap_or_else(|_| {
        panic!(
            "{LARGE_LOG_PATH_ENV} must point to a large local log file when running this ignored test"
        )
    });
    let filter_pattern =
        env::var(FILTER_PATTERN_ENV).unwrap_or_else(|_| DEFAULT_FILTER_PATTERN.to_string());
    let max_read_latency = Duration::from_millis(env_u64(
        MAX_READ_LATENCY_MS_ENV,
        DEFAULT_MAX_READ_LATENCY_MS,
    ));
    let poll_iterations = env_usize(POLL_ITERATIONS_ENV, DEFAULT_POLL_ITERATIONS);

    eprintln!(
        "large-file responsiveness test path={path} filter={filter_pattern:?} max_read_latency={}ms poll_iterations={poll_iterations}",
        max_read_latency.as_millis()
    );

    let mut reader = LogReader::new(path)?;

    let baseline = timed("baseline read_page", || reader.read_page(0, 50))?;
    eprintln!(
        "baseline read_page duration={}ms progress={:.2}% total_lines={}",
        baseline.elapsed.as_millis(),
        baseline.result.indexing_progress,
        baseline.result.total_lines
    );

    sleep(Duration::from_millis(250));

    let filter_started = Instant::now();
    reader.filter(filter_pattern);
    eprintln!(
        "apply filter returned after {}ms",
        filter_started.elapsed().as_millis()
    );

    let mut max_main_read = Duration::ZERO;
    let mut max_filter_read = Duration::ZERO;

    for iteration in 0..poll_iterations {
        let main_page = timed("read_page", || reader.read_page(0, 50))?;
        max_main_read = max_main_read.max(main_page.elapsed);

        let filter_page = timed("read_filter", || reader.read_filter(0, 10))?;
        max_filter_read = max_filter_read.max(filter_page.elapsed);

        eprintln!(
            "iteration={iteration} read_page={}ms main_progress={:.2}% read_filter={}ms filter_progress={:.2}% filtered_lines={}",
            main_page.elapsed.as_millis(),
            main_page.result.indexing_progress,
            filter_page.elapsed.as_millis(),
            filter_page.result.indexing_progress,
            filter_page.result.total_lines
        );

        assert!(
            main_page.elapsed <= max_read_latency,
            "read_page took {}ms, exceeding {}ms responsiveness threshold",
            main_page.elapsed.as_millis(),
            max_read_latency.as_millis()
        );

        sleep(Duration::from_millis(250));
    }

    eprintln!(
        "max read_page latency={}ms max read_filter latency={}ms",
        max_main_read.as_millis(),
        max_filter_read.as_millis()
    );

    Ok(())
}

struct Timed<T> {
    result: T,
    elapsed: Duration,
}

fn timed<T>(label: &str, action: impl FnOnce() -> io::Result<T>) -> io::Result<Timed<T>> {
    let started = Instant::now();
    let result = action()?;
    let elapsed = started.elapsed();
    eprintln!("{label} took {}ms", elapsed.as_millis());
    Ok(Timed { result, elapsed })
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
