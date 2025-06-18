#[macro_use]
mod macros;

mod clean;
mod git;
mod install;
mod smudge;
mod uninstall;
mod yaml;

use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[derive(Parser)]
#[clap(author, version, about)]
enum Commands {
    Install(install::App),
    Uninstall(uninstall::App),
    Smudge(smudge::App),
    Clean(clean::App),
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
        Commands::Uninstall(app) => app.run(),
        Commands::Smudge(app) => app.run(),
        Commands::Clean(app) => app.run(),
    }
}
