use std::{path::Path, process::Command};

use log::{info, warn};

use crate::job::start_job_scheduler;
use crate::message::{Content, Message};
use crate::{BabataResult, agent::AgentLoop, config::Config, error::BabataError};

use super::Args;

const MACOS_LAUNCHD_LABEL: &str = "babata.server";
const LINUX_SYSTEMD_SERVICE: &str = "babata.server.service";
const WINDOWS_SERVICE_NAME: &str = "babata.server";
const WINDOWS_SERVICE_DISPLAY_NAME: &str = "Babata Server";
const WINDOWS_SERVICE_DESCRIPTION: &str =
    "Babata background server managed by Windows Service Control Manager.";

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

pub fn windows_service_host(_args: &Args, home_dir: Option<&str>) {
    if let Err(err) = run_windows_service_host(home_dir) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn start_background_service() -> BabataResult<()> {
    run_start()
}

pub fn install_windows_service() -> BabataResult<()> {
    if std::env::consts::OS != "windows" {
        return Err(BabataError::config(
            "Windows service installation is only supported on Windows",
        ));
    }

    let exe_path = std::env::current_exe().map_err(|err| {
        BabataError::internal(format!("Failed to resolve current executable path: {err}"))
    })?;
    let home_dir = crate::utils::resolve_home_dir()?;
    let bin_path = windows_service_bin_path(&exe_path, &home_dir);

    if let Err(err) = create_or_update_windows_service(&bin_path) {
        if is_windows_service_permission_denied_message(&err.to_string()) {
            return Err(BabataError::config(
                "Installing Windows service requires Administrator privileges. Re-run in an elevated shell, e.g. \"babata onboard\" or \"babata server start\" as Administrator.",
            ));
        }
        return Err(err);
    }
    configure_windows_service_metadata();
    Ok(())
}

fn run_serve(_args: &Args) -> BabataResult<()> {
    let config = Config::load()?;
    let agent_loop = AgentLoop::new(config.clone())?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    runtime.block_on(async move {
        start_job_scheduler();
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
    install_windows_service()?;
    start_windows_service()?;

    println!(
        "Started server with Windows Service: {}",
        WINDOWS_SERVICE_NAME
    );
    Ok(())
}

fn restart_windows() -> BabataResult<()> {
    install_windows_service()?;

    let _ = stop_windows_service();
    start_windows_service()?;

    println!(
        "Restarted server with Windows Service: {}",
        WINDOWS_SERVICE_NAME
    );
    Ok(())
}

fn stop_windows() -> BabataResult<()> {
    stop_windows_service()?;
    stop_windows_running_processes()?;
    println!("Stopped server on Windows");
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

fn windows_service_bin_path(exe_path: &Path, home_dir: &Path) -> String {
    format!(
        "\"{}\" server windows-service-host --home-dir \"{}\"",
        exe_path.to_string_lossy(),
        home_dir.to_string_lossy()
    )
}

fn create_or_update_windows_service(bin_path: &str) -> BabataResult<()> {
    let create_result = run_command(
        "sc",
        &[
            "create",
            WINDOWS_SERVICE_NAME,
            "type=",
            "own",
            "start=",
            "auto",
            "binPath=",
            bin_path,
            "displayname=",
            WINDOWS_SERVICE_DISPLAY_NAME,
        ],
    );

    if create_result.is_ok() {
        return Ok(());
    }

    let create_err = create_result.err().expect("create_result checked is_err");
    let err_text = create_err.to_string();
    if !is_windows_service_already_exists_error(&err_text) {
        return Err(create_err);
    }

    run_command(
        "sc",
        &[
            "config",
            WINDOWS_SERVICE_NAME,
            "type=",
            "own",
            "start=",
            "auto",
            "binPath=",
            bin_path,
            "displayname=",
            WINDOWS_SERVICE_DISPLAY_NAME,
        ],
    )
}

fn configure_windows_service_metadata() {
    if let Err(err) = run_command(
        "sc",
        &[
            "description",
            WINDOWS_SERVICE_NAME,
            WINDOWS_SERVICE_DESCRIPTION,
        ],
    ) {
        warn!("Failed to set Windows service description: {}", err);
    }

    if let Err(err) = run_command(
        "sc",
        &[
            "failure",
            WINDOWS_SERVICE_NAME,
            "reset=",
            "86400",
            "actions=",
            "restart/5000/restart/5000/restart/5000",
        ],
    ) {
        warn!("Failed to set Windows service recovery actions: {}", err);
    }
}

fn start_windows_service() -> BabataResult<()> {
    if let Err(err) = run_command("sc", &["start", WINDOWS_SERVICE_NAME]) {
        if !is_windows_service_already_running_error(&err.to_string()) {
            return Err(err);
        }
    }
    Ok(())
}

fn stop_windows_service() -> BabataResult<()> {
    if let Err(err) = run_command("sc", &["stop", WINDOWS_SERVICE_NAME]) {
        let err_text = err.to_string();
        if !is_windows_service_not_running_error(&err_text)
            && !is_windows_service_not_found_error(&err_text)
        {
            return Err(err);
        }
    }
    Ok(())
}

fn stop_windows_running_processes() -> BabataResult<()> {
    run_command(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "$procs = Get-CimInstance Win32_Process | Where-Object { $_.Name -ieq 'babata.exe' -and $_.CommandLine -match '(?i)\\bserver\\s+serve\\b' }; foreach ($proc in $procs) { Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue }",
        ],
    )
}

#[cfg(windows)]
fn run_windows_service_host(home_dir: Option<&str>) -> BabataResult<()> {
    windows_service_host::run(home_dir)
}

#[cfg(not(windows))]
fn run_windows_service_host(_home_dir: Option<&str>) -> BabataResult<()> {
    Err(BabataError::config(
        "Windows service host can only run on Windows",
    ))
}

fn is_windows_service_already_exists_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("1073") || lower.contains("already exists")
}

fn is_windows_service_already_running_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("1056") || lower.contains("already running")
}

fn is_windows_service_not_running_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("1062") || lower.contains("not been started")
}

fn is_windows_service_not_found_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("1060") || lower.contains("does not exist")
}

pub fn is_windows_service_permission_denied_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let is_service_cmd = lower.contains("command 'sc create")
        || lower.contains("command 'sc config")
        || lower.contains("openscmanager")
        || lower.contains("openservice");
    let is_access_denied = lower.contains("status exit code: 5")
        || lower.contains("access is denied")
        || lower.contains("failed 5");
    is_service_cmd && is_access_denied
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

#[cfg(windows)]
mod windows_service_host {
    use std::{
        ffi::OsString,
        path::PathBuf,
        process::{Child, Command},
        sync::{OnceLock, mpsc},
        time::Duration,
    };

    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
        service_dispatcher,
    };

    use crate::{BabataResult, error::BabataError};

    static SERVICE_HOME_DIR: OnceLock<String> = OnceLock::new();
    static SERVICE_EXE_PATH: OnceLock<PathBuf> = OnceLock::new();

    define_windows_service!(ffi_service_main, service_main);

    pub fn run(home_dir: Option<&str>) -> BabataResult<()> {
        let resolved_home_dir = match home_dir {
            Some(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => crate::utils::resolve_home_dir()?
                .to_string_lossy()
                .into_owned(),
        };
        let _ = SERVICE_HOME_DIR.set(resolved_home_dir);

        let exe_path = std::env::current_exe().map_err(|err| {
            BabataError::internal(format!("Failed to resolve current executable path: {err}"))
        })?;
        let _ = SERVICE_EXE_PATH.set(exe_path);

        service_dispatcher::start(super::WINDOWS_SERVICE_NAME, ffi_service_main).map_err(|err| {
            BabataError::internal(format!(
                "Failed to start Windows service dispatcher: {}",
                err
            ))
        })
    }

    fn service_main(_arguments: Vec<OsString>) {
        if let Err(err) = run_service_main() {
            eprintln!("{err}");
        }
    }

    fn run_service_main() -> BabataResult<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

        let status_handle =
            service_control_handler::register(super::WINDOWS_SERVICE_NAME, move |control| {
                match control {
                    ServiceControl::Stop | ServiceControl::Shutdown => {
                        let _ = shutdown_tx.send(());
                        ServiceControlHandlerResult::NoError
                    }
                    ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                    _ => ServiceControlHandlerResult::NotImplemented,
                }
            })
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to register Windows service control handler: {}",
                    err
                ))
            })?;

        set_service_status(
            &status_handle,
            ServiceState::StartPending,
            ServiceControlAccept::empty(),
            ServiceExitCode::Win32(0),
            5,
        )?;

        let mut child = match spawn_server_child() {
            Ok(child) => child,
            Err(err) => {
                let _ = set_service_status(
                    &status_handle,
                    ServiceState::Stopped,
                    ServiceControlAccept::empty(),
                    ServiceExitCode::Win32(1),
                    0,
                );
                return Err(err);
            }
        };

        set_service_status(
            &status_handle,
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            ServiceExitCode::Win32(0),
            0,
        )?;

        loop {
            if shutdown_rx.recv_timeout(Duration::from_secs(1)).is_ok() {
                break;
            }

            if let Some(exit_status) = child.try_wait().map_err(|err| {
                BabataError::internal(format!("Failed to monitor child process: {}", err))
            })? {
                let _ = set_service_status(
                    &status_handle,
                    ServiceState::Stopped,
                    ServiceControlAccept::empty(),
                    ServiceExitCode::Win32(1),
                    0,
                );
                return Err(BabataError::internal(format!(
                    "Babata server child process exited unexpectedly: {}",
                    exit_status
                )));
            }
        }

        set_service_status(
            &status_handle,
            ServiceState::StopPending,
            ServiceControlAccept::empty(),
            ServiceExitCode::Win32(0),
            10,
        )?;
        terminate_child(&mut child)?;
        set_service_status(
            &status_handle,
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            ServiceExitCode::Win32(0),
            0,
        )?;
        Ok(())
    }

    fn set_service_status(
        status_handle: &ServiceStatusHandle,
        state: ServiceState,
        controls_accepted: ServiceControlAccept,
        exit_code: ServiceExitCode,
        wait_hint_secs: u64,
    ) -> BabataResult<()> {
        let checkpoint = match state {
            ServiceState::StartPending | ServiceState::StopPending => 1,
            _ => 0,
        };
        status_handle
            .set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted,
                exit_code,
                checkpoint,
                wait_hint: Duration::from_secs(wait_hint_secs),
                process_id: None,
            })
            .map_err(|err| {
                BabataError::internal(format!("Failed to update Windows service status: {}", err))
            })
    }

    fn spawn_server_child() -> BabataResult<Child> {
        let home_dir = SERVICE_HOME_DIR.get().cloned().ok_or_else(|| {
            BabataError::internal("Windows service home directory not initialized")
        })?;
        let home_path = PathBuf::from(&home_dir);
        let workdir = home_path.join(".babata");
        std::fs::create_dir_all(&workdir).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create working directory '{}': {}",
                workdir.display(),
                err
            ))
        })?;

        let exe_path = SERVICE_EXE_PATH.get().cloned().ok_or_else(|| {
            BabataError::internal("Windows service executable path not initialized")
        })?;

        let cargo_bin = home_path.join(".cargo").join("bin");
        let existing_path = std::env::var("PATH").unwrap_or_default();
        let merged_path = if existing_path.is_empty() {
            cargo_bin.to_string_lossy().into_owned()
        } else {
            format!("{};{}", cargo_bin.to_string_lossy(), existing_path)
        };

        let mut child_cmd = Command::new(exe_path);
        child_cmd
            .arg("server")
            .arg("serve")
            .current_dir(&workdir)
            .env("HOME", &home_dir)
            .env("USERPROFILE", &home_dir)
            .env("PATH", merged_path);

        child_cmd.spawn().map_err(|err| {
            BabataError::internal(format!(
                "Failed to start babata server child process: {}",
                err
            ))
        })
    }

    fn terminate_child(child: &mut Child) -> BabataResult<()> {
        if child
            .try_wait()
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to inspect child process before termination: {}",
                    err
                ))
            })?
            .is_some()
        {
            return Ok(());
        }

        child.kill().map_err(|err| {
            BabataError::internal(format!("Failed to stop child process: {}", err))
        })?;
        let _ = child.wait();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::message::{Content, Message};

    use super::{
        is_macos_service_not_found_error, service_started_message, windows_service_bin_path,
    };

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
    fn builds_windows_service_bin_path_with_home_dir() {
        let cmdline = windows_service_bin_path(
            std::path::Path::new(r"C:\Users\alice\.cargo\bin\babata.exe"),
            std::path::Path::new(r"C:\Users\alice"),
        );
        assert_eq!(
            cmdline,
            "\"C:\\Users\\alice\\.cargo\\bin\\babata.exe\" server windows-service-host --home-dir \"C:\\Users\\alice\""
        );
    }
}
