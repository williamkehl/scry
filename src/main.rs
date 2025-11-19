mod app;
mod config;
mod input_source;
mod keyboard;
mod openai;
mod plugins;
mod utils;
mod views;

use app::AppState;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use clap::Parser;
use std::io::{self, BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as sync_mpsc, Arc};
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "scry")]
#[command(about = "A magical TUI log viewer that uses AI to pick the best layout")]
#[command(version)]
#[command(arg_required_else_help = false)]
struct Cli {
    /// Start the TUI (default behavior when stdin is piped)
    #[arg(short, long)]
    start: bool,
    
    /// Set the OpenAI API key
    #[arg(short = 'k', long = "key")]
    api_key: Option<String>,
    
    /// Delete the existing API key
    #[arg(short = 'd', long = "delete")]
    delete: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal on panic
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        original_hook(panic_info);
    }));

    let cli = Cli::parse();

    // Handle API key commands
    if cli.delete {
        config::delete_api_key()?;
        println!("API key deleted successfully!");
        return Ok(());
    }
    
    if let Some(api_key) = cli.api_key {
        config::set_api_key(&api_key)?;
        println!("API key saved successfully!");
        return Ok(());
    }

    // Check if stdin is piped
    let stdin_is_tty = atty::is(atty::Stream::Stdin);
    
    // If no stdin and no --start flag, show help
    if stdin_is_tty && !cli.start {
        // Show usage information with ASCII art
        println!();
        println!(" .::::::.   .,-::::: :::::::...-:.     ::-.");
        println!(" ;;;`    ` ,;;;'````' ;;;;``;;;;';;.   ;;;;'");
        println!(" '[==/[[[[,[[[         [[[,/[[['  '[[,[[['");
        println!("   '''    $$$$         $$$$$$c      c$$\"");
        println!("  88b    dP`88bo,__,o, 888b \"88bo,,8P\"`");
        println!("  \"YMmMY\"   \"YUMMMMMP\"MMMM   \"W\"mM\"");
        println!();
        println!("A magical TUI log viewer that uses AI to pick the best layout\n");
        println!("USAGE:");
        println!("    scry [OPTIONS]");
        println!("    <command> | scry");
        println!("    scry < <file>\n");
        println!("EXAMPLES:");
        println!("    tail -f app.log | scry          # View streaming logs");
        println!("    journalctl -f | scry            # View systemd logs");
        println!("    scry < app.log                  # View a log file");
        println!("    scry --start                    # Start TUI (waiting for input)\n");
        println!("COMMANDS:");
        println!("    -k, --key <API_KEY>             Set OpenAI API key");
        println!("    -d, --delete                    Delete existing API key\n");
        println!("OPTIONS:");
        println!("    -h, --help                      Print help information");
        println!("    -V, --version                   Print version information");
        println!("    -s, --start                     Start TUI even without piped input\n");
        println!("GitHub: https://github.com/williamkehl/scry");
        println!("License: Unlicense (Public Domain)");
        println!("\nFor more information, run: scry --help");
        return Ok(());
    }

    // Check if API key is set before starting TUI
    if let Err(e) = config::get_api_key() {
        eprintln!("Error: {}", e);
        eprintln!("\nTo set your API key, run: scry key YOUR_API_KEY");
        return Err(e.into());
    }

    // Run TUI with proper cleanup
    let result = run_tui(stdin_is_tty).await;

    // Ensure terminal is restored even on error/panic
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );

    result
}

async fn run_tui(stdin_is_tty: bool) -> Result<(), Box<dyn std::error::Error>> {
    // stdin_is_tty is passed as parameter to avoid re-checking
    // Check if stdout is a TTY (needed for terminal)
    let stdout_is_tty = atty::is(atty::Stream::Stdout);

    if !stdout_is_tty {
        return Err("stdout is not a TTY. scry requires a terminal to display the TUI.".into());
    }

    // Set up signal handler for Ctrl+C (works even when stdin is piped)
    let should_quit_signal = Arc::new(AtomicBool::new(false));
    let should_quit_clone = should_quit_signal.clone();
    ctrlc::set_handler(move || {
        should_quit_clone.store(true, Ordering::Relaxed);
    })?;

    // Create channel for log lines
    let (log_tx, log_rx) = mpsc::channel::<String>(1000);

    // Spawn stdin reader task BEFORE terminal setup
    let log_tx_clone = log_tx.clone();
    if stdin_is_tty {
        // No stdin, send a waiting message
        tokio::spawn(async move {
            let _ = log_tx_clone.send("Waiting for log input on stdin...".to_string()).await;
        });
    } else {
        // Read stdin in a blocking task
        // Accept ANY input - binary data, invalid UTF-8, control chars, etc.
        let tx = log_tx_clone.clone();
        tokio::task::spawn_blocking(move || {
            let stdin = io::stdin();
            let mut reader = BufReader::new(stdin.lock());
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        // Accept the line as-is, even if it contains:
                        // - Control characters
                        // - Binary data (will be lossy converted to UTF-8)
                        // - Very long lines
                        // - Empty lines
                        // - Special unicode characters
                        // The views will handle sanitization for display
                        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
                        // Accept even empty lines - they're valid log input
                        if tx.blocking_send(trimmed).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        // On read error, try to continue or break gracefully
                        // This handles cases like broken pipes, etc.
                        break;
                    }
                }
            }
        });
    }

    // Setup terminal AFTER stdin reader is spawned
    // Try to enable raw mode
    // Note: enable_raw_mode operates on stdout, so it should work even when stdin is piped
    let raw_mode_enabled = enable_raw_mode().is_ok();
    
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Detect input source
    let input_source = input_source::detect_input_source(stdin_is_tty);
    
    // Initialize app state
    let mut app_state = AppState::new(log_rx, input_source);

    // Channel for analysis results
    let (analysis_tx, mut analysis_rx) = mpsc::channel::<(views::ViewKind, String)>(10);

    // When stdin is piped, use /dev/tty for keyboard input
    let keyboard_rx = if !stdin_is_tty {
        let (tx, rx) = sync_mpsc::channel();
        let _handle = keyboard::spawn_keyboard_reader(tx)?;
        Some(rx)
    } else {
        None
    };

    // Main event loop
    let mut should_quit = false;
    while !should_quit && !should_quit_signal.load(Ordering::Relaxed) {
        // Process incoming log lines
        while let Ok(line) = app_state.log_receiver.try_recv() {
            app_state.add_log(line);
        }

        // Process analysis results
        while let Ok((view_kind, summary)) = analysis_rx.try_recv() {
            app_state.set_view(view_kind.clone());
            app_state.set_model_response(summary);
            
            // If external tool is selected, launch it
            if let views::ViewKind::ExternalTool(tool_name) = &view_kind {
                // Restore terminal before launching external tool
                let _ = disable_raw_mode();
                let _ = execute!(
                    io::stdout(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                );
                terminal.show_cursor().ok();
                
                // Launch external tool
                let registry = plugins::ToolRegistry::new();
                if let Some(tool) = registry.get(tool_name) {
                    if tool.is_available() {
                        let logs = app_state.log_buffer.clone();
                        match tool.spawn_with_logs(&logs).await {
                            Ok(_) => {
                                // Tool exited successfully, return to scry
                            }
                            Err(e) => {
                                eprintln!("\nError launching {}: {}\nPress Enter to continue...", tool_name, e);
                                let mut buf = String::new();
                                let _ = io::stdin().read_line(&mut buf);
                            }
                        }
                    } else {
                        eprintln!("\n{} is not installed. Falling back to built-in view.\nPress Enter to continue...", tool_name);
                        let mut buf = String::new();
                        let _ = io::stdin().read_line(&mut buf);
                        // Fallback to Json view
                        app_state.set_view(views::ViewKind::Json);
                    }
                }
                
                // Re-enter alternate screen and re-enable raw mode
                let _ = enable_raw_mode();
                let _ = execute!(
                    io::stdout(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                );
            }
        }

        // Draw UI
        terminal.draw(|f| ui(f, &app_state))?;

        // Handle events
        // When stdin is piped, read from /dev/tty channel; otherwise use crossterm
        if let Some(ref kb_rx) = keyboard_rx {
            // Read from /dev/tty keyboard channel
            while let Ok(event) = kb_rx.try_recv() {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => {
                                should_quit = true;
                            }
                            KeyCode::Char('a') => {
                                // Check if API key is set before analyzing
                                if !config::has_api_key() {
                                    app_state.set_model_response("API key not set. Run 'scry key YOUR_API_KEY' to set it.".to_string());
                                } else {
                                    // Show API call status
                                    app_state.set_model_response("Calling OpenAI API (gpt-4o-mini) to analyze logs...".to_string());
                                    
                                    // Trigger analysis
                                    let logs = app_state.log_buffer.clone();
                                    let tx = analysis_tx.clone();
                                    
                                    tokio::spawn(async move {
                                        match openai::analyze_logs(&logs).await {
                                            Ok((view_kind, summary)) => {
                                                let _ = tx.send((view_kind, summary)).await;
                                            }
                                            Err(e) => {
                                                let _ = tx.send((
                                                    views::ViewKind::Plain,
                                                    format!("OpenAI API error: {}", e),
                                                )).await;
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                should_quit = true;
                            }
                            KeyCode::Up => {
                                // Scroll up or move selection up
                                if let Some(selected) = app_state.selected_index {
                                    if selected > 0 {
                                        app_state.select_line(selected - 1);
                                        // Update scroll to follow selection
                                        let display_count = app_state.get_display_count();
                                        if let Some(display_idx) = app_state.filtered_indices.iter().position(|&i| i == selected - 1) {
                                            app_state.scroll_offset = display_idx;
                                        } else if app_state.filtered_indices.is_empty() {
                                            app_state.scroll_offset = (selected - 1).min(display_count.saturating_sub(1));
                                        }
                                    }
                                } else {
                                    app_state.scroll_up(1);
                                }
                            }
                            KeyCode::Down => {
                                // Scroll down or move selection down
                                if let Some(selected) = app_state.selected_index {
                                    if selected < app_state.log_buffer.len().saturating_sub(1) {
                                        app_state.select_line(selected + 1);
                                        // Update scroll to follow selection
                                        let display_count = app_state.get_display_count();
                                        if let Some(display_idx) = app_state.filtered_indices.iter().position(|&i| i == selected + 1) {
                                            app_state.scroll_offset = display_idx;
                                        } else if app_state.filtered_indices.is_empty() {
                                            app_state.scroll_offset = (selected + 1).min(display_count.saturating_sub(1));
                                        }
                                    }
                                } else {
                                    let display_count = app_state.get_display_count();
                                    app_state.scroll_down(1, display_count);
                                }
                            }
                            KeyCode::PageUp => {
                                app_state.scroll_up(10);
                            }
                            KeyCode::PageDown => {
                                let display_count = app_state.get_display_count();
                                app_state.scroll_down(10, display_count);
                            }
                            KeyCode::Home => {
                                app_state.scroll_offset = 0;
                                app_state.selected_index = None;
                            }
                            KeyCode::End => {
                                let display_count = app_state.get_display_count();
                                if display_count > 0 {
                                    app_state.scroll_offset = display_count.saturating_sub(1);
                                }
                            }
                            KeyCode::Char('f') => {
                                // Toggle filter mode - select current line
                                if app_state.selected_index.is_some() {
                                    app_state.clear_selection();
                                } else if !app_state.log_buffer.is_empty() {
                                    let idx = app_state.scroll_offset.min(app_state.log_buffer.len().saturating_sub(1));
                                    app_state.select_line(idx);
                                }
                            }
                            KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // 'c' alone clears selection/filter
                                app_state.clear_selection();
                            }
                            KeyCode::Esc => {
                                app_state.clear_selection();
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else if raw_mode_enabled {
            // Use crossterm's event system when stdin is not piped
            match crossterm::event::poll(std::time::Duration::from_millis(50)) {
                Ok(true) => {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            if key.kind == KeyEventKind::Press {
                                match key.code {
                                    KeyCode::Char('q') => {
                                        should_quit = true;
                                    }
                                    KeyCode::Char('a') => {
                                        // Check if API key is set before analyzing
                                        if !config::has_api_key() {
                                            app_state.set_model_response("API key not set. Run 'scry key YOUR_API_KEY' to set it.".to_string());
                                        } else {
                                            // Show API call status
                                            app_state.set_model_response("Calling OpenAI API (gpt-4o-mini) to analyze logs...".to_string());
                                            
                                            // Trigger analysis
                                            let logs = app_state.log_buffer.clone();
                                            let tx = analysis_tx.clone();
                                            
                                            tokio::spawn(async move {
                                                match openai::analyze_logs(&logs).await {
                                                    Ok((view_kind, summary)) => {
                                                        let _ = tx.send((view_kind, summary)).await;
                                                    }
                                                    Err(e) => {
                                                        let _ = tx.send((
                                                            views::ViewKind::Plain,
                                                            format!("OpenAI API error: {}", e),
                                                        )).await;
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                        should_quit = true;
                                    }
                                    KeyCode::Up => {
                                        if let Some(selected) = app_state.selected_index {
                                            if selected > 0 {
                                                app_state.select_line(selected - 1);
                                                let display_count = app_state.get_display_count();
                                                if let Some(display_idx) = app_state.filtered_indices.iter().position(|&i| i == selected - 1) {
                                                    app_state.scroll_offset = display_idx;
                                                } else if app_state.filtered_indices.is_empty() {
                                                    app_state.scroll_offset = (selected - 1).min(display_count.saturating_sub(1));
                                                }
                                            }
                                        } else {
                                            app_state.scroll_up(1);
                                        }
                                    }
                                    KeyCode::Down => {
                                        if let Some(selected) = app_state.selected_index {
                                            if selected < app_state.log_buffer.len().saturating_sub(1) {
                                                app_state.select_line(selected + 1);
                                                let display_count = app_state.get_display_count();
                                                if let Some(display_idx) = app_state.filtered_indices.iter().position(|&i| i == selected + 1) {
                                                    app_state.scroll_offset = display_idx;
                                                } else if app_state.filtered_indices.is_empty() {
                                                    app_state.scroll_offset = (selected + 1).min(display_count.saturating_sub(1));
                                                }
                                            }
                                        } else {
                                            let display_count = app_state.get_display_count();
                                            app_state.scroll_down(1, display_count);
                                        }
                                    }
                                    KeyCode::PageUp => {
                                        app_state.scroll_up(10);
                                    }
                                    KeyCode::PageDown => {
                                        let display_count = app_state.get_display_count();
                                        app_state.scroll_down(10, display_count);
                                    }
                                    KeyCode::Home => {
                                        app_state.scroll_offset = 0;
                                        app_state.selected_index = None;
                                    }
                                    KeyCode::End => {
                                        let display_count = app_state.get_display_count();
                                        if display_count > 0 {
                                            app_state.scroll_offset = display_count.saturating_sub(1);
                                        }
                                    }
                                    KeyCode::Char('f') => {
                                        if app_state.selected_index.is_some() {
                                            app_state.clear_selection();
                                        } else if !app_state.log_buffer.is_empty() {
                                            let idx = app_state.scroll_offset.min(app_state.log_buffer.len().saturating_sub(1));
                                            app_state.select_line(idx);
                                        }
                                    }
                                    KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                        app_state.clear_selection();
                                    }
                                    KeyCode::Esc => {
                                        app_state.clear_selection();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(_) => {} // Other events, ignore
                        Err(_) => {} // Error reading event, continue
                    }
                }
                Ok(false) => {} // No event available
                Err(_) => {} // Error polling, continue
            }
        }
    }

    // Restore terminal
    if raw_mode_enabled {
        let _ = disable_raw_mode(); // Ignore errors on cleanup
    }
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app_state: &AppState) {
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3), // Top bar
            Constraint::Min(0),    // Main area
            Constraint::Length(3), // Bottom bar
        ])
        .split(f.size());

    // Top bar
    let mut top_text = vec![
        Span::styled("scry", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::raw(format!("View: {}", app_state.active_view.name())),
        Span::raw(" | "),
        Span::styled("[a]", Style::default().fg(Color::Yellow)),
        Span::raw(" analyze "),
    ];
    
    if app_state.filter_text.is_some() {
        top_text.push(Span::styled("[f]", Style::default().fg(Color::Green)));
        top_text.push(Span::raw(" filter "));
    } else {
        top_text.push(Span::styled("[f]", Style::default().fg(Color::Yellow)));
        top_text.push(Span::raw(" filter "));
    }
    
    top_text.extend(vec![
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::raw(" nav "),
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]);
    let top_paragraph = Paragraph::new(Line::from(top_text))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    f.render_widget(top_paragraph, chunks[0]);

    // Main area - render active view
    app_state.active_view.render(f, chunks[1], app_state);

    // Bottom bar - show input source, API key status, and last model response
    let api_key_status = if config::has_api_key() {
        "API: ✓"
    } else {
        "API: ✗"
    };
    
    let status_parts = vec![
        app_state.input_source.clone(),
        api_key_status.to_string(),
    ];
    
    let status_text = if let Some(ref response) = app_state.last_model_response {
        format!("{} | {}", status_parts.join(" | "), response)
    } else {
        format!("{} | Ready", status_parts.join(" | "))
    };
    
    let status_color = if config::has_api_key() {
        Color::Green
    } else {
        Color::Yellow
    };
    
    let bottom_paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(Style::default().fg(status_color));
    f.render_widget(bottom_paragraph, chunks[2]);
}


