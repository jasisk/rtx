#[macro_use]
extern crate log;
// #[macro_use]
// extern crate rtx_common;

use eyre::Result;
use rtx_common::Context;

mod cli;
mod logging;

fn main() -> Result<()> {
    logging::init();
    let mut ctx = Context::new();
    ctx.user_agent = format!("gimme/{}", env!("CARGO_PKG_VERSION"));
    ctx.cache_dir = ctx.xdg_cache_dir.join("gimme");
    cli::run(ctx)
}
