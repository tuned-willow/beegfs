use crate::{config, transport};
use crossterm::{terminal, execute, event::{self, Event, KeyEvent, KeyCode}};
use ratatui::{prelude::*, widgets::*};
use std::{io::stdout, time::{Duration, Instant}, sync::mpsc, thread};

// Uses super::ClientMountArgs from checks::mod

#[derive(Clone, Debug, Default)]
struct RowState {
    mount_defined: Option<String>,
    client_active: Option<String>,
    df: Option<String>,
    ls: Option<String>,
    rw: Option<String>,
}

#[derive(Clone, Debug)]
enum Update {
    Set { idx: usize, col: usize, val: String },
    Done,
}

pub fn run_mount_tui(_cli: &crate::Cli, cfg: &config::Config, args: &super::ClientMountArgs) -> anyhow::Result<()> {
    let nodes = config::select_nodes(cfg, &args.selector);
    let tr = transport::from_config(cfg);
    let timeout = args.timeout;
    let mount = args.mount.clone();

    // Channel for updates from worker threads
    let (tx, rx) = mpsc::channel::<Update>();

    // Spawn workers per node
    for (idx, n) in nodes.iter().enumerate() {
        let tx = tx.clone();
        let host = n.host.clone();
        let name = n.name.clone();
        let tr = transport::from_config(cfg);
        let mount = mount.clone();
        thread::spawn(move || {
            // 0: mount defined in config
            let cmd_mount_defined = format!(
                "grep -E '^[^#].*\\s+{}(\\s|$)' /etc/beegfs/beegfs-mounts.conf >/dev/null 2>&1 && echo OK || echo MISSING",
                shell_escape::escape(mount.clone().into())
            );
            let out = tr.exec(&host, &wrap_timeout(&cmd_mount_defined, timeout));
            let val = pick_ok(out);
            let _ = tx.send(Update::Set { idx, col: 0, val });

            // 1: client active
            let cmd_client = "systemctl is-active beegfs-client >/dev/null 2>&1 && systemctl is-active beegfs-helperd >/dev/null 2>&1 && echo OK || echo MISSING";
            let out = tr.exec(&host, &wrap_timeout(cmd_client, timeout));
            let val = pick_ok(out);
            let _ = tx.send(Update::Set { idx, col: 1, val });

            // 2: df -h mount
            let cmd_df = format!("df -h {} 2>&1 | tail -n +2 || true", shell_escape::escape(mount.clone().into()));
            let out = tr.exec(&host, &wrap_timeout(&cmd_df, timeout));
            let val = match out {
                Ok(o) => {
                    if o.stdout.trim().is_empty() { "ERR".to_string() } else { "OK".to_string() }
                }
                Err(e) => format!("ERR:{}", e),
            };
            let _ = tx.send(Update::Set { idx, col: 2, val });

            // 3: ls mount
            let cmd_ls = format!("ls -la {} >/dev/null 2>&1 && echo OK || echo ERR", shell_escape::escape(mount.clone().into()));
            let out = tr.exec(&host, &wrap_timeout(&cmd_ls, timeout));
            let val = pick_ok(out);
            let _ = tx.send(Update::Set { idx, col: 3, val });

            // 4: write+delete random file
            let rnd_name = format!(".beeg_check_{}", rand_suffix());
            let file_path = format!("{}/{}", mount, rnd_name);
            let cmd_rw = format!(
                "dd if=/dev/urandom of={} bs=4K count=1 status=none && rm -f {} && echo OK || echo ERR",
                shell_escape::escape(file_path.clone().into()),
                shell_escape::escape(file_path.into())
            );
            let out = tr.exec(&host, &wrap_timeout(&cmd_rw, timeout));
            let val = pick_ok(out);
            let _ = tx.send(Update::Set { idx, col: 4, val });

            let _ = tx.send(Update::Done);
        });
    }

    // TUI setup
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Model
    let mut rows: Vec<(&str, &str, RowState)> = nodes.iter().map(|n| (n.name.as_str(), n.host.as_str(), RowState::default())).collect();
    let total_done = nodes.len();
    let mut done_count = 0usize;

    // Event loop
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();
    'outer: loop {
        // Apply updates
        while let Ok(upd) = rx.try_recv() {
            match upd {
                Update::Set { idx, col, val } => {
                    if let Some((_, _, ref mut st)) = rows.get_mut(idx) {
                        match col {
                            0 => st.mount_defined = Some(val),
                            1 => st.client_active = Some(val),
                            2 => st.df = Some(val),
                            3 => st.ls = Some(val),
                            4 => st.rw = Some(val),
                            _ => {}
                        }
                    }
                }
                Update::Done => { done_count += 1; }
            }
        }

        // Draw UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(f.size());

            let title = Paragraph::new("beeg check client mount — press q to quit")
                .block(Block::default().borders(Borders::ALL).title("Client Mount"));
            f.render_widget(title, chunks[0]);

            let header = Row::new(vec!["Node", "Host", "Defined", "Client", "df -h", "ls", "rw"])
                .style(Style::default().add_modifier(Modifier::BOLD));
            let body_rows = rows.iter().map(|(name, host, st)| {
                Row::new(vec![
                    (*name).to_string(),
                    (*host).to_string(),
                    cell(&st.mount_defined),
                    cell(&st.client_active),
                    cell(&st.df),
                    cell(&st.ls),
                    cell(&st.rw),
                ])
            });
            let table = Table::new(body_rows, [
                    Constraint::Length(14),
                    Constraint::Length(18),
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(8),
                ])
                .header(header)
                .block(Block::default().borders(Borders::ALL).title(format!("Mount {}", args.mount)))
                ;
            f.render_widget(table, chunks[1]);

            let footer = Paragraph::new(format!("Completed: {}/{}", done_count, total_done))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        // Exit conditions: all done or user pressed q
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) = event::read()? {
                break 'outer;
            }
        }
        if last_tick.elapsed() >= tick_rate { last_tick = Instant::now(); }
        if done_count >= total_done { break 'outer; }
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    // Move out of alternate screen
    let mut out = std::io::stdout();
    execute!(out, crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

fn cell(v: &Option<String>) -> String {
    match v {
        Some(s) => s.clone(),
        None => "...".to_string(),
    }
}

fn wrap_timeout(cmd: &str, seconds: u64) -> String {
    // Use GNU coreutils timeout; if unavailable on remote, command may fail quickly
    format!("timeout {}s sh -lc {}", seconds, shell_escape::escape(cmd.into()))
}

fn rand_suffix() -> String {
    use rand::RngCore;
    let mut rng = rand::rngs::OsRng;
    let mut buf = [0u8; 4];
    rng.fill_bytes(&mut buf);
    hex::encode(buf)
}

fn pick_ok(res: anyhow::Result<transport::ExecOutput>) -> String {
    match res {
        Ok(o) => {
            let s = o.stdout.trim();
            if s.starts_with("OK") { "OK".into() } else { "ERR".into() }
        }
        Err(_) => "ERR".into(),
    }
}
