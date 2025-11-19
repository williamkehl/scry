use crate::config;
use crate::plugins::ToolRegistry;
use crate::utils;
use crate::views::ViewKind;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ModelResponse {
    view: String,
    #[serde(default)]
    tool: Option<String>,
}

pub async fn analyze_logs(logs: &[String]) -> Result<(ViewKind, String)> {
    let api_key = config::get_api_key()?;

    // Log what we're doing (this will be shown in status bar via the caller)
    let client = reqwest::Client::new();

    // Get available external tools
    let registry = ToolRegistry::new();
    let available_tools = registry.get_available();
    let tool_descriptions = if !available_tools.is_empty() {
        format!("\n\nExternal tools (if installed):\n{}", registry.get_available_descriptions())
    } else {
        String::new()
    };

    let system_prompt = format!(r#"You are selecting the best terminal UI layout for viewing incoming logs. Respond ONLY with JSON. Available layouts:

Built-in views:
- Plain: good for freeform unstructured lines.
- KeyValue: good for lines with key=value pairs.
- Json: good for structured JSON logs.

{}

If an external tool would provide a better viewing experience (e.g., jless for complex JSON, visidata for tabular data, lnav for log files with timestamps), prefer it over built-in views. Otherwise, use a built-in view.

Respond with JSON:
{{ "view": "Plain" }} OR
{{ "view": "KeyValue" }} OR
{{ "view": "Json" }} OR
{{ "view": "ExternalTool", "tool": "tool_name" }}

Examples:
{{ "view": "Plain" }}
{{ "view": "Json" }}
{{ "view": "ExternalTool", "tool": "jless" }}
{{ "view": "ExternalTool", "tool": "visidata" }}"#, tool_descriptions);

    // Safely prepare logs for OpenAI API
    // Sanitize and truncate to avoid issues with:
    // - Very long lines
    // - Control characters that might break the API
    // - Special characters
    let sample_logs: String = logs
        .iter()
        .rev()
        .take(100)
        .rev()
        .map(|s| {
            // Sanitize each line for safe API transmission
            // Remove control chars, truncate very long lines
            let sanitized = utils::sanitize_for_display(s, 500); // Max 500 chars per line for API
            sanitized
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Truncate total message if it's too long (OpenAI has token limits)
    let max_message_len = 10000; // Reasonable limit
    let user_message = if sample_logs.len() > max_message_len {
        format!(
            "Analyze these log lines and select the best view:\n\n{}...\n[truncated {} chars]",
            &sample_logs[..max_message_len],
            sample_logs.len() - max_message_len
        )
    } else {
        format!(
            "Analyze these log lines and select the best view:\n\n{}",
            sample_logs
        )
    };

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct RequestBody {
        model: String,
        messages: Vec<Message>,
        response_format: ResponseFormat,
    }

    #[derive(Serialize)]
    struct ResponseFormat {
        #[serde(rename = "type")]
        type_field: String,
    }

    let model_name = "gpt-4o-mini"; // Using gpt-4o-mini as gpt-5.1-mini doesn't exist yet
    
    let request_body = RequestBody {
        model: model_name.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_message,
            },
        ],
        response_format: ResponseFormat {
            type_field: "json_object".to_string(),
        },
    };

    // Make the API call to OpenAI
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context(format!("Failed to send request to OpenAI API (POST /v1/chat/completions with model {})", model_name))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "OpenAI API error: {} - {}",
            status,
            error_text
        ));
    }

    let json_response: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse OpenAI API response")?;

    let content = json_response["choices"][0]["message"]["content"]
        .as_str()
        .context("No content in OpenAI response")?;

    let model_response: ModelResponse = serde_json::from_str(content)
        .context("Failed to parse model response as JSON")?;

    let (view_kind, view_name) = match model_response.view.as_str() {
        "Plain" => (ViewKind::Plain, "Plain".to_string()),
        "KeyValue" => (ViewKind::KeyValue, "KeyValue".to_string()),
        "Json" => (ViewKind::Json, "Json".to_string()),
        "ExternalTool" => {
            let tool_name = model_response.tool
                .ok_or_else(|| anyhow::anyhow!("ExternalTool view requires 'tool' field"))?;
            
            // Verify tool is available
            let registry = ToolRegistry::new();
            if let Some(tool) = registry.get(&tool_name) {
                if tool.is_available() {
                    (ViewKind::ExternalTool(tool_name.clone()), format!("External: {}", tool_name))
                } else {
                    // Fallback to Json if tool not available
                    (ViewKind::Json, format!("Json ({} not available)", tool_name))
                }
            } else {
                return Err(anyhow::anyhow!("Unknown external tool: {}", tool_name));
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown view type: {}",
                model_response.view
            ));
        }
    };

    // Return summary with API call details
    let summary = format!("OpenAI API ({}) → Selected view: {}", model_name, view_name);
    Ok((view_kind, summary))
}

