#[macro_use]
mod macros;

mod git;
mod install;

use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[derive(Parser)]
#[clap(author, version, about)]
enum Commands {
    Install(install::App),
}

fn main() -> Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .env()
        .init()
        .unwrap();
    let args: Commands = Commands::parse();

    match args {
        Commands::Install(app) => app.run(),
    }
}
