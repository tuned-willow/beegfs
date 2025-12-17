# Checks

Overview
- Modular checks run with `beeg check <name>` on selected nodes
- Output supports human (table) or `--output json`

Available checks
- `nvidia-driver`: detects NVIDIA driver version using `nvidia-smi` or `modinfo`
- `cuda`: detects CUDA version using `nvidia-smi`, `nvcc --version`, or `/usr/local/cuda/version.txt`
- `nvidia-fs`: detects NVIDIA GPUDirect Storage kernel module (`nvidia_fs`) via `modinfo` or `lsmod`
- `ofed`: detects OFED/RDMA stack version via `ofed_info -s`, `modinfo mlx5_*`, or `ibv_devinfo --version`
- `client-mount`: runs client-side mount checks in a live TUI
- `storage-target`: checks storage target presence/state from a single node

Examples
- Human table: `beeg check nvidia-driver -s all`
- Human table: `beeg check cuda -s gpu`
- Human table: `beeg check nvidia-fs -s all`
- Human table: `beeg check ofed -s all`
- Live TUI: `beeg check client-mount --mount /mnt/beegfs -s all`
- Storage targets: `beeg check storage-target --selector node-a --targets all`
- Storage targets subset: `beeg check storage-target --selector node-a --targets 101,102,205`
- JSON: `beeg --output json check nvidia-driver -s gpu`
- JSON: `beeg --output json check cuda -s all`
- JSON: `beeg --output json check nvidia-fs -s all`
- JSON: `beeg --output json check ofed -s all`

Exit behavior
- Currently prints results; future versions may return non-zero if any node fails

Adding new checks (dev)
- Add a new variant to `src/checks/mod.rs` enum `CheckCmd`
- Implement a handler similar to `check_nvidia_driver` and route it in `run_check_cmd`
- Use `transport::from_config(cfg)` and `config::select_nodes(cfg, selector)` to retrieve nodes and run commands
