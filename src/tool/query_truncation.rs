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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir(context: &ToolContext) -> std::path::PathBuf {
        let dir = task_dir(*context.task_id).unwrap();
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_no_truncation_when_under_limit() {
        let context = ToolContext::test();
        let _ = setup_test_dir(&context);
        let results = vec![1, 2, 3];
        let output = process_query_results_with_truncation(&results, &context, "test_tool").unwrap();
        
        assert_eq!(output, "[1,2,3]");
        
        // Verify no file was created
        let log_path = get_query_log_path(&context, "test_tool").unwrap();
        assert!(!log_path.exists());
    }

    #[test]
    fn test_truncation_when_over_limit() {
        let context = ToolContext::test();
        let _ = setup_test_dir(&context);
        let results: Vec<i32> = (0..110).collect();
        let tool_name = "test_tool_trunc";
        
        let output = process_query_results_with_truncation(&results, &context, tool_name).unwrap();
        
        // Check header
        assert!(output.contains("results truncated"));
        assert!(output.contains("showing last 100 rows"));
        
        // Check truncated content (last 100: 10 to 109)
        let json_part = output.split('\n').next_back().unwrap();
        let decoded: Vec<i32> = serde_json::from_str(json_part).unwrap();
        assert_eq!(decoded.len(), 100);
        assert_eq!(decoded[0], 10);
        assert_eq!(decoded[99], 109);
        
        // Verify file creation and content
        let log_path = get_query_log_path(&context, tool_name).unwrap();
        assert!(log_path.exists());
        
        let file_content = fs::read_to_string(&log_path).unwrap();
        let full_results: Vec<i32> = serde_json::from_str(&file_content).unwrap();
        assert_eq!(full_results.len(), 110);
        assert_eq!(full_results[0], 0);
        assert_eq!(full_results[109], 109);
        
        // Cleanup
        let _ = fs::remove_file(log_path);
    }

    #[test]
    fn test_empty_results() {
        let context = ToolContext::test();
        let _ = setup_test_dir(&context);
        let results: Vec<i32> = vec![];
        let output = process_query_results_with_truncation(&results, &context, "test_tool_empty").unwrap();
        
        assert_eq!(output, "[]");
        
        let log_path = get_query_log_path(&context, "test_tool_empty").unwrap();
        assert!(!log_path.exists());
    }
}
