use crate::git::GitConfigOptions;
use anyhow::{Result, bail};
use clap::Parser;

#[derive(Parser)]
/// Installs git-lfs
pub(crate) struct App {
    #[clap(flatten)]
    git_config_options: GitConfigOptions,

    // configuration targets
    /// configure git config
    #[clap(long)]
    config: bool,
}

impl App {
    pub(crate) fn run(mut self) -> Result<()> {
        if !self.config {
            bail!("git vrc uninstall without --config is not supported.")
        }

        self.git_config_options.default_system();

        if self.config {
            self.configure_config()?;
        }

        Ok(())
    }

    fn configure_config(&self) -> Result<()> {
        self.git_config_options.unset("filter.vrc.smudge")?;
        self.git_config_options.unset("filter.vrc.clean")?;
        //self.git_config_options.unset("filter.vrc.process")?;
        self.git_config_options.unset("filter.vrc.required")?;

        Ok(())
    }
}
