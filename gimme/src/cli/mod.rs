use clap::{Parser, Subcommand};
use eyre::Result;
use rtx_common::Context;

mod node;

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[clap(subcommand)]
    subcmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Node(node::Node),
}

pub trait Command {
    fn run(&self, ctx: Context) -> Result<()>;
}

pub fn run(ctx: Context) -> Result<()> {
    debug!(
        "args: {}",
        std::env::args().skip(1).collect::<Vec<_>>().join(" ")
    );
    let cli = Cli::parse();
    let cmd: Box<dyn Command> = match cli.subcmd {
        Commands::Node(node) => Box::new(node),
    };
    cmd.run(ctx)
}
