# git-vrc

Git VRC is a command line extension for git to reduce meaningless diff on git of VRC project 

This will remove local-specific attributes from `.asset`, `.prefab`, and `.unity` file on git.

## Installation

### Windows setup.exe

If you're on windows, you can use installer can be downloaded from [releases].

You can download setup.exe for latest version [here][setup-latest].

This installer not just places the exe to the place, but also runs [setup] steps shown below

### Manual installation

You can download binary from [releases] and manually install to anywhere you want.

Remember renaming executable to git-vrc and adding to PATH environment variable, and process [setup] steps below.

### Cargo Binstall

When you have [cargo binstall], you can install via cargo binstall.

```bash
cargo binstall --git https://github.com/anatawa12/git-vrc.git git-vrc
```

### Cargo install

When you have rust toolchain, you can install via cargo install.

```bash
cargo install --git https://github.com/anatawa12/git-vrc.git git-vrc
```

[cargo binstall]: https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#cargo-binaryinstall
[setup-latest]: https://github.com/anatawa12/git-vrc/releases/latest/download/git-vrc-setup.exe
[releases]: https://github.com/anatawa12/git-vrc/releases
[setup]: #setting-up-git

## Setting up git

To set up this tool for git, type the following command:

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

OR you can manually write .gitattributes like following

```gitattributes
*.asset filter=vrc eol=lf text=auto
*.prefab filter=vrc eol=lf text=auto
*.unity filter=vrc eol=lf text=auto
```

Finally, if there already are some commits with unity files,
force git to re-index unity files!

```sh
# touch all files in your repository to let git re-index files.
$ find . -type f -print0 | xargs -0 touch
# and commit this
$ git commit -am "chore: start using git-vrc"
```

## Additional configurations

### Sorting elements in the file

We can sort elements in the unityyaml file by setting `unity-sort` git attribute to true.

```gitattributes
*.asset filter=vrc eol=lf text=auto unity-sort
```

### Specifying the filter version to keep git-vrc in sync among repositories

By setting `git-vrc-filter-version` attribute to number, you can use behavior of older version of git-vrc,
or have error if the repository uses a newer version of git-vrc filtering.

```gitattributes
*.asset filter=vrc eol=lf text=auto git-vrc-filter-version=1
```

## License

<sub>

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

</sub>
