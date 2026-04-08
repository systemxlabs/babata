use chrono::{Local, TimeZone};
use comfy_table::{ContentArrangement, Table, presets::ASCII_MARKDOWN};
use reqwest::Client;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::{CountTasksResponse, DEFAULT_HTTP_BASE_URL, ListTasksResponse, TaskResponse},
    message::Content,
    task::CreateTaskRequest,
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

pub fn create(prompt: &str, agent: Option<&str>, parent_task_id: Option<&str>, never_ends: bool) {
    if let Err(err) = run_create(prompt, agent, parent_task_id, never_ends) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn list(status: Option<&str>, limit: Option<usize>, pretty_format: bool) {
    if let Err(err) = run_list(status, limit, pretty_format) {
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

pub fn count(status: Option<&str>) {
    if let Err(err) = run_count(status) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_control(action: &str, task_id: &str) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let response = Client::new()
            .post(format!(
                "{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}/{action}"
            ))
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

fn run_create(
    prompt: &str,
    agent: Option<&str>,
    parent_task_id: Option<&str>,
    never_ends: bool,
) -> BabataResult<()> {
    if prompt.trim().is_empty() {
        return Err(BabataError::config("prompt cannot be empty"));
    }

    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let parent_task_id = match parent_task_id {
            Some(parent_task_id) => Some(Uuid::parse_str(parent_task_id).map_err(|err| {
                BabataError::config(format!(
                    "Invalid parent_task_id '{}': {}",
                    parent_task_id, err
                ))
            })?),
            None => None,
        };
        let request = CreateTaskRequest {
            prompt: vec![Content::Text {
                text: prompt.to_string(),
            }],
            agent: agent.map(ToOwned::to_owned),
            parent_task_id,
            never_ends,
        };
        let response = Client::new()
            .post(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks"))
            .json(&request)
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

fn run_list(status: Option<&str>, limit: Option<usize>, pretty_format: bool) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let client = Client::new();
        let mut request = client.get(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks"));
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
        if pretty_format {
            println!("{}", format_tasks_table(&response.tasks));
        } else {
            println!("{}", format_tasks_json_lines(&response.tasks)?);
        }
        Ok(())
    })
}

fn run_get(task_id: &str) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let response = Client::new()
            .get(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}"))
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

fn run_count(status: Option<&str>) -> BabataResult<()> {
    let runtime = build_runtime()?;
    runtime.block_on(async move {
        let client = Client::new();
        let mut request = client.get(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/count"));
        if let Some(status) = status {
            request = request.query(&[("status", status)]);
        }

        let response = request.send().await.map_err(|err| {
            BabataError::internal(format!(
                "Failed to call local HTTP API for task count: {}",
                err
            ))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Task count request failed with status {}: {}",
                status, body
            )));
        }

        let body = response.text().await.map_err(|err| {
            BabataError::internal(format!("Failed to read task count response body: {}", err))
        })?;
        let count_response: CountTasksResponse = serde_json::from_str(&body).map_err(|err| {
            BabataError::internal(format!("Failed to parse task count response body: {}", err))
        })?;
        println!("{}", count_response.count);
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
        .set_header([
            "TASK ID",
            "STATUS",
            "NEVER ENDS",
            "AGENT",
            "PARENT",
            "CREATED AT",
            "DESCRIPTION",
        ]);

    for task in tasks {
        table.add_row([
            task.task_id.clone(),
            task.status.clone(),
            task.never_ends.to_string(),
            task.agent.clone().unwrap_or_else(|| "-".to_string()),
            task.parent_task_id
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            format_timestamp(task.created_at),
            task.description.clone(),
        ]);
    }

    table.to_string()
}

fn format_tasks_json_lines(tasks: &[TaskResponse]) -> BabataResult<String> {
    let lines = tasks
        .iter()
        .map(|task| {
            serde_json::to_string(task).map_err(|err| {
                BabataError::internal(format!("Failed to serialize task list item: {}", err))
            })
        })
        .collect::<BabataResult<Vec<_>>>()?;

    Ok(lines.join("\n"))
}

fn format_timestamp(timestamp_millis: i64) -> String {
    match Local.timestamp_millis_opt(timestamp_millis).single() {
        Some(datetime) => datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => timestamp_millis.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tasks_table_renders_headers_and_rows() {
        let tasks = vec![TaskResponse {
            task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
            description: "run a very long task prompt here".to_string(),
            agent: Some("babata".to_string()),
            status: "running".to_string(),
            parent_task_id: None,
            root_task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
            created_at: 1_773_994_800_000,
            never_ends: false,
        }];

        let table = format_tasks_table(&tasks);

        assert!(table.contains("TASK ID"));
        assert!(table.contains("STATUS"));
        assert!(table.contains("NEVER ENDS"));
        assert!(table.contains("12345678-1234-1234-1234-123456789abc"));
        assert!(table.contains("running"));
        assert!(table.contains("false"));
        assert!(table.contains("babata"));
        assert!(table.contains("run a very long task prompt here"));
    }

    #[test]
    fn format_tasks_json_lines_renders_one_json_per_line() {
        let tasks = vec![
            TaskResponse {
                task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
                description: "first".to_string(),
                agent: Some("babata".to_string()),
                status: "running".to_string(),
                parent_task_id: None,
                root_task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
                created_at: 1_773_994_800_000,
                never_ends: true,
            },
            TaskResponse {
                task_id: "abcdefab-cdef-cdef-cdef-abcdefabcdef".to_string(),
                description: "second".to_string(),
                agent: None,
                status: "done".to_string(),
                parent_task_id: None,
                root_task_id: "abcdefab-cdef-cdef-cdef-abcdefabcdef".to_string(),
                created_at: 1_773_994_800_001,
                never_ends: false,
            },
        ];

        let output = format_tasks_json_lines(&tasks).expect("format json lines");
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"task_id\":\"12345678-1234-1234-1234-123456789abc\""));
        assert!(lines[0].contains("\"never_ends\":true"));
        assert!(lines[1].contains("\"task_id\":\"abcdefab-cdef-cdef-cdef-abcdefabcdef\""));
    }
}
