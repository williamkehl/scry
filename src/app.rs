use crate::views::ViewKind;
use tokio::sync::mpsc;

pub struct AppState {
    pub log_buffer: Vec<String>,
    pub active_view: ViewKind,
    pub last_model_response: Option<String>,
    pub log_receiver: mpsc::Receiver<String>,
    pub input_source: String,
    // Navigation and filtering
    pub scroll_offset: usize,  // Current scroll position
    pub selected_index: Option<usize>,  // Currently selected/highlighted line index
    pub filter_text: Option<String>,  // Current filter text (from selected line)
    pub filtered_indices: Vec<usize>,  // Indices of logs matching the filter
}

impl AppState {
    pub fn new(log_receiver: mpsc::Receiver<String>, input_source: String) -> Self {
        Self {
            log_buffer: Vec::with_capacity(2000),
            active_view: ViewKind::Plain,
            last_model_response: None,
            log_receiver,
            input_source,
            scroll_offset: 0,
            selected_index: None,
            filter_text: None,
            filtered_indices: Vec::new(),
        }
    }

    pub fn add_log(&mut self, line: String) {
        // Accept any line, even if it's empty or contains weird characters
        // The views will handle sanitization for display
        let new_index = self.log_buffer.len();
        self.log_buffer.push(line);
        
        // Keep buffer capped at ~2000 lines
        if self.log_buffer.len() > 2000 {
            let removed_index = 0;
            self.log_buffer.remove(0);
            
            // Update filtered_indices: remove the old index and adjust all indices
            if !self.filtered_indices.is_empty() {
                self.filtered_indices.retain(|&idx| idx != removed_index);
                // Decrement all indices since we removed the first item
                self.filtered_indices = self.filtered_indices
                    .iter()
                    .map(|&idx| idx.saturating_sub(1))
                    .collect();
            }
            
            // Update selected_index if it was pointing to the removed item
            if let Some(selected) = self.selected_index {
                if selected == removed_index {
                    self.selected_index = None;
                } else if selected > removed_index {
                    self.selected_index = Some(selected - 1);
                }
            }
            
            // Adjust scroll_offset if needed
            if self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        } else {
            // New log was added at new_index
            // If there's an active filter, check if this new log matches
            if let Some(ref filter) = self.filter_text {
                if !filter.is_empty() && self.log_buffer[new_index].contains(filter) {
                    // Add to filtered_indices (it's already at the correct index)
                    self.filtered_indices.push(new_index);
                }
            }
        }
    }

    pub fn set_view(&mut self, view: ViewKind) {
        self.active_view = view;
    }

    pub fn set_model_response(&mut self, response: String) {
        self.last_model_response = Some(response);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        }
    }

    pub fn scroll_down(&mut self, amount: usize, max_lines: usize) {
        let max_scroll = max_lines.saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
        }
    }

    pub fn get_display_count(&self) -> usize {
        if !self.filtered_indices.is_empty() {
            self.filtered_indices.len()
        } else {
            self.log_buffer.len()
        }
    }

    pub fn select_line(&mut self, index: usize) {
        if index < self.log_buffer.len() {
            self.selected_index = Some(index);
            // Extract filter text from selected line
            let line = &self.log_buffer[index];
            // Try to extract meaningful text (word, value, etc.)
            self.filter_text = extract_filter_text(line);
            self.update_filter();
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_index = None;
        self.filter_text = None;
        self.filtered_indices.clear();
    }

    fn update_filter(&mut self) {
        if let Some(ref filter) = self.filter_text {
            if filter.is_empty() {
                self.filtered_indices.clear();
                return;
            }
            
            self.filtered_indices = self.log_buffer
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| {
                    if line.contains(filter) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();
            // Reset scroll when filter changes
            self.scroll_offset = 0;
        } else {
            self.filtered_indices.clear();
        }
    }

    pub fn get_display_logs(&self) -> Vec<(usize, &String)> {
        // Return logs with their indices, applying filter if active
        let logs_to_show: Vec<(usize, &String)> = if !self.filtered_indices.is_empty() {
            self.filtered_indices
                .iter()
                .map(|&idx| (idx, &self.log_buffer[idx]))
                .collect()
        } else {
            self.log_buffer
                .iter()
                .enumerate()
                .collect()
        };

        logs_to_show
    }
}

/// Extract meaningful text from a line for filtering
/// Tries to extract words, values, or other meaningful tokens
fn extract_filter_text(line: &str) -> Option<String> {
    // Try to extract JSON values
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start+1..].find('"') {
            let value = &line[start+1..start+1+end];
            if !value.is_empty() && value.len() < 100 {
                return Some(value.to_string());
            }
        }
    }
    
    // Try to extract key=value pairs
    if let Some(eq_pos) = line.find('=') {
        if eq_pos > 0 && eq_pos < line.len() - 1 {
            let value = line[eq_pos+1..].split_whitespace().next().unwrap_or("");
            if !value.is_empty() && value.len() < 100 {
                return Some(value.to_string());
            }
        }
    }
    
    // Extract first meaningful word (alphanumeric, at least 2 chars)
    let words: Vec<&str> = line
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphanumeric()) && w.len() >= 2)
        .collect();
    
    if let Some(word) = words.first() {
        // Clean up the word (remove punctuation at end)
        let cleaned: String = word
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if cleaned.len() >= 2 {
            return Some(cleaned);
        }
    }
    
    None
}

