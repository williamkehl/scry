/// Utility functions for safe log handling

/// Sanitize a string for safe TUI display
/// - Removes or replaces control characters
/// - Truncates extremely long lines
/// - Handles invalid UTF-8 gracefully
pub fn sanitize_for_display(s: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(s.len().min(max_len));
    
    for ch in s.chars() {
        // Truncate if we've reached max length
        if result.len() >= max_len {
            result.push_str("...");
            break;
        }
        
        // Handle control characters
        match ch {
            // Keep common whitespace
            '\t' => result.push_str("  "), // Replace tabs with spaces
            '\n' => result.push(' '),      // Replace newlines with space
            '\r' => continue,              // Skip carriage returns
            // Replace other control characters with a visible placeholder
            ch if ch.is_control() => {
                // For common control chars, use readable representation
                let replacement = match ch as u32 {
                    0 => "\\0",
                    1..=31 => "?", // Other control chars
                    _ => "?",
                };
                result.push_str(replacement);
            }
            // Keep all other characters (including unicode)
            _ => result.push(ch),
        }
    }
    
    result
}

/// Safely convert a string to a display-safe version
/// Handles any input including binary data, invalid UTF-8 sequences, etc.
pub fn safe_string_display(s: &str) -> String {
    // First, try to sanitize the string
    let sanitized = sanitize_for_display(s, 1000); // Max 1000 chars per line
    
    // If the string is empty after sanitization, show a placeholder
    if sanitized.trim().is_empty() {
        return "[empty line]".to_string();
    }
    
    sanitized
}

/// Safely extract key-value pairs from a line
/// Handles edge cases like empty keys, special characters, etc.
pub fn extract_key_value_pairs(line: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    
    // Split by whitespace, but be careful with quoted values
    for part in line.split_whitespace() {
        // Skip empty parts
        if part.is_empty() {
            continue;
        }
        
        // Try to split on '=' but handle edge cases
        if let Some((key, value)) = part.split_once('=') {
            // Sanitize key and value
            let key = sanitize_for_display(key, 100);
            let value = sanitize_for_display(value, 200);
            
            // Only add if key is not empty
            if !key.is_empty() {
                pairs.push((key, value));
            }
        }
    }
    
    pairs
}

/// Safely format a JSON value for display
pub fn safe_json_display(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => sanitize_for_display(s, 500),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                format!("[{} items]", arr.len())
            }
        }
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                format!("{{{} keys}}", obj.len())
            }
        }
    }
}

