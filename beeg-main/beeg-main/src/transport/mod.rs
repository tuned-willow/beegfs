use anyhow::Result;
use std::process::Command;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct ExecOutput { pub stdout: String, pub stderr: String }

pub trait Transport: Send + Sync {
    fn exec(&self, host: &str, cmd: &str) -> Result<ExecOutput>;
}

#[derive(Debug, Clone)]
struct SshTransport { user: Option<String> }

#[derive(Debug, Clone)]
struct LocalTransport;

impl Transport for SshTransport {
    fn exec(&self, host: &str, cmd: &str) -> Result<ExecOutput> {
        let target = if let Some(u) = &self.user { format!("{}@{}", u, host) } else { host.to_string() };
        let output = Command::new("ssh")
            .arg("-o").arg("BatchMode=yes")
            .arg("-o").arg("StrictHostKeyChecking=accept-new")
            .arg("-o").arg("ConnectTimeout=5")
            .arg(target)
            .arg(cmd)
            .output()?;
        Ok(ExecOutput { stdout: String::from_utf8_lossy(&output.stdout).into(), stderr: String::from_utf8_lossy(&output.stderr).into() })
    }
}

impl Transport for LocalTransport {
    fn exec(&self, _host: &str, cmd: &str) -> Result<ExecOutput> {
        let output = Command::new("sh").arg("-lc").arg(cmd).output()?;
        Ok(ExecOutput { stdout: String::from_utf8_lossy(&output.stdout).into(), stderr: String::from_utf8_lossy(&output.stderr).into() })
    }
}

pub fn from_config(cfg: &Config) -> Box<dyn Transport + Send + Sync> {
    match cfg.transport.as_str() {
        "local" => Box::new(LocalTransport),
        _ => Box::new(SshTransport { user: cfg.ssh_user.clone() }),
    }
}
