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
