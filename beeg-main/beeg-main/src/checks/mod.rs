use crate::{config, transport};
use clap::{Args, Subcommand};
use comfy_table::{Table, presets::UTF8_FULL};
use std::collections::BTreeMap;
pub mod client;

#[derive(Debug, Subcommand)]
pub enum CheckCmd {
    /// Check NVIDIA driver presence and version on nodes
    NvidiaDriver(NvidiaArgs),
    /// Check CUDA toolkit/version on nodes
    Cuda(CudaArgs),
    /// Check NVIDIA GPUDirect Storage (nvidia-fs) kernel module
    NvidiaFs(NvidiaFsArgs),
    /// Check OFED / RDMA stack version
    Ofed(OfedArgs),

    /// Client mount checks with live TUI
    ClientMount(ClientMountArgs),
    /// Storage target health check from a single node
    StorageTarget(StorageTargetArgs),
}

#[derive(Debug, Args)]
pub struct ClientMountArgs {
    /// Target mountpoint (e.g., /mnt/beegfs)
    #[arg(long)]
    pub mount: String,
    /// Node selector: name/ip/label, or 'all'
    #[arg(short, long, default_value = "all")]
    pub selector: String,
    /// Timeout seconds per operation
    #[arg(long, default_value_t = 10)]
    pub timeout: u64,
}

#[derive(Debug, Args)]
pub struct StorageTargetArgs {
    /// Node to run the check on (name/host/label); must resolve to one node
    #[arg(long, visible_alias = "node")]
    pub selector: String,
    /// Target IDs: comma-separated or 'all'
    #[arg(long, default_value = "all")]
    pub targets: String,
    /// Timeout seconds per operation
    #[arg(long, default_value_t = 10)]
    pub timeout: u64,
}

#[derive(Debug, Args)]
pub struct NvidiaArgs {
    /// Node selector: name/ip/label, or 'all'
    #[arg(short, long, default_value = "all")]
    pub selector: String,
}

#[derive(Debug, Args)]
pub struct CudaArgs {
    /// Node selector: name/ip/label, or 'all'
    #[arg(short, long, default_value = "all")]
    pub selector: String,
}

#[derive(Debug, Args)]
pub struct NvidiaFsArgs {
    /// Node selector: name/ip/label, or 'all'
    #[arg(short, long, default_value = "all")]
    pub selector: String,
}

#[derive(Debug, Args)]
pub struct OfedArgs {
    /// Node selector: name/ip/label, or 'all'
    #[arg(short, long, default_value = "all")]
    pub selector: String,
}

pub fn run_check_cmd(cli: &crate::Cli, cfg: &config::Config, cmd: &CheckCmd) -> anyhow::Result<()> {
    match cmd {
        CheckCmd::NvidiaDriver(args) => check_nvidia_driver(cli, cfg, args),
        CheckCmd::Cuda(args) => check_cuda(cli, cfg, args),
        CheckCmd::NvidiaFs(args) => check_nvidia_fs(cli, cfg, args),
        CheckCmd::Ofed(args) => check_ofed(cli, cfg, args),
        CheckCmd::ClientMount(args) => client::run_mount_tui(cli, cfg, args),
        CheckCmd::StorageTarget(args) => check_storage_target(cli, cfg, args),
    }
}

fn check_storage_target(cli: &crate::Cli, cfg: &config::Config, args: &StorageTargetArgs) -> anyhow::Result<()> {
    use regex::Regex;
    let timeout = args.timeout;
    let nodes = config::select_nodes(cfg, &args.selector);
    if nodes.len() != 1 {
        anyhow::bail!("selector must resolve to exactly one node (got {})", nodes.len());
    }
    let node = nodes[0];
    let tr = transport::from_config(cfg);

    // Check service
    let svc_cmd = "systemctl is-active beegfs-storage >/dev/null 2>&1 && echo active || echo inactive";
    let svc = tr.exec(&node.host, &format!("timeout {}s sh -lc {}", timeout, shell_escape::escape(svc_cmd.into())))?;
    let service_active = svc.stdout.trim().starts_with("active");

    // List targets and states
    let list_cmd = "beegfs-ctl --listtargets --state --storage 2>/dev/null || beegfs-ctl --listtargets --storage 2>/dev/null";
    let out = tr.exec(&node.host, &format!("timeout {}s sh -lc {}", timeout, shell_escape::escape(list_cmd.into())))?;
    let text = out.stdout;

    // Parse lines like: "   101 @ <hostname> (Good) ..." robustly: capture leading number and last word in parentheses
    let re = Regex::new(r"(?m)^\s*(\d+)\b.*?(?:\(([^)]+)\))?").unwrap();
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for cap in re.captures_iter(&text) {
        let id = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        if id.is_empty() { continue; }
        let state = cap.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
        found.insert(id, state);
    }

    // Desired target set
    let target_list: Vec<String> = if args.targets.eq_ignore_ascii_case("all") {
        found.keys().cloned().collect()
    } else {
        args.targets.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    };

    // Build result rows
    let mut rows = Vec::new();
    for tid in target_list {
        let present = found.get(&tid).is_some();
        let state = found.get(&tid).cloned().unwrap_or_else(|| "missing".to_string());
        rows.push((tid, present, state, service_active));
    }

    match cli.output {
        crate::Output::Human => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["TargetID", "Present", "State", "Service"]);
            for (tid, present, state, svc) in &rows {
                table.add_row(vec![
                    tid.as_str(),
                    if *present { "YES" } else { "NO" },
                    state.as_str(),
                    if *svc { "active" } else { "inactive" },
                ]);
            }
            println!("{}", table);

            // Warnings
            let missing: Vec<&str> = rows.iter().filter(|(_,p,_,_)| !*p).map(|(t,_,_,_)| t.as_str()).collect();
            if !missing.is_empty() { eprintln!("WARNING: missing targets: {}", missing.join(", ")); }
            let mut states: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for (tid, present, state, _) in &rows { if *present { states.entry(state.as_str()).or_default().push(tid.as_str()); } }
            if states.len() > 1 { eprintln!("WARNING: target state mismatch:"); for (st, ids) in states { eprintln!("  {}: {}", st, ids.join(", ")); } }
            if !service_active { eprintln!("WARNING: beegfs-storage service is inactive on {}", node.name); }
        }
        crate::Output::Json => {
            let arr: Vec<_> = rows.iter().map(|(tid, present, state, svc)| serde_json::json!({
                "target": tid,
                "present": present,
                "state": state,
                "service_active": svc,
            })).collect();
            println!("{}", serde_json::to_string_pretty(&arr)?);
            // Warnings to stderr
            let missing: Vec<&str> = rows.iter().filter(|(_,p,_,_)| !*p).map(|(t,_,_,_)| t.as_str()).collect();
            if !missing.is_empty() { eprintln!("WARNING: missing targets: {}", missing.join(", ")); }
            if !service_active { eprintln!("WARNING: beegfs-storage service is inactive on {}", node.name); }
        }
    }
    Ok(())
}

fn check_nvidia_driver(cli: &crate::Cli, cfg: &config::Config, args: &NvidiaArgs) -> anyhow::Result<()> {
    let tr = transport::from_config(cfg);
    let nodes = config::select_nodes(cfg, &args.selector);

    let query = "nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -n1 || modinfo -F version nvidia 2>/dev/null | head -n1 || echo unknown";

    let mut results = Vec::new();
    for n in nodes {
        let out = tr.exec(&n.host, query);
        let (version, ok, stderr) = match out {
            Ok(v) => {
                let v_str = v.stdout.trim();
                let ver = if v_str.is_empty() { "unknown" } else { v_str };
                (ver.to_string(), ver != "unknown", v.stderr)
            }
            Err(e) => ("error".into(), false, e.to_string()),
        };
        results.push((n.name.clone(), n.host.clone(), version, ok, stderr));
    }

    match cli.output {
        crate::Output::Human => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Node", "Host", "Driver", "Status"]);
            for (name, host, ver, ok, _stderr) in &results {
                let status = if *ok { "OK" } else { "MISSING" };
                table.add_row(vec![name.as_str(), host.as_str(), ver.as_str(), status]);
            }
            println!("{}", table);

            // Warnings: missing or mismatched versions
            warn_on_issues("NVIDIA driver", &results, &["unknown"]);
        }
        crate::Output::Json => {
            let arr: Vec<_> = results.iter().map(|(name, host, ver, ok, stderr)| serde_json::json!({
                "node": name,
                "host": host,
                "driver": ver,
                "ok": ok,
                "stderr": stderr,
            })).collect();
            println!("{}", serde_json::to_string_pretty(&arr)?);
            // Emit warnings to stderr to not break JSON consumers
            warn_on_issues("NVIDIA driver", &results, &["unknown"]);
        }
    }
    Ok(())
}

fn check_cuda(cli: &crate::Cli, cfg: &config::Config, args: &CudaArgs) -> anyhow::Result<()> {
    let tr = transport::from_config(cfg);
    let nodes = config::select_nodes(cfg, &args.selector);

    let query = "nvidia-smi --query-gpu=cuda_version --format=csv,noheader 2>/dev/null | head -n1 || nvcc --version 2>/dev/null | awk '/release/ {print $NF}' | sed 's/^V//' | head -n1 || awk '{print $3}' /usr/local/cuda/version.txt 2>/dev/null | head -n1 || echo unknown";

    let mut results = Vec::new();
    for n in nodes {
        let out = tr.exec(&n.host, query);
        let (version, ok, stderr) = match out {
            Ok(v) => {
                let v_str = v.stdout.trim();
                let ver = if v_str.is_empty() { "unknown" } else { v_str };
                (ver.to_string(), ver != "unknown", v.stderr)
            }
            Err(e) => ("error".into(), false, e.to_string()),
        };
        results.push((n.name.clone(), n.host.clone(), version, ok, stderr));
    }

    match cli.output {
        crate::Output::Human => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Node", "Host", "CUDA", "Status"]);
            for (name, host, ver, ok, _stderr) in &results {
                let status = if *ok { "OK" } else { "MISSING" };
                table.add_row(vec![name.as_str(), host.as_str(), ver.as_str(), status]);
            }
            println!("{}", table);

            warn_on_issues("CUDA", &results, &["unknown"]);
        }
        crate::Output::Json => {
            let arr: Vec<_> = results.iter().map(|(name, host, ver, ok, stderr)| serde_json::json!({
                "node": name,
                "host": host,
                "cuda": ver,
                "ok": ok,
                "stderr": stderr,
            })).collect();
            println!("{}", serde_json::to_string_pretty(&arr)?);
            // Warnings to stderr
            warn_on_issues("CUDA", &results, &["unknown"]);
        }
    }
    Ok(())
}

fn check_nvidia_fs(cli: &crate::Cli, cfg: &config::Config, args: &NvidiaFsArgs) -> anyhow::Result<()> {
    let tr = transport::from_config(cfg);
    let nodes = config::select_nodes(cfg, &args.selector);

    let query = "modinfo -F version nvidia_fs 2>/dev/null | head -n1 || modinfo -F version nvidia-fs 2>/dev/null | head -n1 || lsmod | awk '$1 ~ /^(nvidia_fs|nvidia-fs)$/ {print \"loaded\"}' | head -n1 || echo unknown";

    let mut results = Vec::new();
    for n in nodes {
        let out = tr.exec(&n.host, query);
        let (version, ok, stderr) = match out {
            Ok(v) => {
                let v_str = v.stdout.trim();
                let ver = if v_str.is_empty() { "unknown" } else { v_str };
                let ok = ver != "unknown" && ver != "" || v_str == "loaded";
                let ver_out = if v_str == "loaded" { "loaded".to_string() } else { ver.to_string() };
                (ver_out, ok, v.stderr)
            }
            Err(e) => ("error".into(), false, e.to_string()),
        };
        results.push((n.name.clone(), n.host.clone(), version, ok, stderr));
    }

    match cli.output {
        crate::Output::Human => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Node", "Host", "nvidia-fs", "Status"]);
            for (name, host, ver, ok, _stderr) in &results {
                let status = if *ok { "OK" } else { "MISSING" };
                table.add_row(vec![name.as_str(), host.as_str(), ver.as_str(), status]);
            }
            println!("{}", table);

            warn_on_issues("nvidia-fs", &results, &["unknown", "loaded"]);
        }
        crate::Output::Json => {
            let arr: Vec<_> = results.iter().map(|(name, host, ver, ok, stderr)| serde_json::json!({
                "node": name,
                "host": host,
                "nvidia_fs": ver,
                "ok": ok,
                "stderr": stderr,
            })).collect();
            println!("{}", serde_json::to_string_pretty(&arr)?);
            // Warnings to stderr
            warn_on_issues("nvidia-fs", &results, &["unknown", "loaded"]);
        }
    }
    Ok(())
}

fn check_ofed(cli: &crate::Cli, cfg: &config::Config, args: &OfedArgs) -> anyhow::Result<()> {
    let tr = transport::from_config(cfg);
    let nodes = config::select_nodes(cfg, &args.selector);

    let query = "ofed_info -s 2>/dev/null | head -n1 || modinfo -F version mlx5_core 2>/dev/null | head -n1 || modinfo -F version mlx5_ib 2>/dev/null | head -n1 || ibv_devinfo --version 2>/dev/null | head -n1 || echo unknown";

    let mut results = Vec::new();
    for n in nodes {
        let out = tr.exec(&n.host, query);
        let (version, ok, stderr) = match out {
            Ok(v) => {
                let v_str = v.stdout.trim();
                let ver = if v_str.is_empty() { "unknown" } else { v_str };
                (ver.to_string(), ver != "unknown", v.stderr)
            }
            Err(e) => ("error".into(), false, e.to_string()),
        };
        results.push((n.name.clone(), n.host.clone(), version, ok, stderr));
    }

    match cli.output {
        crate::Output::Human => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Node", "Host", "OFED/RDMA", "Status"]);
            for (name, host, ver, ok, _stderr) in &results {
                let status = if *ok { "OK" } else { "MISSING" };
                table.add_row(vec![name.as_str(), host.as_str(), ver.as_str(), status]);
            }
            println!("{}", table);

            warn_on_issues("OFED/RDMA", &results, &["unknown"]);
        }
        crate::Output::Json => {
            let arr: Vec<_> = results.iter().map(|(name, host, ver, ok, stderr)| serde_json::json!({
                "node": name,
                "host": host,
                "ofed": ver,
                "ok": ok,
                "stderr": stderr,
            })).collect();
            println!("{}", serde_json::to_string_pretty(&arr)?);
            // Warnings to stderr
            warn_on_issues("OFED/RDMA", &results, &["unknown"]);
        }
    }
    Ok(())
}

fn warn_on_issues(label: &str, results: &[(String, String, String, bool, String)], ignore_versions: &[&str]) {
    // Missing/not found
    let missing: Vec<&str> = results
        .iter()
        .filter(|(_, _, _, ok, _)| !*ok)
        .map(|(name, _, _, _, _)| name.as_str())
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "WARNING: {} missing on {} node(s): {}",
            label,
            missing.len(),
            missing.join(", ")
        );
    }

    // Version groups among OK nodes (excluding ignored versions)
    let mut versions: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, _host, ver, _ok, _stderr) in results.iter().filter(|(_,_,_,ok,_)| *ok) {
        if ignore_versions.iter().any(|ig| ig.eq_ignore_ascii_case(ver)) || ver.is_empty() {
            continue;
        }
        versions.entry(ver.as_str()).or_default().push(name.as_str());
    }
    if versions.len() > 1 {
        eprintln!("WARNING: {} version mismatch across nodes:", label);
        for (ver, nodes) in versions {
            eprintln!("  {}: {}", ver, nodes.join(", "));
        }
    }
}
