use crate::{BabataResult, error::BabataError, tool::ToolContext, utils::task_dir};
use std::path::PathBuf;

const MAX_ROWS: usize = 100;

pub fn process_query_results_with_truncation<T: serde::Serialize>(
    results: &[T],
    context: &ToolContext<'_>,
    tool_name: &str,
) -> BabataResult<String> {
    if results.len() <= MAX_ROWS {
        return serde_json::to_string(results).map_err(BabataError::from);
    }

    // Truncate: keep only the last MAX_ROWS rows
    let truncated_results = &results[results.len() - MAX_ROWS..];
    let truncated_json = serde_json::to_string(truncated_results).map_err(BabataError::from)?;

    // Write full results to file
    let log_file_path = get_query_log_path(context, tool_name)?;
    let full_json = serde_json::to_string_pretty(results).map_err(BabataError::from)?;
    std::fs::write(&log_file_path, full_json).map_err(|e| {
        BabataError::internal(format!(
            "Failed to write {} query log to '{}': {}",
            tool_name,
            log_file_path.display(),
            e
        ))
    })?;

    let header = format!(
        "... (results truncated, showing last {} rows, full results written to {})\n",
        MAX_ROWS,
        log_file_path.display()
    );

    Ok(header + &truncated_json)
}

fn get_query_log_path(context: &ToolContext<'_>, tool_name: &str) -> BabataResult<PathBuf> {
    let task_dir = task_dir(*context.task_id)?;
    let log_file_name = format!("{}-call-{}.json", tool_name, context.call_id);
    Ok(task_dir.join(log_file_name))
}
