//! Thin binary wrapper: tracing setup today, with argument handling and output to follow.
//!
//! No commands exist yet. They arrive in the following PRs of the v0.1.0 stack.

use anyhow::Context;
use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

/// Install the global tracing subscriber, honouring `RUST_LOG`.
fn init_tracing() -> anyhow::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(filter);
    tracing_subscriber::registry()
        .with(fmt_layer)
        .try_init()
        .context("failed to initialise tracing")
}

fn main() -> anyhow::Result<()> {
    init_tracing()
}
