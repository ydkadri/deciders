//! The `decider` command line.

use clap::Parser;

/// Manage architecture decision records through their lifecycle.
#[derive(Debug, Parser)]
#[command(name = "decider", version, arg_required_else_help = true)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
