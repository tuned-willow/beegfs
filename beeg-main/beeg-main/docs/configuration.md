# Configuration

Default path
- `~/.config/beeg/config.json` (or set `BEEG_CONFIG` to override)
- Env fallback when no file exists: `BEEG_NODES=hostA,hostB`

Schema (JSON)
- `transport`: `"ssh"` (default) or `"local"`
- `ssh_user`: optional SSH username
- `nodes`: array of node objects `{ name, host, labels[] }`

Example
```
{
  "transport": "ssh",
  "ssh_user": "beegadmin",
  "nodes": [
    { "name": "node-a", "host": "10.0.0.11", "labels": ["gpu"] },
    { "name": "node-b", "host": "10.0.0.12", "labels": ["gpu"] },
    { "name": "node-c", "host": "10.0.0.13", "labels": [] }
  ]
}
```

Selectors
- Use `-s, --selector` with commands that target nodes
- `all` selects all nodes
- Match by exact `name`, `host`, or any `labels[]` value

Environment variables
- `BEEG_CONFIG`: path to config JSON
- `BEEG_NODES`: comma-separated hosts used when no config file is present

