use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub host: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default = "default_transport")] 
    pub transport: String, // "ssh" | "local"
    #[serde(default)]
    pub ssh_user: Option<String>,
}

fn default_transport() -> String { "ssh".to_string() }

pub fn default_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("BEEG_CONFIG") { return PathBuf::from(p); }
    if let Some(dir) = dirs::config_dir() {
        return dir.join("beeg").join("config.json");
    }
    PathBuf::from("./beeg.config.json")
}

pub fn load(explicit: Option<&std::path::PathBuf>) -> Result<Config> {
    let path = if let Some(p) = explicit { p.clone() } else { default_config_path() };
    if path.exists() {
        let data = fs::read_to_string(&path)
            .with_context(|| format!("reading config file: {}", path.display()))?;
        let cfg: Config = serde_json::from_str(&data)
            .with_context(|| format!("parsing config file: {}", path.display()))?;
        Ok(cfg)
    } else {
        // env fallback
        let nodes = std::env::var("BEEG_NODES").ok().map(|s| {
            s.split(',')
                .filter(|x| !x.trim().is_empty())
                .enumerate()
                .map(|(i, host)| Node { name: format!("node-{}", i+1), host: host.trim().to_string(), labels: vec![] })
                .collect::<Vec<_>>()
        }).unwrap_or_default();
        Ok(Config { nodes, transport: default_transport(), ssh_user: None })
    }
}

pub fn select_nodes<'a>(cfg: &'a Config, selector: &str) -> Vec<&'a Node> {
    if selector.eq_ignore_ascii_case("all") { return cfg.nodes.iter().collect(); }
    cfg.nodes
        .iter()
        .filter(|n| n.name == selector || n.host == selector || n.labels.iter().any(|l| l == selector))
        .collect()
}
