use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;

/// Represents an external TUI tool that can be used to view logs
#[derive(Debug, Clone)]
pub struct ExternalTool {
    /// Name of the tool (e.g., "jless", "visidata")
    pub name: String,
    /// Command to check if tool is available (e.g., "jless", "vd")
    pub check_cmd: String,
    /// Command to run the tool (may differ from check_cmd)
    pub run_cmd: String,
    /// Arguments to pass to the tool
    pub args: Vec<String>,
    /// Description of what this tool is good for
    pub description: String,
    /// Whether the tool reads from stdin
    #[allow(dead_code)]
    pub reads_stdin: bool,
    /// Whether the tool needs a file (if false, uses stdin)
    #[allow(dead_code)]
    pub needs_file: bool,
}

impl ExternalTool {
    /// Check if this tool is available on the system
    pub fn is_available(&self) -> bool {
        Command::new(&self.check_cmd)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            || Command::new(&self.check_cmd)
                .arg("-h")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            || Command::new(&self.check_cmd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
    }

    /// Get a list of all available external tools
    #[allow(dead_code)]
    pub fn get_available_tools() -> Vec<ExternalTool> {
        Self::all_tools()
            .into_iter()
            .filter(|tool| tool.is_available())
            .collect()
    }

    /// Get all registered tools (regardless of availability)
    pub fn all_tools() -> Vec<ExternalTool> {
        vec![
            // JSON viewers
            ExternalTool {
                name: "jless".to_string(),
                check_cmd: "jless".to_string(),
                run_cmd: "jless".to_string(),
                args: vec!["--no-auto-expand".to_string()],
                description: "JSON viewer with syntax highlighting and navigation".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            ExternalTool {
                name: "fx".to_string(),
                check_cmd: "fx".to_string(),
                run_cmd: "fx".to_string(),
                args: vec![],
                description: "Interactive JSON viewer with search and filtering".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            // Data analysis tools
            ExternalTool {
                name: "visidata".to_string(),
                check_cmd: "vd".to_string(),
                run_cmd: "vd".to_string(),
                args: vec!["-f".to_string(), "jsonl".to_string()],
                description: "Interactive spreadsheet/data analysis tool for structured data".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            ExternalTool {
                name: "tabview".to_string(),
                check_cmd: "tabview".to_string(),
                run_cmd: "tabview".to_string(),
                args: vec![],
                description: "Table viewer for structured data".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            // Log viewers
            ExternalTool {
                name: "lnav".to_string(),
                check_cmd: "lnav".to_string(),
                run_cmd: "lnav".to_string(),
                args: vec![],
                description: "Advanced log file viewer with SQL queries and filtering".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            ExternalTool {
                name: "gonzo".to_string(),
                check_cmd: "gonzo".to_string(),
                run_cmd: "gonzo".to_string(),
                args: vec![],
                description: "Real-time log analysis terminal UI".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            // CSV/TSV viewers
            ExternalTool {
                name: "csvtk".to_string(),
                check_cmd: "csvtk".to_string(),
                run_cmd: "csvtk".to_string(),
                args: vec!["view".to_string()],
                description: "CSV/TSV viewer and processor".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
            // Generic text viewers with navigation
            ExternalTool {
                name: "less".to_string(),
                check_cmd: "less".to_string(),
                run_cmd: "less".to_string(),
                args: vec!["-R".to_string(), "-S".to_string()],
                description: "Text viewer with search and navigation (fallback)".to_string(),
                reads_stdin: true,
                needs_file: false,
            },
        ]
    }

    /// Spawn the external tool with logs piped to it
    pub async fn spawn_with_logs(&self, logs: &[String]) -> Result<()> {
        let mut cmd = TokioCommand::new(&self.run_cmd);
        cmd.args(&self.args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd.spawn()
            .context(format!("Failed to spawn {}", self.name))?;

        // Write logs to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let log_text = logs.join("\n");
            stdin.write_all(log_text.as_bytes()).await
                .context("Failed to write logs to external tool")?;
            stdin.flush().await
                .context("Failed to flush logs to external tool")?;
            drop(stdin); // Close stdin so tool knows input is done
        }

        // Wait for tool to exit
        let status = child.wait().await
            .context(format!("Failed to wait for {}", self.name))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "{} exited with status: {:?}",
                self.name,
                status.code()
            ));
        }

        Ok(())
    }
}

/// Registry of external tools, mapped by their names
pub struct ToolRegistry {
    tools: HashMap<String, ExternalTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        // Register all tools
        for tool in ExternalTool::all_tools() {
            registry.tools.insert(tool.name.clone(), tool);
        }
        
        registry
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&ExternalTool> {
        self.tools.get(name)
    }

    /// Get all available tools (that are installed)
    pub fn get_available(&self) -> Vec<&ExternalTool> {
        self.tools
            .values()
            .filter(|tool| tool.is_available())
            .collect()
    }

    /// Get tool names for AI prompt
    #[allow(dead_code)]
    pub fn get_available_names(&self) -> Vec<String> {
        self.get_available()
            .iter()
            .map(|t| t.name.clone())
            .collect()
    }

    /// Get tool descriptions for AI prompt
    pub fn get_available_descriptions(&self) -> String {
        self.get_available()
            .iter()
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

