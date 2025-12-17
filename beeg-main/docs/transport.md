# Transport

SSH transport (default)
- Executes commands on remote nodes via `ssh`
- Uses: `-o BatchMode=yes`, `-o StrictHostKeyChecking=accept-new`, `-o ConnectTimeout=5`
- Set `ssh_user` in config to force `user@host`
- Ensure SSH keys/agent are configured for non-interactive auth

Local transport
- Set `"transport": "local"` in config to execute commands locally
- Useful for development or when node tools are locally available

