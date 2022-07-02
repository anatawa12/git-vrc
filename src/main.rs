mod install;

use anyhow::Result;

#[derive(clap::Parser)]
enum Commands {
    Install(install::App)
}

fn main() -> Result<()> {
    let args: Commands = Commands::parse();

    match args {
        Commands::Install(app) => app.run()
    }
}
