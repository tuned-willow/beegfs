# Installation

- Build release: `cargo build --release`
- Install to `/opt/beeg/bin`: `./install.sh` (may require sudo)
- Custom prefix: `PREFIX=/some/path ./install.sh`

Shell completions
- Install completions to `PREFIX/completions`:
  - `./install.sh --install-completions` (all shells)
  - `./install.sh --install-completions --shell zsh --shell bash`
- Generate manually to a directory:
  - `beeg completions --shell <bash|zsh|fish|powershell|elvish> --dir ./completions`

Enable completions (examples)
- Bash: `source /opt/beeg/completions/beeg.bash` or copy to `/etc/bash_completion.d/`
- Zsh: copy `beeg.zsh` to a directory in `$fpath` (e.g., `/usr/local/share/zsh/site-functions/_beeg`), then `autoload -U compinit && compinit`
- Fish: copy `beeg.fish` to `~/.config/fish/completions/`
- PowerShell: `beeg.ps1` into a `$PROFILE`-loaded path
- Elvish: `use` the generated completion file per Elvish docs

Uninstall
- `./install.sh --uninstall`
- Remove symlinks or PATH entries if you added any manually

PATH
- Ensure `/opt/beeg/bin` is on your PATH or create a symlink:
  - `sudo ln -sf /opt/beeg/bin/beeg /usr/local/bin/beeg`

