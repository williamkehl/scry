use std::process;

/// Detect the input source (file, command, or stdin)
pub fn detect_input_source(stdin_is_tty: bool) -> String {
    if stdin_is_tty {
        return "Waiting for input...".to_string();
    }

    // Try to detect parent process command
    // On Unix systems, we can try to read from /proc or use ps
    #[cfg(unix)]
    {
        // Try to get parent process ID
        let ppid = unsafe { libc::getppid() };
        
        // Try to read parent process command from /proc (Linux) or ps (macOS/BSD)
        if let Ok(cmd) = get_parent_command(ppid) {
            // Clean up the command to show what's piping to us
            let cleaned = clean_command(&cmd);
            if !cleaned.is_empty() {
                return format!("Reading from: {}", cleaned);
            }
        }
    }

    // Fallback: generic message
    "Reading from stdin".to_string()
}

#[cfg(unix)]
fn get_parent_command(ppid: i32) -> Result<String, std::io::Error> {
    // Try Linux /proc first
    let proc_cmdline = format!("/proc/{}/cmdline", ppid);
    if let Ok(cmdline) = std::fs::read_to_string(&proc_cmdline) {
        // cmdline is null-separated, replace with spaces
        let cmd = cmdline
            .split('\0')
            .take(3) // Limit to first few args
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !cmd.is_empty() {
            return Ok(cmd);
        }
    }

    // Fallback: try ps command (works on macOS/BSD)
    let output = process::Command::new("ps")
        .args(&["-p", &ppid.to_string(), "-o", "command="])
        .output();
    
    if let Ok(output) = output {
        if output.status.success() {
            let cmd = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !cmd.is_empty() {
                return Ok(cmd);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not determine parent command",
    ))
}

fn clean_command(cmd: &str) -> String {
    let cmd = cmd.trim();
    
    // Remove common shell wrappers
    let cmd = if cmd.starts_with("sh -c ") || cmd.starts_with("/bin/sh -c ") {
        // Extract the actual command from "sh -c 'command'"
        if let Some(start) = cmd.find('\'') {
            if let Some(end) = cmd.rfind('\'') {
                if end > start {
                    return cmd[start + 1..end].to_string();
                }
            }
        }
        cmd
    } else {
        cmd
    };

    // Truncate very long commands
    if cmd.len() > 60 {
        format!("{}...", &cmd[..57])
    } else {
        cmd.to_string()
    }
}

