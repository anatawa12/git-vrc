use anyhow::bail;
use clap::Parser;
use std::io::{BufReader, ErrorKind, Read, Write};

#[derive(Parser)]
/// Smudge file. This is currently cat command but some feature can be added later.
pub(crate) struct App {}

impl App {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let mut stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut buf = vec![0 as u8; 8 * 1024];

        loop {
            let size = match stdin.read(&mut buf) {
                Ok(size) => size,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => bail!(e),
            };
            if size == 0 {
                break;
            }
            stdout.write(&buf[..size])?;
        }

        Ok(())
    }
}
