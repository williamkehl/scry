use crate::utils;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use serde_json::Value;

#[derive(Clone)]
pub enum ViewKind {
    Plain,
    KeyValue,
    Json,
    ExternalTool(String), // Name of external tool (e.g., "jless", "visidata")
}

impl ViewKind {
    pub fn name(&self) -> String {
        match self {
            ViewKind::Plain => "Plain".to_string(),
            ViewKind::KeyValue => "KeyValue".to_string(),
            ViewKind::Json => "Json".to_string(),
            ViewKind::ExternalTool(name) => format!("External: {}", name),
        }
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        app_state: &crate::app::AppState,
    ) {
        match self {
            ViewKind::Plain => PlainView::render(f, area, app_state),
            ViewKind::KeyValue => KeyValueView::render(f, area, app_state),
            ViewKind::Json => JsonView::render(f, area, app_state),
            ViewKind::ExternalTool(name) => {
                // For external tools, show a message that it will launch
                // The actual tool will be spawned separately
                ExternalToolView::render(f, area, name);
            }
        }
    }
}

pub struct PlainView;

impl PlainView {
    pub fn render(
        f: &mut Frame,
        area: Rect,
        app_state: &crate::app::AppState,
    ) {
        let display_logs = app_state.get_display_logs();
        let display_count = display_logs.len();
        
        // Ensure scroll_offset is valid
        let scroll_offset = if display_count > 0 {
            app_state.scroll_offset.min(display_count.saturating_sub(1))
        } else {
            0
        };
        
        // Create items for all display_logs (ratatui List handles scrolling internally)
        let items: Vec<ListItem> = display_logs
            .iter()
            .enumerate()
            .map(|(_display_idx, (original_idx, line))| {
                // Sanitize line for safe display
                let safe_line = utils::safe_string_display(line);
                
                // Highlight if selected or matches filter
                let style = if app_state.selected_index == Some(*original_idx) {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else if !app_state.filtered_indices.is_empty() {
                    // Highlight filtered matches
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                
                // Highlight filter text in the line if filtering
                let content = if let Some(ref filter) = app_state.filter_text {
                    highlight_filter_text(&safe_line, filter, style)
                } else {
                    Line::from(Span::styled(safe_line.clone(), style))
                };
                
                ListItem::new(content)
            })
            .collect();

        let title = if let Some(ref filter) = app_state.filter_text {
            format!("Log Lines (filtered: '{}', {} matches)", filter, app_state.filtered_indices.len())
        } else {
            "Log Lines".to_string()
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::White));

        // Create list_state with current scroll_offset
        let display_count = display_logs.len();
        let selected_idx = if display_count > 0 {
            Some(scroll_offset.min(display_count.saturating_sub(1)))
        } else {
            None
        };
        
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(selected_idx);
        f.render_stateful_widget(list, area, &mut list_state);
    }
}

fn highlight_filter_text(line: &str, filter: &str, base_style: Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut remaining = line;
    
    while let Some(pos) = remaining.find(filter) {
        // Add text before match
        if pos > 0 {
            spans.push(Span::styled(remaining[..pos].to_string(), base_style));
        }
        // Add highlighted match
        spans.push(Span::styled(
            remaining[pos..pos + filter.len()].to_string(),
            Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(ratatui::style::Modifier::BOLD),
        ));
        remaining = &remaining[pos + filter.len()..];
    }
    // Add remaining text
    if !remaining.is_empty() {
        spans.push(Span::styled(remaining.to_string(), base_style));
    }
    
    if spans.is_empty() {
        Line::from(Span::styled(line.to_string(), base_style))
    } else {
        Line::from(spans)
    }
}

pub struct KeyValueView;

impl KeyValueView {
    pub fn render(
        f: &mut Frame,
        area: Rect,
        app_state: &crate::app::AppState,
    ) {
        let display_logs = app_state.get_display_logs();
        let mut rows = Vec::new();

        for (_display_idx, (original_idx, line)) in display_logs.iter().enumerate() {
            // Skip items before scroll_offset (for virtual scrolling if needed)
            // For now, show all items and let ratatui handle scrolling
            // But we'll highlight the selected/filtered items
            
            // Safely extract key-value pairs - handles edge cases
            let pairs = utils::extract_key_value_pairs(line);

            if !pairs.is_empty() {
                // Highlight if selected or matches filter
                let base_style = if app_state.selected_index == Some(*original_idx) {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else if !app_state.filtered_indices.is_empty() {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                
                let cells: Vec<Span> = pairs
                    .iter()
                    .flat_map(|(k, v)| {
                        // Highlight filter text in values if filtering
                        let (k_style, v_style) = if let Some(ref filter) = app_state.filter_text {
                            if k.contains(filter) || v.contains(filter) {
                                (Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), base_style)
                            } else {
                                (base_style, base_style)
                            }
                        } else {
                            (base_style, base_style)
                        };
                        
                        vec![
                            Span::styled(
                                format!("{}: ", k),
                                k_style,
                            ),
                            Span::styled(
                                format!("{} ", v),
                                v_style,
                            ),
                        ]
                    })
                    .collect();
                rows.push(Row::new(cells));
            } else {
                // Fallback: show the sanitized raw line
                let safe_line = utils::safe_string_display(line);
                
                // Highlight if selected or matches filter
                let style = if app_state.selected_index == Some(*original_idx) {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else if !app_state.filtered_indices.is_empty() {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                
                // Highlight filter text if filtering
                let content_spans: Vec<Span> = if let Some(ref filter) = app_state.filter_text {
                    // Extract spans from the Line
                    highlight_filter_text(&safe_line, filter, style)
                        .spans
                        .iter()
                        .cloned()
                        .collect()
                } else {
                    vec![Span::styled(safe_line.clone(), style)]
                };
                
                rows.push(Row::new(content_spans));
            }
        }

        let title = if let Some(ref filter) = app_state.filter_text {
            format!("Key-Value Pairs (filtered: '{}', {} matches)", filter, app_state.filtered_indices.len())
        } else {
            "Key-Value Pairs".to_string()
        };

        if rows.is_empty() {
            let msg = Paragraph::new("No key-value pairs found")
                .block(Block::default().borders(Borders::ALL).title(title))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(msg, area);
        } else {
            let table = Table::new(rows, &[Constraint::Percentage(100)])
                .block(Block::default().borders(Borders::ALL).title(title));

            f.render_widget(table, area);
        }
    }
}

pub struct JsonView;

impl JsonView {
    pub fn render(
        f: &mut Frame,
        area: Rect,
        app_state: &crate::app::AppState,
    ) {
        let display_logs = app_state.get_display_logs();
        let mut rows = Vec::new();

        for (_display_idx, (original_idx, line)) in display_logs.iter().enumerate() {
            // Determine base style for this log entry
            let is_selected = app_state.selected_index == Some(*original_idx);
            let base_key_style = if is_selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else if !app_state.filtered_indices.is_empty() {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };
            
            let base_value_style = if is_selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else if !app_state.filtered_indices.is_empty() {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            
            // Try to parse JSON - handle any errors gracefully
            match serde_json::from_str::<Value>(line) {
                Ok(json) => {
                    if let Value::Object(map) = json {
                        // Create one row per key-value pair for better readability
                        // Show ALL keys - don't limit, be resilient and show all data
                        for (key, value) in map.iter() {
                            // Sanitize key for safe display (but allow longer keys)
                            let safe_key = utils::sanitize_for_display(key, 100);
                            
                            // Check if key or value matches filter for highlighting
                            let key_matches_filter = if let Some(ref filter) = app_state.filter_text {
                                key.contains(filter)
                            } else {
                                false
                            };
                            
                            // Format value properly - show full values, truncate only if extremely long
                            let (value_str, value_matches_filter) = match value {
                                Value::String(s) => {
                                    let sanitized = utils::sanitize_for_display(s, 500);
                                    let matches = if let Some(ref filter) = app_state.filter_text {
                                        s.contains(filter)
                                    } else {
                                        false
                                    };
                                    (sanitized, matches)
                                }
                                Value::Number(n) => (n.to_string(), false),
                                Value::Bool(b) => (b.to_string(), false),
                                Value::Null => ("null".to_string(), false),
                                Value::Array(arr) => {
                                    if arr.is_empty() {
                                        ("[]".to_string(), false)
                                    } else {
                                        // Show array contents if small, otherwise count
                                        if arr.len() <= 5 {
                                            let items: Vec<String> = arr.iter()
                                                .map(|v| match v {
                                                    Value::String(s) => format!("\"{}\"", utils::sanitize_for_display(s, 50)),
                                                    Value::Number(n) => n.to_string(),
                                                    Value::Bool(b) => b.to_string(),
                                                    Value::Null => "null".to_string(),
                                                    _ => format!("{:?}", v),
                                                })
                                                .collect();
                                            (format!("[{}]", items.join(", ")), false)
                                        } else {
                                            (format!("[{} items]", arr.len()), false)
                                        }
                                    }
                                }
                                Value::Object(obj) => {
                                    if obj.is_empty() {
                                        ("{}".to_string(), false)
                                    } else {
                                        // For nested objects, show key count but also try to show some content
                                        if obj.len() <= 3 {
                                            let pairs: Vec<String> = obj.iter()
                                                .take(3)
                                                .map(|(k, v)| {
                                                    let v_str = match v {
                                                        Value::String(s) => format!("\"{}\"", utils::sanitize_for_display(s, 30)),
                                                        Value::Number(n) => n.to_string(),
                                                        Value::Bool(b) => b.to_string(),
                                                        _ => format!("{:?}", v),
                                                    };
                                                    format!("{}: {}", k, v_str)
                                                })
                                                .collect();
                                            (format!("{{{}}}", pairs.join(", ")), false)
                                        } else {
                                            (format!("{{{} keys}}", obj.len()), false)
                                        }
                                    }
                                }
                            };
                            
                            // Apply filter highlighting
                            let key_style = if key_matches_filter {
                                Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                            } else {
                                base_key_style
                            };
                            
                            let value_style = if value_matches_filter {
                                Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                            } else {
                                base_value_style
                            };
                            
                            // Create a row with key and value as separate cells
                            let key_cell = Line::from(vec![Span::styled(safe_key, key_style)]);
                            
                            // For value, highlight filter text if present
                            let value_cell = if let Some(ref filter) = app_state.filter_text {
                                if value_matches_filter {
                                    // Value contains filter - highlight the filter text within it
                                    highlight_filter_text(&value_str, filter, value_style)
                                } else {
                                    Line::from(vec![Span::styled(value_str, value_style)])
                                }
                            } else {
                                Line::from(vec![Span::styled(value_str, value_style)])
                            };
                            
                            rows.push(Row::new(vec![key_cell, value_cell]));
                        }
                    } else {
                        // Non-object JSON - display safely
                        let json_str = utils::safe_json_display(&json);
                        let json_cell = Line::from(vec![Span::styled(json_str, base_value_style)]);
                        rows.push(Row::new(vec![json_cell, Line::from("")]));
                    }
                }
                Err(_) => {
                    // Not valid JSON - silently skip (this is expected for mixed log formats)
                    continue;
                }
            }
        }

        let title = if let Some(ref filter) = app_state.filter_text {
            format!("JSON Logs (filtered: '{}', {} matches)", filter, app_state.filtered_indices.len())
        } else {
            "JSON Logs".to_string()
        };

        if rows.is_empty() {
            let msg = Paragraph::new("No valid JSON logs found")
                .block(Block::default().borders(Borders::ALL).title(title))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(msg, area);
        } else {
            // Use proper column constraints to ensure both key and value are visible
            // First column for keys (30%), second for values (70%)
            let table = Table::new(rows, &[
                Constraint::Percentage(30),  // Key column
                Constraint::Percentage(70),  // Value column
            ])
                .block(Block::default().borders(Borders::ALL).title(title));

            f.render_widget(table, area);
        }
    }
}

pub struct ExternalToolView;

impl ExternalToolView {
    pub fn render(
        f: &mut Frame,
        area: Rect,
        tool_name: &str,
    ) {
        let msg = format!(
            "External tool '{}' will be launched.\n\nLaunching in 1 second...",
            tool_name
        );
        let paragraph = Paragraph::new(msg)
            .block(Block::default().borders(Borders::ALL).title("External Tool"))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(paragraph, area);
    }
}
