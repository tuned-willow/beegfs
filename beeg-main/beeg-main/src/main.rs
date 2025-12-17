use clap::{Args, Parser, Subcommand, ValueEnum, CommandFactory};
use clap_complete::{generate_to, Shell};
use std::path::PathBuf;
use std::fs;

mod config;
mod transport;
mod checks;

#[derive(Debug, Parser)]
#[command(name = "beeg", version, about = "BeegFS CLI assistant", long_about = None)]
struct Cli {
    /// Increase output verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output format
    #[arg(long, value_enum, default_value_t = Output::Human)]
    output: Output,

    /// Config file to use (for node inventory, auth, etc.)
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Output {
    Human,
    Json,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Show a summarized status view (cluster or node)
    Status(StatusArgs),

    /// Node-oriented actions
    #[command(subcommand)]
    Node(NodeCmd),

    /// Read or write configuration values
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Generate shell completion files
    Completions(CompletionsArgs),

    /// Cluster checks
    #[command(subcommand)]
    Check(checks::CheckCmd),
}

#[derive(Debug, Args)]
struct StatusArgs {
    /// Optional node selector (name, ip, label)
    #[arg(short, long)]
    selector: Option<String>,
}

#[derive(Debug, Args)]
struct CompletionsArgs {
    /// Shell to generate completions for (default: all)
    #[arg(long, value_enum)]
    shell: Option<CompShell>,
    /// Output directory to write completion files
    #[arg(long)]
    dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CompShell { Bash, Zsh, Fish, PowerShell, Elvish }

#[derive(Debug, Subcommand)]
enum NodeCmd {
    /// List known nodes
    List,
    /// Execute a read-only command on nodes
    Exec(ExecArgs),
}

#[derive(Debug, Args)]
struct ExecArgs {
    /// Node selector: name/ip/label, or 'all'
    #[arg(short, long, default_value = "all")]
    selector: String,
    /// Command to run (read-only diagnostics)
    #[arg(last = true, required = true)]
    cmd: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    /// Get a config value by key
    Get(ConfigGetArgs),
    /// Set a config value by key
    Set(ConfigSetArgs),
}

#[derive(Debug, Args)]
struct ConfigGetArgs {
    /// Config key to read
    #[arg(long)]
    key: String,
    /// Optional node selector
    #[arg(short, long)]
    selector: Option<String>,
}

#[derive(Debug, Args)]
struct ConfigSetArgs {
    /// Config key to write
    #[arg(long)]
    key: String,
    /// New value
    #[arg(long)]
    value: String,
    /// Optional node selector
    #[arg(short, long)]
    selector: Option<String>,
    /// Require an explicit confirmation for write operations
    #[arg(long, default_value_t = true)]
    confirm: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load configuration once; many commands need it
    let cfg = config::load(cli.config.as_ref())?;

    match &cli.command {
        Commands::Status(args) => cmd_status(&cli, args)?,
        Commands::Node(cmd) => match cmd {
            NodeCmd::List => cmd_node_list(&cli, &cfg)?,
            NodeCmd::Exec(args) => cmd_node_exec(&cli, &cfg, args)?,
        },
        Commands::Config(cmd) => match cmd {
            ConfigCmd::Get(args) => cmd_config_get(&cli, args)?,
            ConfigCmd::Set(args) => cmd_config_set(&cli, args)?,
        },
        Commands::Completions(args) => cmd_completions(args)?,
        Commands::Check(cmd) => checks::run_check_cmd(&cli, &cfg, &cmd)?,
    }

    Ok(())
}

fn cmd_status(cli: &Cli, args: &StatusArgs) -> anyhow::Result<()> {
    match cli.output {
        Output::Human => {
            println!(
                "Status: selector={:?} (prototype; implement remote queries)",
                args.selector
            );
        }
        Output::Json => {
            let obj = serde_json::json!({
                "status": "ok",
                "selector": args.selector,
                "prototype": true,
            });
            println!("{}", serde_json::to_string_pretty(&obj)?);
        }
    }
    Ok(())
}

fn cmd_node_list(cli: &Cli, cfg: &config::Config) -> anyhow::Result<()> {
    let nodes: Vec<_> = cfg.nodes.iter().map(|n| &n.name).collect();
    match cli.output {
        Output::Human => {
            println!("Known nodes (prototype):");
            for n in nodes {
                println!("- {}", n);
            }
        }
        Output::Json => {
            println!("{}", serde_json::to_string_pretty(&nodes)?);
        }
    }
    Ok(())
}

fn cmd_node_exec(cli: &Cli, cfg: &config::Config, args: &ExecArgs) -> anyhow::Result<()> {
    let selector = &args.selector;
    let cmdline = args.cmd.join(" ");
    let targets = config::select_nodes(cfg, selector);
    match cli.output {
        Output::Human => {
            println!(
                "Exec (prototype): selector='{}' cmd='{}' on {} node(s)",
                selector, cmdline, targets.len()
            );
            let tr = transport::from_config(cfg);
            for n in targets {
                match tr.exec(&n.host, &cmdline) {
                    Ok(out) => {
                        println!("=== {} ===\n{}", n.name, out.stdout);
                        if !out.stderr.trim().is_empty() {
                            eprintln!("--- {} (stderr) ---\n{}", n.name, out.stderr);
                        }
                    }
                    Err(e) => eprintln!("!!! {} error: {}", n.name, e),
                }
            }
        }
        Output::Json => {
            let tr = transport::from_config(cfg);
            let mut results = Vec::new();
            for n in targets {
                let res = match tr.exec(&n.host, &cmdline) {
                    Ok(out) => serde_json::json!({
                        "node": n.name,
                        "ok": true,
                        "stdout": out.stdout,
                        "stderr": out.stderr,
                    }),
                    Err(e) => serde_json::json!({
                        "node": n.name,
                        "ok": false,
                        "error": e.to_string(),
                    }),
                };
                results.push(res);
            }
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
    }
    Ok(())
}

fn cmd_config_get(cli: &Cli, args: &ConfigGetArgs) -> anyhow::Result<()> {
    let value = serde_json::json!({"key": args.key, "value": "<value>", "proto": true});
    match cli.output {
        Output::Human => println!("{} = <value> (prototype)", args.key),
        Output::Json => println!("{}", serde_json::to_string_pretty(&value)?),
    }
    Ok(())
}

fn cmd_config_set(cli: &Cli, args: &ConfigSetArgs) -> anyhow::Result<()> {
    if args.confirm {
        // In a future version, prompt y/N. For now, print a warning.
        eprintln!("WARNING: write operations are not implemented; dry-run only.");
    }

    match cli.output {
        Output::Human => println!(
            "Set (prototype): key='{}' value='{}' selector={:?}",
            args.key, args.value, args.selector
        ),
        Output::Json => {
            let obj = serde_json::json!({
                "action": "set",
                "key": args.key,
                "value": args.value,
                "selector": args.selector,
                "prototype": true,
            });
            println!("{}", serde_json::to_string_pretty(&obj)?);
        }
    }
    Ok(())
}

fn cmd_completions(args: &CompletionsArgs) -> anyhow::Result<()> {
    let mut wrote = Vec::new();
    let outdir = if let Some(d) = &args.dir { d.clone() } else { std::env::current_dir()? };
    fs::create_dir_all(&outdir)?;

    let shells: Vec<Shell> = match args.shell {
        Some(CompShell::Bash) => vec![Shell::Bash],
        Some(CompShell::Zsh) => vec![Shell::Zsh],
        Some(CompShell::Fish) => vec![Shell::Fish],
        Some(CompShell::PowerShell) => vec![Shell::PowerShell],
        Some(CompShell::Elvish) => vec![Shell::Elvish],
        None => vec![Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell, Shell::Elvish],
    };

    let mut cmd = Cli::command();
    for sh in shells {
        let path = generate_to(sh, &mut cmd, "beeg", &outdir)?;
        wrote.push(path);
    }
    for p in wrote { println!("wrote completion: {}", p.display()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_json() {
        let cli = Cli::parse_from(["beeg", "--output", "json", "status"]);
        match cli.command { Commands::Status(_) => {}, _ => panic!("expected status"), }
        assert!(matches!(cli.output, Output::Json));
    }

    #[test]
    fn parse_check_nvidia() {
        let cli = Cli::parse_from(["beeg", "check", "nvidia-driver", "-s", "all"]);
        match cli.command { Commands::Check(_) => {}, _ => panic!("expected check"), }
    }

    #[test]
    fn parse_check_cuda() {
        let cli = Cli::parse_from(["beeg", "check", "cuda", "-s", "gpu"]);
        match cli.command { Commands::Check(_) => {}, _ => panic!("expected check cuda"), }
    }

    #[test]
    fn parse_check_nvidia_fs() {
        let cli = Cli::parse_from(["beeg", "check", "nvidia-fs", "-s", "all"]);
        match cli.command { Commands::Check(_) => {}, _ => panic!("expected check nvidia-fs"), }
    }

    #[test]
    fn parse_check_ofed() {
        let cli = Cli::parse_from(["beeg", "check", "ofed", "-s", "all"]);
        match cli.command { Commands::Check(_) => {}, _ => panic!("expected check ofed"), }
    }

    #[test]
    fn parse_check_client_mount() {
        let cli = Cli::parse_from(["beeg", "check", "client-mount", "--mount", "/mnt/beegfs", "-s", "all"]);
        match cli.command { Commands::Check(_) => {}, _ => panic!("expected check client mount"), }
    }

    #[test]
    fn parse_check_storage_target() {
        let cli = Cli::parse_from(["beeg", "check", "storage-target", "--selector", "node-a", "--targets", "all"]);
        match cli.command { Commands::Check(_) => {}, _ => panic!("expected check storage-target"), }
    }

    #[test]
    fn parse_node_exec() {
        let cli = Cli::parse_from(["beeg", "node", "exec", "--", "echo", "hi"]);
        match cli.command { Commands::Node(NodeCmd::Exec(_)) => {}, _ => panic!("expected node exec"), }
    }
}
