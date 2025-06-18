use crate::git::GitConfigOptions;
use anyhow::{Context, Result, bail};
use clap::Parser;
use log::warn;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Parser)]
/// Installs git-lfs
pub(crate) struct App {
    //// git config targets
    ///// git config target to --system
    //#[clap(long)]
    //system: bool,
    ///// git config target to --global
    //#[clap(long)]
    //global: bool,
    ///// git config target to --local
    //#[clap(long)]
    //local: bool,
    ///// git config target to --worktree
    //#[clap(long)]
    //worktree: bool,
    #[clap(flatten)]
    git_config_options: GitConfigOptions,

    // configuration targets
    /// configure git config
    #[clap(long)]
    config: bool,
    /// configure .gitattributes
    #[clap(long)]
    attributes: bool,
}

impl App {
    fn default_target(&self) -> bool {
        !self.config && !self.attributes
    }

    pub(crate) fn run(mut self) -> Result<()> {
        let config_always;
        let attributes_always;
        if self.default_target() {
            self.config = true;
            config_always = false;
            self.attributes = true;
            attributes_always = false;
        } else {
            config_always = true;
            attributes_always = true;
        }

        if !self.config && self.git_config_options.set_any() {
            bail!("git config options is not valid without --config")
        }

        self.git_config_options.default_system();

        if self.config {
            self.configure_config(config_always)?;
        }

        if self.attributes {
            self.configure_attributes(attributes_always)?;
        }

        Ok(())
    }

    fn configure_config(&self, always: bool) -> Result<()> {
        if !always {
            if self
                .git_config_options
                .exists("filter.vrc.clean", true)
                .context("git config to check exists")?
            {
                // if there's filter.vrc.clean, there's no need to
                return Ok(());
            }
        }

        self.git_config_options
            .set("filter.vrc.smudge", "git vrc smudge --file %f")?;
        self.git_config_options
            .set("filter.vrc.clean", "git vrc clean --file %f")?;
        //self.git_config_options
        //    .set("filter.vrc.process", "git vrc filter-process")?;
        self.git_config_options.set("filter.vrc.required", "true")?;

        Ok(())
    }

    fn configure_attributes(&self, always: bool) -> Result<()> {
        if !always {
            // if CWD is not git repo, this doesn't run
            if crate::git::repo_root().is_none() {
                return Ok(());
            }
            // if all required config are set, nothing to do
            if crate::git::check_attr(
                &["filter", "diff", "merge"],
                &["*.asset", "*.prefab", "*.unity"],
            )?
            .all(|(_file, _kind, value)| value == "vrc")
            {
                return Ok(());
            }
        }
        let file_path = Path::new(".gitattributes");

        // try create new .gitattributes.
        if let Ok(mut file) = OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(false)
            .open(file_path)
        {
            // if .gitattribute is new, just create it.
            for x in FILES_CONTROLLED_BY_THIS_TOOL {
                file.write(x.as_bytes())?;
                file.write(b" ")?;
                file.write(FILE_ATTRIBUTES.as_bytes())?;
                file.write(b"\n")?;
            }
            file.flush()?;
            drop(file);
            return Ok(());
        }

        // the file should be exist. open as read&write
        let mut file = OpenOptions::new().read(true).write(true).open(file_path)?;

        let mut attr_file = String::new();
        file.read_to_string(&mut attr_file)?;
        file.seek(SeekFrom::Start(0))?;
        file.write(update_attributes_file(attr_file.lines()).as_bytes())?;
        file.flush()?;
        drop(file);

        Ok(())
    }
}

fn update_attributes_file<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    let mut result = String::new();
    let mut added = HashSet::with_capacity(3);

    for line in lines {
        if let Some(first_non_ws) = line.find(|c: char| !c.is_ascii_whitespace()) {
            let trimmed = &line[first_non_ws..];
            // not a comment line
            if trimmed.as_bytes()[0] != b'#' {
                let name_end = trimmed
                    .find(|c: char| c.is_ascii_whitespace())
                    .unwrap_or(trimmed.len());
                let name = &trimmed[..name_end];
                if FILES_CONTROLLED_BY_THIS_TOOL.contains(&name) {
                    added.insert(name);
                    result.push_str(&line[..first_non_ws]);
                    result.push_str(&trimmed[..name_end]);
                    result.push_str(&add_attributes(&trimmed[name_end..], "*.asset" == name));
                    result.push('\n');
                    continue;
                }
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    for name in FILES_CONTROLLED_BY_THIS_TOOL {
        if !added.contains(name) {
            result.push_str(name);
            result.push(' ');
            result.push_str(FILE_ATTRIBUTES);
            if &"*.asset" == name {
                result.push_str(" unity-sort");
            }
            result.push('\n');
        }
    }

    result
}

fn add_attributes(mut attrs: &str, set_unity_sort: bool) -> String {
    // fast path: if no attributes are defined, append our attributes
    if attrs.is_empty() {
        return format!(" {}", FILE_ATTRIBUTES);
    }

    if attrs.trim().is_empty() {
        return format!("{}{}", attrs, FILE_ATTRIBUTES);
    }

    // parse & check for existence
    let mut result = attrs.to_owned();
    let mut filter_found = false;
    let mut text_found = false;
    let mut eol_found = false;
    let mut unity_sort_found = false;

    loop {
        if let Some(non_ws) = attrs.find(|c: char| !c.is_ascii_whitespace()) {
            attrs = &attrs[non_ws..];
        } else {
            break;
        }
        let attr: &str;
        if let Some(ws) = attrs.find(|c: char| c.is_ascii_whitespace()) {
            (attr, attrs) = attrs.split_at(ws);
        } else {
            (attr, attrs) = (attrs, "");
        };

        if attr == "-text" {
            die!("-text for unity files found! git-vrc requires text format of unit files!")
        } else if attr == "text" || attr.starts_with("text=") {
            text_found = true
        } else if attr.starts_with("filter=") {
            if attr != "filter=vrc" {
                die!("configured attribute filter for unity files is not 'vrc'!");
            }
            filter_found = true
        } else if attr.starts_with("eol=") {
            if attr != "eol=lf" {
                warn!(
                    "configured attribute eol for unity files is not 'lf'! \
                    Unity files will use lf on all platforms!"
                );
            }
            eol_found = true
        } else if attr == "unity-sort" || attr.starts_with("unity-sort=") {
            unity_sort_found = true;
        }
    }

    fn append_attr(result: &mut String, attr: &str) {
        if !result
            .chars()
            .rev()
            .next()
            .map(|c| c.is_ascii_whitespace())
            .unwrap_or(false)
        {
            result.push(' ');
        }
        result.push_str(attr);
    }

    if !filter_found {
        append_attr(&mut result, "filter=vrc");
    }

    if !text_found {
        append_attr(&mut result, "text");
    }

    if !eol_found {
        append_attr(&mut result, "eol=lf");
    }

    if !unity_sort_found && set_unity_sort {
        append_attr(&mut result, "unity-sort");
    }

    result
}

#[cfg(test)]
mod test {
    #[test]
    fn update_attributes_file() {
        assert_eq!(
            super::update_attributes_file(["* text=auto", "* eol=lf",].into_iter()),
            format!(
                concat!(
                    "* text=auto\n",
                    "* eol=lf\n",
                    "*.asset {0} unity-sort\n",
                    "*.prefab {0}\n",
                    "*.unity {0}\n",
                ),
                super::FILE_ATTRIBUTES
            )
        );

        assert_eq!(
            super::update_attributes_file([].into_iter()),
            format!(
                concat!(
                    "*.asset {0} unity-sort\n",
                    "*.prefab {0}\n",
                    "*.unity {0}\n",
                ),
                super::FILE_ATTRIBUTES
            )
        );

        assert_eq!(
            super::update_attributes_file(
                ["*.asset  eol=lf", "*.prefab text eol=lf   ",].into_iter()
            ),
            format!(
                concat!(
                    "*.asset  eol=lf filter=vrc text unity-sort\n",
                    "*.prefab text eol=lf   filter=vrc\n",
                    "*.unity {0}\n",
                ),
                super::FILE_ATTRIBUTES
            )
        );

        assert_eq!(
            super::update_attributes_file(
                [
                    format!("*.asset {0} unity-sort", super::FILE_ATTRIBUTES).as_str(),
                    format!("*.prefab {0}", super::FILE_ATTRIBUTES).as_str(),
                    format!("*.unity {0}", super::FILE_ATTRIBUTES).as_str(),
                ]
                .into_iter()
            ),
            format!(
                concat!(
                    "*.asset {0} unity-sort\n",
                    "*.prefab {0}\n",
                    "*.unity {0}\n",
                ),
                super::FILE_ATTRIBUTES
            )
        );
    }
}

const FILE_ATTRIBUTES: &'static str = "filter=vrc eol=lf text=auto";

const FILES_CONTROLLED_BY_THIS_TOOL: &'static [&'static str] = &["*.asset", "*.prefab", "*.unity"];
