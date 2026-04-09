use std::{path::Path, process::Command, sync::Arc};

use log::{error, info};

use crate::{
    BabataResult, agent::load_agents, config::Config, error::BabataError, http::HttpApp,
    task::TaskStore,
};
use crate::{
    channel::{build_channels, start_channel_loops},
    message::Content,
};
use crate::{
    task::{CreateTaskRequest, TaskLauncher, TaskManager},
    utils::babata_dir,
};

const MACOS_LAUNCHD_LABEL: &str = "babata.server";
const LINUX_SYSTEMD_SERVICE: &str = "babata.server.service";

pub fn serve() {
    if let Err(err) = run_serve() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn start() {
    if let Err(err) = run_start() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn stop() {
    if let Err(err) = run_stop() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn restart() {
    if let Err(err) = run_restart() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn start_background_service() -> BabataResult<()> {
    run_start()
}

fn run_serve() -> BabataResult<()> {
    info!("Server run babata dir: {}", babata_dir()?.display());

    let config = Config::load()?;
    let channels = build_channels(&config)?;
    let task_store = TaskStore::new()?;
    let task_launcher = TaskLauncher::new(load_agents()?, channels.clone())?;
    let task_manager = Arc::new(TaskManager::new(task_store, task_launcher)?);

    let http_app = HttpApp::new(task_manager.clone());

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| BabataError::internal(format!("Failed to build Tokio runtime: {err}")))?;

    runtime.block_on(async move {
        task_manager.start()?;
        start_channel_loops(channels, task_manager.clone());

        broadcast_service_started(&task_manager).await;

        http_app.serve().await
    })?;

    Ok(())
}

async fn broadcast_service_started(task_manager: &Arc<TaskManager>) {
    let babata_home = match crate::utils::babata_dir() {
        Ok(path) => path.display().to_string(),
        Err(err) => format!("unavailable ({err})"),
    };
    let notification = format!(
        "Babata server started.\nVersion: {}\nBabata home: {}",
        env!("CARGO_PKG_VERSION"),
        babata_home
    );

    let prompt = Content::Text {
        text: format!("Send below notification to each channel: \n{notification}"),
    };

    let task = CreateTaskRequest {
        description: "broadcast service started notification".to_string(),
        prompt: vec![prompt],
        parent_task_id: None,
        agent: None,
        never_ends: false,
    };
    if let Err(e) = task_manager.create_task(task) {
        error!("Failed to create service started notification task: {}", e);
    }
}

fn run_start() -> BabataResult<()> {
    match std::env::consts::OS {
        "macos" => start_macos(),
        "linux" => start_linux(),
        "windows" => Err(BabataError::config(
            "Windows service is not supported. Use 'babata server serve' to run in foreground.",
        )),
        os => Err(BabataError::config(format!(
            "Server start is not supported on '{}'",
            os
        ))),
    }
}

fn run_stop() -> BabataResult<()> {
    match std::env::consts::OS {
        "macos" => stop_macos(),
        "linux" => stop_linux(),
        "windows" => Err(BabataError::config(
            "Windows service is not supported. Use 'babata server serve' to run in foreground.",
        )),
        os => Err(BabataError::config(format!(
            "Server stop is not supported on '{}'",
            os
        ))),
    }
}

fn run_restart() -> BabataResult<()> {
    match std::env::consts::OS {
        "macos" => restart_macos(),
        "linux" => restart_linux(),
        "windows" => Err(BabataError::config(
            "Windows service is not supported. Use 'babata server serve' to run in foreground.",
        )),
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
    if let Err(err) = run_command("launchctl", &["kickstart", "-k", &service]) {
        if is_macos_service_not_found_error(&err.to_string()) {
            // Service has not been loaded yet; fall back to start semantics.
            return start_macos();
        }
        return Err(err);
    }
    println!("Restarted server with launchd: {}", MACOS_LAUNCHD_LABEL);
    Ok(())
}

fn stop_macos() -> BabataResult<()> {
    let uid = current_uid()?;
    let service = format!("gui/{uid}/{MACOS_LAUNCHD_LABEL}");

    // Try stopping by service label first.
    if run_command("launchctl", &["bootout", &service]).is_ok() {
        println!("Stopped server with launchd: {}", MACOS_LAUNCHD_LABEL);
        return Ok(());
    }

    // Fallback to plist-based unload for older launchctl flows.
    let plist_path = macos_plist_path()?;
    let plist_path = plist_path.to_string_lossy().to_string();
    run_command("launchctl", &["unload", &plist_path])?;
    println!("Stopped server with launchd: {}", MACOS_LAUNCHD_LABEL);
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

fn stop_linux() -> BabataResult<()> {
    run_command("systemctl", &["--user", "stop", LINUX_SYSTEMD_SERVICE])?;
    println!("Stopped server with systemd: {}", LINUX_SYSTEMD_SERVICE);
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

fn is_macos_service_not_found_error(message: &str) -> bool {
    message.contains("Could not find service")
        || message.contains("service not found")
        || message.contains("No such process")
}

#[cfg(test)]
mod tests {
    use super::is_macos_service_not_found_error;

    #[test]
    fn detects_launchctl_missing_service_error() {
        let message = "Internal error: Command 'launchctl kickstart -k gui/501/babata.server' failed with status exit status: 113: Could not find service \"babata.server\" in domain for user gui: 501";
        assert!(is_macos_service_not_found_error(message));
    }

    #[test]
    fn does_not_misclassify_unrelated_launchctl_error() {
        let message = "Internal error: Command 'launchctl kickstart -k gui/501/babata.server' failed: permission denied";
        assert!(!is_macos_service_not_found_error(message));
    }
}
