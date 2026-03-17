use std::time::Duration;

use chrono::{DateTime, Utc};
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
                    "Sleep for a period of time or until a specific time and return after the wait completes. Use this when you intentionally need to wait before the next step."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "seconds": {
                            "type": "number",
                            "description": "Sleep duration in seconds"
                        },
                        "until": {
                            "type": "string",
                            "description": "Wake-up time in RFC3339 format with timezone, for example 2026-03-16T18:30:00+08:00"
                        }
                    }
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

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let duration = parse_sleep_duration(&args)?;

        tokio::time::sleep(duration).await;

        Ok(format!("Slept for {:.3} seconds", duration.as_secs_f64()))
    }
}

fn parse_sleep_duration(args: &Value) -> BabataResult<Duration> {
    let seconds = args.get("seconds").and_then(Value::as_f64);
    let until = args.get("until").and_then(Value::as_str);

    match (seconds, until) {
        (Some(_), Some(_)) => Err(BabataError::tool("Provide exactly one of seconds or until")),
        (Some(seconds), None) => {
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(BabataError::tool(
                    "seconds must be a non-negative finite number",
                ));
            }
            Ok(Duration::from_secs_f64(seconds))
        }
        (None, Some(until)) => parse_until_duration(until),
        (None, None) => Err(BabataError::tool(
            "Missing sleep duration: provide either seconds or until",
        )),
    }
}

fn parse_until_duration(until: &str) -> BabataResult<Duration> {
    let until = DateTime::parse_from_rfc3339(until).map_err(|err| {
        BabataError::tool(format!(
            "until must be a valid RFC3339 datetime with timezone: {}",
            err
        ))
    })?;
    let now = Utc::now();
    compute_sleep_until_duration(now, until.with_timezone(&Utc))
}

fn compute_sleep_until_duration(
    now: DateTime<Utc>,
    until: DateTime<Utc>,
) -> BabataResult<Duration> {
    let delay = until.signed_duration_since(now);
    if delay.num_milliseconds() < 0 {
        return Err(BabataError::tool("until must be in the future"));
    }

    delay.to_std().map_err(|err| {
        BabataError::tool(format!(
            "Failed to convert until datetime into sleep duration: {}",
            err
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::{compute_sleep_until_duration, parse_sleep_duration};

    #[test]
    fn parse_sleep_duration_accepts_seconds() {
        let duration = parse_sleep_duration(&json!({ "seconds": 1.5 })).expect("parse seconds");
        assert_eq!(duration, Duration::from_millis(1500));
    }

    #[test]
    fn parse_sleep_duration_rejects_both_seconds_and_until() {
        let err = parse_sleep_duration(&json!({
            "seconds": 1,
            "until": "2026-03-16T18:30:00+08:00"
        }))
        .expect_err("reject conflicting inputs");
        assert!(
            err.to_string()
                .contains("Provide exactly one of seconds or until")
        );
    }

    #[test]
    fn compute_sleep_until_duration_accepts_future_time() {
        let now = Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 2).unwrap();
        let duration =
            compute_sleep_until_duration(now, until).expect("compute future sleep duration");
        assert_eq!(duration, Duration::from_secs(2));
    }

    #[test]
    fn compute_sleep_until_duration_rejects_past_time() {
        let now = Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 2).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 0).unwrap();
        let err = compute_sleep_until_duration(now, until).expect_err("reject past time");
        assert!(err.to_string().contains("until must be in the future"));
    }
}
