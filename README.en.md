# git-vrc

Git VRC is a command line extension for git to reduce meaningless diff on git of VRC project 

This will remove local-specific attributes from `.asset`, `.prefab`, and `.unity` file on git.

## Installation

This tool is not yet published however, you can install via cargo with the following command.

It's planned to publish this tool via .zip file, homebrew (for linux and macos), and msi installer (for windows).

```sh
# If you are not installed rust,
# please follow the following instruction to install rust
# https://www.rust-lang.org/tools/install
$ cargo install --locked --git 'https://github.com/anatawa12/git-vrc.git'
```

To install this tool to git, type the following command:

```sh
# if you want to install system wide (git config --system wide)
$ sudo git vrc install --config --system
# if you want to install user globally (git config --global wide)
$ git vrc install --config --global
```

And to add .gitattributes to your repository, run the following command.

```sh
$ cd /path/to/YourUnityProject
$ git vrc install --attributes
$ git add .gitattributes
```

OR you can manually write .gitattributes as following

```gitattributes
*.asset filter=vrc eol=lf text
*.prefab filter=vrc eol=lf text
*.unity filter=vrc eol=lf text
```

Finally, if there already are some commits with unity files,
force git to re-index unity files!

```sh
# touch all files in your repository to let git re-index files.
$ find . -type f -print0 | xargs -0 touch
# and commit this
$ git commit -a "chore: start using git-vrc"
```

## License

<sub>

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

</sub>
