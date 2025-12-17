# BeegFS CLI Assistant (beeg)

Beeg is a lightweight, extensible command-line assistant to help manage
and enhance BeegFS clusters. It focuses on day-2 operations like querying
status across nodes, pulling and pushing settings, running targeted checks,
and offering quality-of-life helpers for administrators.

This project aims to centralize common admin actions behind a single,
consistent CLI, making it easier to interact with multiple nodes from one
workstation or management host.

## Goals

- Provide a friendly CLI for BeegFS administers
- Perform cross-node settings inspection and tweaks
- Offer safe defaults; require explicit confirmation for destructive ops
- Be easily extensible via subcommands and plugins
- Distribute as a single binary with a simple installer

## Status

Prototype with initial CLI, SSH/local transport, a modular check
framework (including `check nvidia-driver`), shell completions, and a
simple installer. Designed to grow with more cluster-aware operations.

## Quick Start

- Build: `cargo build --release`
- Run help: `./target/release/beeg --help`
- Install to `/opt/beeg/bin`: `./install.sh` (may require sudo)
- Install completions: `./install.sh --install-completions --shell zsh` (or all)

After install, ensure `/opt/beeg/bin` is on your PATH, or add a symlink:

```
sudo ln -sf /opt/beeg/bin/beeg /usr/local/bin/beeg
```

## Usage

The CLI is organized into subcommands to reflect common admin actions.
A few examples (subject to change as features land):

- `beeg status` — high-level cluster or node status
- `beeg node list` — list known nodes
- `beeg node exec -- cmd ...` — run a read-only command on nodes
- `beeg config get --key <k>` — read a config value from nodes
- `beeg config set --key <k> --value <v>` — write a config value (with confirm)
- `beeg check nvidia-driver` — check NVIDIA driver version on nodes
- `beeg check cuda` — check CUDA version on nodes
- `beeg check nvidia-fs` — check NVIDIA GPUDirect Storage kernel module
- `beeg check ofed` — check OFED/RDMA stack version
- `beeg check client-mount --mount /mnt/beegfs` — TUI with client mount checks
- `beeg check storage-target --selector <node> --targets all|id1,id2` — storage target health on a node
- `beeg completions --shell <sh> --dir <path>` — generate shell completions

Run `beeg --help` or `beeg <subcommand> --help` for detailed flags.

## Installation

- Scripted install (recommended during early development):
  - `./install.sh` builds (when cargo is available) and installs the
    binary to `/opt/beeg/bin/beeg` by default. Use `PREFIX=/some/path`
    to customize. Add `--install-completions` to generate completions to
    `PREFIX/completions`. See docs for shell-specific enablement tips.

- Manual install:
  - Copy `target/release/beeg` to a directory on your PATH, or to
    `/opt/beeg/bin` and update your PATH accordingly.

## Uninstall

If you installed via `install.sh` with the default prefix:

```
sudo rm -f /opt/beeg/bin/beeg
sudo rmdir /opt/beeg/bin 2>/dev/null || true
sudo rmdir /opt/beeg 2>/dev/null || true
```

If you installed elsewhere, remove the corresponding files/dirs.

## Development

- Prereqs: Rust toolchain (1.74+ recommended), `cargo`
- Build: `cargo build --release`
- Lint (optional): `cargo clippy` if installed
- Test: `cargo test` (tests will be added as the project grows)

### Configuration

- Default config path: `~/.config/beeg/config.json` (or set `BEEG_CONFIG`)
- Env fallback: set nodes via `BEEG_NODES=host1,host2`
- Structure includes: `transport` (`ssh`|`local`), `ssh_user`, `nodes[]`
- See `examples/config.sample.json` and docs for details

### Transport

- SSH (default): uses `ssh` with batch mode and short timeouts
- Local: run commands locally (helpful for dev/test)

### Checks

- Modular checks under `beeg check <name>`
- Available: `nvidia-driver` (version detection via `nvidia-smi`/`modinfo`)
For more details, see the docs folder:
- docs/installation.md
- docs/configuration.md
- docs/checks.md
- docs/completions.md
- docs/transport.md
## Roadmap

- Node discovery and inventory helpers
- Secure remote execution for read-only diagnostics
- Fetch/apply BeegFS configuration across nodes
- Rich status and health checks
- Shell completions and man pages
- Pluggable transports (SSH, API gateways)

## Contributing

Issues and PRs are welcome. Please keep changes small and focused.

## License

TBD. For now, all rights reserved until a license is chosen.
