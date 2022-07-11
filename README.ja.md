# git-vrc

Git VRC は VRC のプロジェクトで発生する意味のない diff をへらすための git 拡張です。

ローカル環境でのみ意味のある値を `.asset`、 `.prefab`、 `.unity` ファイルから git の上でのみ削除します。

## Installation

このツールはリリースされてませんが、 cargo を使用して以下のコマンドでインストール可能です。

このツールは zip ファイル、 linux と macos 向けに homebrew、 windows 向けに msi インストーラで公開する予定です。

```sh
# もし rust をインストールしていなければ、以下のリンクの通り rust をインストールしてください。
# https://www.rust-lang.org/tools/install
$ cargo install --locked --git 'https://github.com/anatawa12/git-vrc.git'
```

このツールを git にインストールするため、以下のコマンドを実行してください。

```sh
# もしこのツールをシステム全体(git config の --system と同等)にインストールしたい場合
$ sudo git vrc install --config
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
*.asset filter=vrc eol=lf text
*.prefab filter=vrc eol=lf text
*.unity filter=vrc eol=lf text
```

最後に、もしすでに unity のファイルを git にコミットしたことがある場合、
git に再 index してもらうため以下のコマンドを実行してください。

```sh
# レポジトリ内のファイルすべてを touch することで、 git に再 index してもらいます。
# all files in your repository to let git re-index files.
$ find . -type f -print0 | xargs -0 touch
# そしてコミットします
$ git commit -a "chore: start using git-vrc"
```

## License

<sub>

[Apache License, Version 2.0](LICENSE-APACHE) または [MIT license](LICENSE-MIT) のどちらかお好きな方でライセンスされています。

</sub>
