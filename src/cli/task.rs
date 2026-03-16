use chrono::{Local, TimeZone};
use comfy_table::{ContentArrangement, Table, presets::{ASCII_MARKDOWN}};
use reqwest::Client;
use serde_json::json;

use crate::{
    BabataResult,
    error::BabataError,
    http::{DEFAULT_HTTP_BASE_URL, ListTasksResponse, TaskResponse},
    message::Content,
};

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

        println!("Task '{}' {} completed", task_id, action);
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
        let response: ListTasksResponse = serde_json::from_str(&body).map_err(|err| {
            BabataError::internal(format!("Failed to parse task list response body: {}", err))
        })?;
        println!("{}", format_tasks_table(&response.tasks));
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

fn format_tasks_table(tasks: &[TaskResponse]) -> String {
    if tasks.is_empty() {
        return "No tasks found.".to_string();
    }

    let mut table = Table::new();
    table
        .load_preset(ASCII_MARKDOWN)
        .set_content_arrangement(ContentArrangement::Disabled)
        .set_header(["TASK ID", "STATUS", "AGENT", "PARENT", "CREATED AT", "PROMPT"]);

    for task in tasks {
        table.add_row([
            task.task_id.clone(),
            task.status.clone(),
            task.agent.clone().unwrap_or_else(|| "-".to_string()),
            task.parent_task_id
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            format_timestamp(task.created_at),
            summarize_prompt(&task.prompt),
        ]);
    }

    table.to_string()
}

fn format_timestamp(timestamp_millis: i64) -> String {
    match Local.timestamp_millis_opt(timestamp_millis).single() {
        Some(datetime) => datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => timestamp_millis.to_string(),
    }
}

fn summarize_prompt(prompt: &[Content]) -> String {
    prompt
        .iter()
        .map(|content| match content {
            Content::Text { text } => text.clone(),
            Content::ImageUrl { url } => format!("[image] {url}"),
            Content::ImageData { media_type, .. } => format!("[image_data] {}", media_type),
            Content::AudioData { media_type, .. } => format!("[audio_data] {}", media_type),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use crate::message::MediaType;

    use super::*;

    #[test]
    fn format_tasks_table_renders_headers_and_rows() {
        let tasks = vec![TaskResponse {
            task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
            prompt: vec![Content::Text {
                text: "run a very long task prompt here".to_string(),
            }],
            agent: Some("babata".to_string()),
            status: "running".to_string(),
            parent_task_id: None,
            root_task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
            created_at: 1_773_994_800_000,
        }];

        let table = format_tasks_table(&tasks);

        assert!(table.contains("TASK ID"));
        assert!(table.contains("STATUS"));
        assert!(table.contains("12345678-1234-1234-1234-123456789abc"));
        assert!(table.contains("running"));
        assert!(table.contains("babata"));
        assert!(table.contains("run a very long task prompt here"));
    }

    #[test]
    fn summarize_prompt_formats_non_text_content_without_truncation() {
        let summary = summarize_prompt(&[
            Content::Text {
                text: "hello".to_string(),
            },
            Content::ImageData {
                data: "abc".to_string(),
                media_type: MediaType::ImagePng,
            },
            Content::Text {
                text: "x".repeat(80),
            },
        ]);

        assert!(summary.starts_with("hello [image_data] image/png "));
        assert!(summary.ends_with(&"x".repeat(80)));
    }
}
