use tokio::task::JoinHandle;
use tracing::{subscriber::set_global_default, Span, Subscriber};
use tracing_subscriber::{
    fmt::{format::FmtSpan, MakeWriter},
    layer::SubscriberExt,
    EnvFilter, Registry,
};

/// Composes multiple layers into a `tracing`'s subscriber.
pub fn get_tracing_subscriber<Sink>(sink: Sink) -> impl Subscriber
where
    // we use Higher-Rank Trait Bounds (HRTBs) https://doc.rust-lang.org/nomicon/hrtb.html
    // here because we need to Sink to implement `MakeWriter` trait for all choices
    // of our lifetime parameter `'a`
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,supermarket_tracker=debug".into());
    let formatting_layer = tracing_subscriber::fmt::layer()
        .with_writer(sink)
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE);

    Registry::default().with(env_filter).with(formatting_layer)
}

/// Registers a subscriber as the global default to process span data.
///
/// This method should only be called once in an executable.
///
/// # Panics
/// If the global default has already been set (which may be the case if this function is called multiple times in an executable).
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    set_global_default(subscriber).expect("Failed to set subscriber");
}

/// Spawns a new blocking [`tokio::task`], where the spawned function's span
/// extends from the current span.
///
/// This is useful for heavy computational tasks which may block the executor.
///
/// Common use cases include creating password hashes.
#[allow(dead_code)]
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
