use std::{path::Path, process::Command};

use log::{info, warn};

use crate::message::{Content, Message};
use crate::{BabataResult, agent::AgentLoop, config::Config, error::BabataError, job::JobManager};

use super::Args;

const MACOS_LAUNCHD_LABEL: &str = "babata.server";
const LINUX_SYSTEMD_SERVICE: &str = "babata.server.service";
const WINDOWS_TASK_NAME: &str = "babata.server";

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

pub fn stop(_args: &Args) {
    if let Err(err) = run_stop() {
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
    let agent_loop = AgentLoop::new(config.clone())?;
    let job_manager = JobManager::new(config, agent_loop.channels.clone())?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    runtime.block_on(async move {
        let has_job_scheduler = job_manager.start_scheduler().await?;
        if agent_loop.channels.is_empty() && !has_job_scheduler {
            return Err(BabataError::config(
                "No channels or enabled jobs configured; cannot start server",
            ));
        }
        broadcast_service_started(&agent_loop.channels).await;
        agent_loop.run().await
    })?;
    Ok(())
}

async fn broadcast_service_started(channels: &[std::sync::Arc<dyn crate::channel::Channel>]) {
    if channels.is_empty() {
        return;
    }

    let message = service_started_message();
    for channel in channels {
        if let Err(err) = channel.send(std::slice::from_ref(&message)).await {
            warn!(
                "Server started but failed to send startup message to channel: {}",
                err
            );
        }
    }
}

fn service_started_message() -> Message {
    let text = "Babata server started. This is a startup notification.".to_string();

    info!("{text}");

    Message::AssistantResponse {
        content: vec![Content::Text { text }],
        reasoning_content: None,
    }
}

fn run_start() -> BabataResult<()> {
    match std::env::consts::OS {
        "macos" => start_macos(),
        "linux" => start_linux(),
        "windows" => start_windows(),
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
        "windows" => stop_windows(),
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
        "windows" => restart_windows(),
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

fn start_windows() -> BabataResult<()> {
    let script_path = windows_task_script_path()?;
    ensure_file_exists(
        &script_path,
        "Windows task script not found; run \"babata onboard\" first",
    )?;
    let task_action = windows_task_action(&script_path);

    run_command(
        "schtasks",
        &[
            "/Create",
            "/TN",
            WINDOWS_TASK_NAME,
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/F",
            "/TR",
            &task_action,
        ],
    )?;
    run_command("schtasks", &["/Run", "/TN", WINDOWS_TASK_NAME])?;
    println!(
        "Started server with Windows Task Scheduler: {}",
        WINDOWS_TASK_NAME
    );
    Ok(())
}

fn restart_windows() -> BabataResult<()> {
    let script_path = windows_task_script_path()?;
    ensure_file_exists(
        &script_path,
        "Windows task script not found; run \"babata onboard\" first",
    )?;
    let task_action = windows_task_action(&script_path);

    run_command(
        "schtasks",
        &[
            "/Create",
            "/TN",
            WINDOWS_TASK_NAME,
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/F",
            "/TR",
            &task_action,
        ],
    )?;
    let _ = run_command("schtasks", &["/End", "/TN", WINDOWS_TASK_NAME]);
    run_command("schtasks", &["/Run", "/TN", WINDOWS_TASK_NAME])?;
    println!(
        "Restarted server with Windows Task Scheduler: {}",
        WINDOWS_TASK_NAME
    );
    Ok(())
}

fn stop_windows() -> BabataResult<()> {
    let script_path = windows_task_script_path()?;
    ensure_file_exists(
        &script_path,
        "Windows task script not found; run \"babata onboard\" first",
    )?;

    run_command("schtasks", &["/Query", "/TN", WINDOWS_TASK_NAME])?;
    let _ = run_command("schtasks", &["/End", "/TN", WINDOWS_TASK_NAME]);
    println!(
        "Stopped server with Windows Task Scheduler: {}",
        WINDOWS_TASK_NAME
    );
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

fn windows_task_script_path() -> BabataResult<std::path::PathBuf> {
    Ok(crate::utils::babata_dir()?
        .join("services")
        .join("babata.server.ps1"))
}

fn windows_task_action(script_path: &Path) -> String {
    format!(
        "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File \"{}\"",
        script_path.to_string_lossy()
    )
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
    use crate::message::{Content, Message};

    use super::{is_macos_service_not_found_error, service_started_message, windows_task_action};

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

    #[test]
    fn service_started_message_without_scheduler_details() {
        let message = service_started_message();
        let Message::AssistantResponse { content, .. } = message else {
            panic!("expected assistant response");
        };
        let text = content
            .into_iter()
            .find_map(|part| match part {
                Content::Text { text } => Some(text),
                _ => None,
            })
            .expect("text content");
        assert!(text.contains("Babata server started."));
        assert!(!text.contains("Job scheduler"));
    }

    #[test]
    fn builds_windows_task_action_with_quoted_script_path() {
        let action = windows_task_action(std::path::Path::new(
            r"C:\Users\alice\.babata\services\babata.server.ps1",
        ));
        assert_eq!(
            action,
            "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File \"C:\\Users\\alice\\.babata\\services\\babata.server.ps1\""
        );
    }
}
