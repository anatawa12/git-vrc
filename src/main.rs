#[macro_use]
mod macros;

mod git;
mod install;
mod smudge;

use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[derive(Parser)]
#[clap(author, version, about)]
enum Commands {
    Install(install::App),
    Smudge(smudge::App),
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
        Commands::Smudge(app) => app.run(),
    }
}
