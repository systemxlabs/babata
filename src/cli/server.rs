use std::{path::Path, process::Command};

use crate::{BabataResult, agent::AgentLoop, config::Config, error::BabataError};

use super::Args;

const MACOS_LAUNCHD_LABEL: &str = "babata.server";
const LINUX_SYSTEMD_SERVICE: &str = "babata.server.service";

pub fn serve(args: &Args) {
    if let Err(err) = run_serve(args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn start(_args: &Args) {
    if let Err(err) = run_start() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn restart(_args: &Args) {
    if let Err(err) = run_restart() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_serve(_args: &Args) -> BabataResult<()> {
    let config = Config::load()?;
    let agent_loop = AgentLoop::new(config)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    runtime.block_on(agent_loop.run())?;
    Ok(())
}

fn run_start() -> BabataResult<()> {
    match std::env::consts::OS {
        "macos" => start_macos(),
        "linux" => start_linux(),
        os => Err(BabataError::config(format!(
            "Server start is not supported on '{}'",
            os
        ))),
    }
}

fn run_restart() -> BabataResult<()> {
    match std::env::consts::OS {
        "macos" => restart_macos(),
        "linux" => restart_linux(),
        os => Err(BabataError::config(format!(
            "Server restart is not supported on '{}'",
            os
        ))),
    }
}

fn start_macos() -> BabataResult<()> {
    let plist_path = macos_plist_path()?;
    ensure_file_exists(
        &plist_path,
        "launchd plist not found; run \"babata onboard\" first",
    )?;
    let plist_path = plist_path.to_string_lossy().to_string();

    // Best effort unload to support repeated starts.
    let _ = run_command("launchctl", &["unload", &plist_path]);
    run_command("launchctl", &["load", &plist_path])?;
    println!("Started server with launchd: {}", MACOS_LAUNCHD_LABEL);
    Ok(())
}

fn restart_macos() -> BabataResult<()> {
    let uid = current_uid()?;
    let service = format!("gui/{uid}/{MACOS_LAUNCHD_LABEL}");
    run_command("launchctl", &["kickstart", "-k", &service])?;
    println!("Restarted server with launchd: {}", MACOS_LAUNCHD_LABEL);
    Ok(())
}

fn start_linux() -> BabataResult<()> {
    let service_path = linux_systemd_service_path()?;
    ensure_file_exists(
        &service_path,
        "systemd service file not found; run \"babata onboard\" first",
    )?;
    let service_path = service_path.to_string_lossy().to_string();

    run_command("systemctl", &["--user", "daemon-reload"])?;
    // Best effort link to support repeated starts.
    let _ = run_command("systemctl", &["--user", "link", &service_path]);
    run_command(
        "systemctl",
        &["--user", "enable", "--now", LINUX_SYSTEMD_SERVICE],
    )?;
    println!("Started server with systemd: {}", LINUX_SYSTEMD_SERVICE);
    Ok(())
}

fn restart_linux() -> BabataResult<()> {
    run_command("systemctl", &["--user", "restart", LINUX_SYSTEMD_SERVICE])?;
    println!("Restarted server with systemd: {}", LINUX_SYSTEMD_SERVICE);
    Ok(())
}

fn macos_plist_path() -> BabataResult<std::path::PathBuf> {
    Ok(crate::utils::resolve_home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join("babata.server.plist"))
}

fn linux_systemd_service_path() -> BabataResult<std::path::PathBuf> {
    Ok(crate::utils::babata_dir()?
        .join("services")
        .join(LINUX_SYSTEMD_SERVICE))
}

fn ensure_file_exists(path: &Path, message: &str) -> BabataResult<()> {
    if path.exists() {
        return Ok(());
    }

    Err(BabataError::config(format!(
        "{}: '{}'",
        message,
        path.display()
    )))
}

fn current_uid() -> BabataResult<String> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|err| BabataError::internal(format!("Failed to run 'id -u': {err}")))?;
    if !output.status.success() {
        return Err(BabataError::internal(format!(
            "'id -u' failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid.is_empty() {
        return Err(BabataError::internal("'id -u' returned empty uid"));
    }

    Ok(uid)
}

fn run_command(cmd: &str, args: &[&str]) -> BabataResult<()> {
    let output = Command::new(cmd).args(args).output().map_err(|err| {
        BabataError::internal(format!(
            "Failed to execute command '{} {}': {}",
            cmd,
            args.join(" "),
            err
        ))
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if stderr.is_empty() { stdout } else { stderr };

    Err(BabataError::internal(format!(
        "Command '{} {}' failed with status {}: {}",
        cmd,
        args.join(" "),
        output.status,
        details
    )))
}
