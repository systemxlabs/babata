use reqwest::Client;
use serde_json::json;

use crate::{BabataResult, error::BabataError, http::DEFAULT_HTTP_BASE_URL};

pub fn pause(task_id: &str) {
    if let Err(err) = run_control("pause", task_id) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn resume(task_id: &str) {
    if let Err(err) = run_control("resume", task_id) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn cancel(task_id: &str) {
    if let Err(err) = run_control("cancel", task_id) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn create(prompt: &str, agent: Option<&str>, parent_task_id: Option<&str>) {
    if let Err(err) = run_create(prompt, agent, parent_task_id) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn list(status: Option<&str>, limit: Option<usize>) {
    if let Err(err) = run_list(status, limit) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn get(task_id: &str) {
    if let Err(err) = run_get(task_id) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_control(action: &str, task_id: &str) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let response = Client::new()
            .post(format!("{DEFAULT_HTTP_BASE_URL}/tasks/{task_id}/{action}"))
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to call local HTTP API for task {} '{}': {}",
                    action, task_id, err
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Task {} request failed with status {}: {}",
                action, status, body
            )));
        }

        println!("Task '{}' {}", task_id, action);
        Ok(())
    })
}

fn run_create(prompt: &str, agent: Option<&str>, parent_task_id: Option<&str>) -> BabataResult<()> {
    if prompt.trim().is_empty() {
        return Err(BabataError::config("prompt cannot be empty"));
    }

    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let response = Client::new()
            .post(format!("{DEFAULT_HTTP_BASE_URL}/tasks"))
            .json(&json!({
                "prompt": prompt,
                "agent": agent,
                "parent_task_id": parent_task_id,
            }))
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to call local HTTP API for task create: {}",
                    err
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Task create request failed with status {}: {}",
                status, body
            )));
        }

        let body = response.text().await.map_err(|err| {
            BabataError::internal(format!("Failed to read task create response body: {}", err))
        })?;
        println!("{body}");
        Ok(())
    })
}

fn run_list(status: Option<&str>, limit: Option<usize>) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let client = Client::new();
        let mut request = client.get(format!("{DEFAULT_HTTP_BASE_URL}/tasks"));
        if let Some(status) = status {
            request = request.query(&[("status", status)]);
        }
        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)]);
        }

        let response = request.send().await.map_err(|err| {
            BabataError::internal(format!(
                "Failed to call local HTTP API for task list: {}",
                err
            ))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Task list request failed with status {}: {}",
                status, body
            )));
        }

        let body = response.text().await.map_err(|err| {
            BabataError::internal(format!("Failed to read task list response body: {}", err))
        })?;
        println!("{body}");
        Ok(())
    })
}

fn run_get(task_id: &str) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let response = Client::new()
            .get(format!("{DEFAULT_HTTP_BASE_URL}/tasks/{task_id}"))
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to call local HTTP API for task get '{}': {}",
                    task_id, err
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Task get request failed with status {}: {}",
                status, body
            )));
        }

        let body = response.text().await.map_err(|err| {
            BabataError::internal(format!("Failed to read task get response body: {}", err))
        })?;
        println!("{body}");
        Ok(())
    })
}

fn build_runtime() -> BabataResult<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| BabataError::internal(format!("Failed to build Tokio runtime: {err}")))
}
