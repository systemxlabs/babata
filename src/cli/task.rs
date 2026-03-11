use std::sync::Arc;

use crate::{
    BabataResult,
    config::Config,
    error::BabataError,
    runtime::TaskRuntime,
    task::{TaskStatus, TaskStore},
};

use super::TaskAction;

pub fn run(_args: &super::Args, action: &TaskAction) {
    let result = match action {
        TaskAction::List { status } => list_tasks(status.as_deref()),
        TaskAction::Show { task_id } => show_task(task_id),
        TaskAction::Pause { task_id } => update_task_status(task_id, TaskStatus::Paused),
        TaskAction::Cancel { task_id } => update_task_status(task_id, TaskStatus::Canceled),
        TaskAction::Resume { task_id } => resume_task(task_id),
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn list_tasks(status: Option<&str>) -> BabataResult<()> {
    let store = TaskStore::open_default()?;
    let tasks = match status {
        Some(status) => store.list_tasks_by_status(TaskStatus::parse(status)?)?,
        None => store.list_tasks()?,
    };

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    for task in tasks {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            task.task_id,
            task.status.as_str(),
            task.agent_name,
            task.updated_at,
            task.model
        );
    }

    Ok(())
}

fn show_task(task_id: &str) -> BabataResult<()> {
    let store = TaskStore::open_default()?;
    let snapshot = store.load_snapshot(task_id)?;
    let final_output = store.read_final_output(task_id)?;

    println!("task_id: {}", snapshot.record.task_id);
    println!("status: {}", snapshot.record.status.as_str());
    println!("agent: {}", snapshot.record.agent_name);
    println!("provider: {}", snapshot.record.provider_name);
    println!("model: {}", snapshot.record.model);
    println!("created_at: {}", snapshot.record.created_at);
    println!("updated_at: {}", snapshot.record.updated_at);
    if let Some(completed_at) = &snapshot.record.completed_at {
        println!("completed_at: {}", completed_at);
    }
    if let Some(parent_task_id) = &snapshot.record.parent_task_id {
        println!("parent_task_id: {}", parent_task_id);
    }
    println!("root_task_id: {}", snapshot.record.root_task_id);
    if let Some(last_error) = &snapshot.record.last_error {
        println!("last_error: {}", last_error);
    }

    println!("\n=== task.md ===\n{}", snapshot.task_markdown);
    println!("\n=== progress.md ===\n{}", snapshot.progress_markdown);

    if let Some(final_output) = final_output {
        println!("\n=== final_output.md ===\n{}", final_output);
    }

    println!("\n=== artifacts ===");
    if snapshot.artifacts.is_empty() {
        println!("(none)");
    } else {
        for artifact in snapshot.artifacts {
            println!("{}", artifact.relative_path);
        }
    }

    Ok(())
}

fn update_task_status(task_id: &str, status: TaskStatus) -> BabataResult<()> {
    let store = TaskStore::open_default()?;
    let record = store.get_task(task_id)?;
    if record.status == status {
        println!("Task '{}' already {}", task_id, status.as_str());
        return Ok(());
    }

    store.set_status(task_id, status, None, None)?;
    println!("Task '{}' set to {}", task_id, status.as_str());
    Ok(())
}

fn resume_task(task_id: &str) -> BabataResult<()> {
    let config = Config::load()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    runtime.block_on(async {
        let runtime = Arc::new(TaskRuntime::new(config)?);
        let record = runtime.store().get_task(task_id)?;

        if record.status == TaskStatus::Done {
            return Err(BabataError::config(format!(
                "Task '{}' is already done",
                task_id
            )));
        }

        runtime
            .store()
            .set_status(task_id, TaskStatus::Running, None, None)?;
        runtime.spawn_task(task_id.to_string()).await?;
        let message = runtime.wait_for_task(task_id).await?;

        if let crate::message::Message::AssistantResponse { content, .. } = message {
            for part in content {
                if let crate::message::Content::Text { text } = part {
                    println!("{text}");
                }
            }
        }

        Ok(())
    })
}
