use crate::utils::lazylock::EVICTION;
use pingora_cache::eviction::EvictionManager;
use pingora_http::Method;
use pingora_http::StatusCode;
use pingora_http::Version;
use prometheus::{register_histogram, register_int_counter, register_int_counter_vec, register_int_gauge, Histogram, IntCounter, IntCounterVec, IntGauge};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

pub struct MetricTypes {
    pub method: Method,
    pub upstream: Arc<str>,
    pub code: Option<StatusCode>,
    pub latency: Duration,
    pub version: Version,
}

pub static OPEN_FILES: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_open_files", "Number of open file descriptors").unwrap());
pub static LOGGING_ERRORS: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_logging_errors", "Number of log errors").unwrap());
pub static MEMORY_USAGE: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_memory_bytes", "Total memory allocated in bytes").unwrap());
pub static ACTIVE_SESSIONS: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_active_sessions", "Current number of active sessions").unwrap());
pub static REQUEST_COUNT: LazyLock<IntCounter> = LazyLock::new(|| register_int_counter!("aralez_requests_total", "Total number of requests handled by Aralez").unwrap());

pub static CACHE_SIZE_BYTES: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_cache_size_bytes", "Current cache size in bytes").unwrap());
pub static CACHE_ITEMS: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_cache_items", "Current number of cached objects").unwrap());
pub static CACHE_EVICTED_BYTES: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_cache_evicted_bytes_total", "Total bytes evicted from cache").unwrap());

pub static CACHE_EVICTED_ITEMS: LazyLock<IntGauge> = LazyLock::new(|| register_int_gauge!("aralez_cache_evicted_items_total", "Total cache items evicted").unwrap());
pub static RESPONSE_CODES: LazyLock<IntCounterVec> =
    LazyLock::new(|| register_int_counter_vec!("aralez_responses_total", "Responses grouped by status code", &["status"]).unwrap());

pub static RESPONSE_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "aralez_response_latency_seconds",
        "Response latency in seconds",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0]
    )
    .unwrap()
});

pub static REQUESTS_BY_METHOD: LazyLock<IntCounterVec> =
    LazyLock::new(|| register_int_counter_vec!("aralez_requests_by_method_total", "Number of requests by HTTP method", &["method"]).unwrap());

pub static REQUESTS_BY_UPSTREAM: LazyLock<IntCounterVec> =
    LazyLock::new(|| register_int_counter_vec!("aralez_requests_by_upstream", "Number of requests by UPSTREAM server", &["upstream"]).unwrap());

pub static REQUESTS_BY_VERSION: LazyLock<IntCounterVec> =
    LazyLock::new(|| register_int_counter_vec!("aralez_requests_by_version_total", "Number of requests by HTTP versions", &["version"]).unwrap());

pub fn calc_metrics(metric_types: &MetricTypes) {
    REQUEST_COUNT.inc();
    let version_str = match metric_types.version {
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        Version::HTTP_3 => "HTTP/3.0",
        Version::HTTP_10 => "HTTP/1.0",
        _ => "Unknown",
    };
    REQUESTS_BY_VERSION.with_label_values(&[version_str]).inc();
    RESPONSE_CODES.with_label_values(&[metric_types.code.unwrap_or(StatusCode::GONE).as_str()]).inc();
    REQUESTS_BY_METHOD.with_label_values(&[metric_types.method.as_str()]).inc();
    REQUESTS_BY_UPSTREAM.with_label_values(&[metric_types.upstream.as_ref()]).inc();
    RESPONSE_LATENCY.observe(metric_types.latency.as_secs_f64());

    if let Some(eviction) = EVICTION.get() {
        CACHE_SIZE_BYTES.set(eviction.total_size() as i64);
        CACHE_ITEMS.set(eviction.total_items() as i64);
        CACHE_EVICTED_BYTES.set(eviction.evicted_size() as i64);
        CACHE_EVICTED_ITEMS.set(eviction.evicted_items() as i64);
    }
}

#[cfg(unix)]
pub(crate) fn get_memory_usage() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<usize>().ok())
        })
        .unwrap_or(0)
        * 1024
}

#[cfg(windows)]
pub(crate) fn get_memory_usage() -> usize {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};

    unsafe {
        let handle = GetCurrentProcess();
        let mut counters = MaybeUninit::<PROCESS_MEMORY_COUNTERS>::uninit();
        
        if GetProcessMemoryInfo(
            handle,
            counters.as_mut_ptr(),
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ) != 0 
        {
            let counters = counters.assume_init();
            counters.WorkingSetSize as usize
        } else {
            0
        }
    }
}


pub fn get_open_files() -> usize {
    std::fs::read_dir("/proc/self/fd").map(|dir| dir.count()).unwrap_or(0)
}
