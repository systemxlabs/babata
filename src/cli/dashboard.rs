use std::{
    ffi::OsString,
    process,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::get;

use crate::{BabataResult, cli::server, error::BabataError, http::DEFAULT_HTTP_BASE_URL};

const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(200);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(15);

pub fn run(no_open: bool) {
    if let Err(err) = run_dashboard(no_open) {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run_dashboard(no_open: bool) -> BabataResult<()> {
    run_dashboard_impl(
        no_open,
        default_health_check,
        start_dashboard_service,
        open_dashboard_browser,
    )
}

pub fn run_dashboard_impl<H, S, O>(
    no_open: bool,
    health_check: H,
    start_service: S,
    opener: O,
) -> BabataResult<()>
where
    H: Fn() -> BabataResult<bool>,
    S: Fn() -> BabataResult<()>,
    O: Fn(&str) -> BabataResult<()>,
{
    run_dashboard_impl_with_announce(no_open, health_check, start_service, opener, |url| {
        println!("{url}");
        Ok(())
    })
}

pub fn run_dashboard_impl_with_announce<H, S, O, A>(
    no_open: bool,
    health_check: H,
    start_service: S,
    opener: O,
    announce: A,
) -> BabataResult<()>
where
    H: Fn() -> BabataResult<bool>,
    S: Fn() -> BabataResult<()>,
    O: Fn(&str) -> BabataResult<()>,
    A: Fn(&str) -> BabataResult<()>,
{
    ensure_dashboard_running(health_check, start_service)?;
    let url = dashboard_url();
    announce(url)?;
    attempt_open_dashboard(no_open, url, opener)?;
    Ok(())
}

pub fn dashboard_url() -> &'static str {
    DEFAULT_HTTP_BASE_URL
}

pub fn start_dashboard_service() -> BabataResult<()> {
    start_dashboard_service_with_fallback(
        server::start_background_service,
        start_detached_server_process,
    )
}

pub fn start_dashboard_service_with_fallback<S, F>(
    start_background_service: S,
    start_detached_server: F,
) -> BabataResult<()>
where
    S: Fn() -> BabataResult<()>,
    F: Fn() -> BabataResult<()>,
{
    match start_background_service() {
        Ok(()) => Ok(()),
        Err(err) if should_fallback_to_detached_server(&err) => start_detached_server(),
        Err(err) => Err(err),
    }
}

pub fn attempt_open_dashboard<F>(no_open: bool, url: &str, opener: F) -> BabataResult<()>
where
    F: Fn(&str) -> BabataResult<()>,
{
    if no_open { Ok(()) } else { opener(url) }
}

pub fn ensure_dashboard_running<H, S>(health_check: H, start_service: S) -> BabataResult<()>
where
    H: Fn() -> BabataResult<bool>,
    S: Fn() -> BabataResult<()>,
{
    if health_check()? {
        return Ok(());
    }

    start_service()?;
    wait_for_dashboard_ready(&health_check)
}

fn wait_for_dashboard_ready<FHealth>(health_check: &FHealth) -> BabataResult<()>
where
    FHealth: Fn() -> BabataResult<bool>,
{
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    while Instant::now() <= deadline {
        if health_check()? {
            return Ok(());
        }
        thread::sleep(HEALTH_POLL_INTERVAL);
    }

    Err(BabataError::internal(format!(
        "Timed out waiting for dashboard at {DEFAULT_HTTP_BASE_URL}/health"
    )))
}

fn default_health_check() -> BabataResult<bool> {
    let health_url = format!("{DEFAULT_HTTP_BASE_URL}/health");
    match get(&health_url) {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

fn open_dashboard_browser(url: &str) -> BabataResult<()> {
    webbrowser::open(url)
        .map_err(|err| BabataError::internal(format!("Failed to open browser for {url}: {err}")))
}

fn should_fallback_to_detached_server(error: &BabataError) -> bool {
    match error {
        BabataError::Config(message, _) => {
            message.contains("run \"babata onboard\" first")
                || message.contains("Server start is not supported")
        }
        _ => false,
    }
}

fn start_detached_server_process() -> BabataResult<()> {
    let exe_path = std::env::current_exe().map_err(|err| {
        BabataError::internal(format!("Failed to resolve current executable path: {err}"))
    })?;

    let mut command = Command::new(exe_path);
    command
        .args([OsString::from("server"), OsString::from("serve")])
        .env(server::DASHBOARD_BOOTSTRAP_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command.spawn().map_err(|err| {
        BabataError::internal(format!("Failed to spawn detached dashboard server: {err}"))
    })?;

    Ok(())
}
