use std::cell::Cell;

use babata::{
    cli::{
        Command,
        dashboard::{
            attempt_open_dashboard, dashboard_url, ensure_dashboard_running, run_dashboard_impl,
        },
    },
    http::DEFAULT_HTTP_BASE_URL,
};
use clap::Parser;

#[test]
fn dashboard_command_parses_no_open() {
    let command = Command::parse_from(["babata", "dashboard", "--no-open"]);
    assert!(matches!(command, Command::Dashboard { no_open: true }));
}

#[test]
fn dashboard_command_parses_default_open() {
    let command = Command::parse_from(["babata", "dashboard"]);
    assert!(matches!(command, Command::Dashboard { no_open: false }));
}

#[test]
fn dashboard_url_matches_http_default() {
    assert_eq!(dashboard_url(), DEFAULT_HTTP_BASE_URL);
}

#[test]
fn dashboard_service_already_healthy_skips_start() {
    let started = Cell::new(false);

    ensure_dashboard_running(
        || Ok(true),
        || {
            started.set(true);
            Ok(())
        },
    )
    .unwrap();

    assert!(!started.get());
}

#[test]
fn dashboard_service_unhealthy_triggers_start_and_waits() {
    let started = Cell::new(false);
    let health_checks = Cell::new(0);

    ensure_dashboard_running(
        || {
            let checks = health_checks.get();
            health_checks.set(checks + 1);
            Ok(started.get())
        },
        || {
            started.set(true);
            Ok(())
        },
    )
    .unwrap();

    assert!(started.get());
    assert!(health_checks.get() >= 2);
}

#[test]
fn dashboard_opens_browser_after_readiness() {
    let opened = Cell::new(false);

    run_dashboard_impl(
        false,
        || Ok(true),
        || panic!("start should not run when already healthy"),
        |received| {
            opened.set(true);
            assert_eq!(received, dashboard_url());
            Ok(())
        },
    )
    .unwrap();

    assert!(opened.get());
}

#[test]
fn dashboard_skips_browser_with_no_open() {
    let opened = Cell::new(false);

    run_dashboard_impl(
        true,
        || Ok(true),
        || Ok(()),
        |_| {
            opened.set(true);
            Ok(())
        },
    )
    .unwrap();

    assert!(!opened.get());
}

#[test]
fn dashboard_opener_helper_invokes_closure() {
    let url = dashboard_url();
    let opened = Cell::new(false);

    attempt_open_dashboard(false, url, |received| {
        opened.set(true);
        assert_eq!(received, url);
        Ok(())
    })
    .unwrap();

    assert!(opened.get());
}

#[test]
fn dashboard_opener_helper_skips_when_no_open() {
    let url = dashboard_url();
    let opened = Cell::new(false);

    attempt_open_dashboard(true, url, |_| {
        opened.set(true);
        Ok(())
    })
    .unwrap();

    assert!(!opened.get());
}
