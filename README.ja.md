# git-vrc

Git VRC は VRC のプロジェクトで発生する意味のない diff をへらすための git 拡張です。

ローカル環境でのみ意味のある値を `.asset`、 `.prefab`、 `.unity` ファイルから git の上でのみ削除します。

## Installation

### Windows setup.exe

Windows をお使いの場合は、[releases] からダウンロードできるインストーラーを使用できます。

最新バージョンの setup.exe は [こちら][setup-latest] からダウンロードできます。

このインストーラーは単に exe を所定の場所に配置するだけでなく、下記の [setup] 手順も実行します。

### Manual installation

[releases] からバイナリをダウンロードして、任意の場所に手動でインストールできます。

実行ファイルの名前を `git-vrc` に変更し、PATH 環境変数に追加すること、そして下記の [setup] 手順を実行することを忘れないでください。

### Cargo Binstall

[cargo binstall] を使っている場合は、以下のコマンドでインストールできます：

```bash
cargo binstall --git https://github.com/anatawa12/git-vrc.git git-vrc
```

### Cargo install

Rust ツールチェーンがある場合は、以下のコマンドでインストールできます：

```bash
cargo install --git https://github.com/anatawa12/git-vrc.git git-vrc
```

[cargo binstall]: https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#cargo-binaryinstall
[setup-latest]: https://github.com/anatawa12/git-vrc/releases/latest/download/git-vrc-setup.exe
[releases]: https://github.com/anatawa12/git-vrc/releases
[setup]: #setting-up-git

## Setting up git

このツールを Git に設定するには、次のコマンドを実行してください：

```sh
# もしこのツールをシステム全体(git config の --system と同等)にインストールしたい場合
$ sudo git vrc install --config --system
# もしこのツールをユーザー単位(git config の --global と同等)にインストールしたい場合
$ git vrc install --config --global
```

また、 .gitattributes をレポジトリに追加するため、以下のコマンドを実行してください。

```sh
$ cd /path/to/YourUnityProject
$ git vrc install --attributes
$ git add .gitattributes
```

または以下のような .gitattributes ファイルを作成してください。

```gitattributes
*.asset filter=vrc eol=lf text=auto
*.prefab filter=vrc eol=lf text=auto
*.unity filter=vrc eol=lf text=auto
```

最後に、もしすでに unity のファイルを git にコミットしたことがある場合、
git に再 index してもらうため以下のコマンドを実行してください。

```sh
# レポジトリ内のファイルすべてを touch することで、 git に再 index してもらいます。
# all files in your repository to let git re-index files.
$ find . -type f -print0 | xargs -0 touch
# そしてコミットします
$ git commit -am "chore: start using git-vrc"
```

## Additional configurations

### Sorting elements in the file

unityyamlのファイル内の要素を fileid でソートすることができます。
`unity-sort` を attributes で set してください。

```gitattributes
*.asset filter=vrc eol=lf text=auto unity-sort
```

### Specifying the filter version to keep git-vrc in sync among repositories

`git-vrc-filter-version` 属性に数値を設定することで、古いバージョンの git-vrc の動作を使用したり、リポジトリがより新しいバージョンの git-vrc フィルタリングを使用している場合にエラーを発生させたりすることができます。

```gitattributes
*.asset filter=vrc eol=lf text=auto git-vrc-filter-version=1
```

## License

<sub>

[Apache License, Version 2.0](LICENSE-APACHE) または [MIT license](LICENSE-MIT) のどちらかお好きな方でライセンスされています。

</sub>
