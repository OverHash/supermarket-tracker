use std::time::Duration;

pub mod config;
pub mod countdown;
pub mod error;
pub mod initialize_database;
pub mod new_world;
pub mod supermarket;
pub mod telemetry;

pub const CACHE_PATH: &str = "cache.json";
/// The amount of milliseconds to wait between performing iterations on the pages.
pub const PAGE_ITERATION_INTERVAL: Duration = Duration::from_millis(500);
/// The amount of requests to perform in parallel.
pub const CONCURRENT_REQUESTS: i64 = 2;
