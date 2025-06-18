use clap::{ArgGroup, Parser};
use log::debug;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn get_path_command(args: &[impl AsRef<OsStr>]) -> Option<PathBuf> {
    let mut result = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?
        .wait_with_output()
        .ok()?
        .stdout;
    if result.is_empty() {
        return None;
    }
    // remove trailing '\n'
    result.pop();
    Some(PathBuf::from(std::str::from_utf8(&result).ok()?))
}

#[derive(Parser)]
#[clap(group(
    ArgGroup::new("git-config")
        .required(false)
        .args(&["system", "global", "local", "worktree"]),
))]
/// git config options
pub(crate) struct GitConfigOptions {
    /// --system in git config
    #[clap(long)]
    system: bool,
    /// --global in git config
    #[clap(long)]
    global: bool,
    /// --local in git config
    #[clap(long)]
    local: bool,
    /// --worktree in git config
    #[clap(long)]
    worktree: bool,
}

impl GitConfigOptions {
    pub(crate) fn set_any(&self) -> bool {
        self.system || self.global || self.local || self.worktree
    }

    pub(crate) fn exists(&self, key: &str, anywhere: bool) -> io::Result<bool> {
        let mut command = Command::new("git");
        command.stdin(Stdio::null()).stdout(Stdio::null());
        command.arg("config");
        if !anywhere {
            self.options(&mut command);
        }
        command.arg("--").arg(key);
        Ok(command.status()?.success())
    }

    pub(crate) fn set(&self, key: &str, value: &str) -> io::Result<()> {
        let mut command = Command::new("git");
        command.stdin(Stdio::null()).stdout(Stdio::null());
        command.arg("config");
        self.options(&mut command);
        command.arg("--").arg(key).arg(value);
        let status = command.status()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "git config command returns non-zero value",
            ));
        }
        Ok(())
    }

    pub(crate) fn unset(&self, key: &str) -> io::Result<()> {
        let mut command = Command::new("git");
        command.stdin(Stdio::null()).stdout(Stdio::null());
        command.arg("config");
        command.arg("unset");
        self.options(&mut command);
        command.arg("--").arg(key);
        let status = command.status()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "git config command returns non-zero value",
            ));
        }
        Ok(())
    }

    pub(crate) fn default_system(&mut self) {
        if !self.set_any() {
            self.system = true;
        }
    }

    fn options(&self, cmd: &mut Command) {
        if self.system {
            cmd.arg("--system");
        }
        if self.global {
            cmd.arg("--global");
        }
        if self.local {
            cmd.arg("--local");
        }
        if self.worktree {
            cmd.arg("--worktree");
        }
    }
}

pub(crate) fn repo_root() -> Option<PathBuf> {
    get_path_command(&["rev-parse", "--show-toplevel"])
}

pub(crate) fn check_attr(
    attrs: &[impl AsRef<OsStr>],
    targets: &[impl AsRef<OsStr>],
) -> io::Result<GitCheckAttrResult> {
    let mut command = Command::new("git");
    command.arg("check-attr").arg("-z");
    command.args(attrs).arg("--").args(targets);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command.stdin(Stdio::null());

    let output = command.spawn()?.wait_with_output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "git check-attr command returns non-zero value",
        ));
    }
    let output = match String::from_utf8(output.stdout) {
        Ok(output) => output,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "check-attr returns non-utf8",
            ));
        }
    };

    debug!("got output: {:?}", output);

    Ok(GitCheckAttrResult {
        str: output,
        index: 0,
    })
}

pub(crate) struct GitCheckAttrResult {
    str: String,
    index: usize,
}

impl Iterator for GitCheckAttrResult {
    type Item = (String, String, String);

    fn next(&mut self) -> Option<Self::Item> {
        if self.str.len() == self.index {
            return None;
        }
        let begin = self.index;
        debug!("find since {:?}", self.index);

        self.index += self.str[self.index..].find('\0').expect("no \\0 found");
        let first_sep = self.index;
        self.index += 1;

        self.index += self.str[self.index..].find('\0').expect("no \\0 found");
        let second_sep = self.index;
        self.index += 1;

        self.index += self.str[self.index..].find('\0').expect("no \\0 found");
        let third_sep = self.index;
        self.index += 1;

        self.index = third_sep;
        unsafe {
            Some((
                self.str.get_unchecked(begin..first_sep).to_string(),
                self.str
                    .get_unchecked((first_sep + 1)..second_sep)
                    .to_string(),
                self.str
                    .get_unchecked((second_sep + 1)..third_sep)
                    .to_string(),
            ))
        }
    }
}
