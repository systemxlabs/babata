use std::time::Duration;

use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
};

#[derive(Debug, Clone)]
pub struct SleepTool {
    spec: ToolSpec,
}

impl SleepTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "sleep".to_string(),
                description:
                    "Sleep for a period of time and return after the wait completes. Use this when you intentionally need to wait before the next step."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "seconds": {
                            "type": "number",
                            "description": "Sleep duration in seconds"
                        }
                    },
                    "required": ["seconds"]
                }),
            },
        }
    }
}

impl Default for SleepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for SleepTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;

        let duration = if let Some(seconds) = args["seconds"].as_f64() {
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(BabataError::tool(
                    "seconds must be a non-negative finite number",
                ));
            }
            Duration::from_secs_f64(seconds)
        } else {
            return Err(BabataError::tool("Missing sleep duration: provide seconds"));
        };

        tokio::time::sleep(duration).await;

        Ok(format!("Slept for {:.3} seconds", duration.as_secs_f64()))
    }
}
