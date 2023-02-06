use clap::Parser;
use std::io;

#[derive(Parser)]
/// Smudge file. This is currently cat command but some feature can be added later.
pub(crate) struct App {
    #[clap(long = "file")]
    file: Option<String>,
}

impl App {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();

        io::copy(&mut stdin, &mut stdout)?;
        io::Write::flush(&mut stdout)?;

        Ok(())
    }
}
