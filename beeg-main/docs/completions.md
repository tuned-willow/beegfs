# Shell Completions

Generate completions via the CLI
- `beeg completions --shell <bash|zsh|fish|powershell|elvish> --dir <out-dir>`
- If `--shell` is omitted, generates for all supported shells

Install with the installer
- `./install.sh --install-completions` installs to `PREFIX/completions`
- Limit shells: `./install.sh --install-completions --shell zsh --shell bash`

Enable per shell
- Bash: `source /opt/beeg/completions/beeg.bash` or copy to `/etc/bash_completion.d/`
- Zsh: place `beeg.zsh` into `$fpath` (e.g., `/usr/local/share/zsh/site-functions/_beeg`), then `autoload -U compinit && compinit`
- Fish: copy `beeg.fish` to `~/.config/fish/completions/`
- PowerShell: add `beeg.ps1` into a path loaded by `$PROFILE`
- Elvish: follow elvish completion usage with generated file

