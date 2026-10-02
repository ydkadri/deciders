//! Thin binary wrapper: tracing setup, argument handling and output only.

mod output;

use anyhow::Context;
use tracing::debug;
use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

/// Install the global tracing subscriber, honouring `RUST_LOG`.
fn init_tracing() -> anyhow::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
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
    init_tracing()?;
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "world".to_owned());
    debug!(%name, "building greeting");
    let greeting = decider_adr::greet(&name).context("could not build greeting")?;
    output::line(&greeting);
    Ok(())
}
