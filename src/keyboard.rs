use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::fs::File;
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::sync::mpsc;
use std::thread;

// Read keyboard input from /dev/tty when stdin is piped
pub fn spawn_keyboard_reader(tx: mpsc::Sender<Event>) -> io::Result<thread::JoinHandle<()>> {
    let handle = thread::spawn(move || {
        // Open /dev/tty to read from the terminal device directly
        let tty = match File::open("/dev/tty") {
            Ok(f) => f,
            Err(_) => {
                // If /dev/tty doesn't work, we can't read keyboard
                return;
            }
        };

        let fd = tty.as_raw_fd();
        
        // Set terminal to raw mode for this file descriptor
        unsafe {
            use libc::{tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSANOW};
            let mut termios: termios = std::mem::zeroed();
            if tcgetattr(fd, &mut termios) == 0 {
                let original_termios = termios;
                termios.c_lflag &= !(ICANON | ECHO);
                if tcsetattr(fd, TCSANOW, &termios) != 0 {
                    return;
                }
                
                // Read bytes from /dev/tty
                let mut tty_reader = tty;
                let mut single_byte = [0u8; 1];
                
                loop {
                    // Read first byte
                    match tty_reader.read_exact(&mut single_byte) {
                        Ok(_) => {
                            let byte = single_byte[0];
                            
                            // Check for escape sequences (arrow keys start with 0x1b = ESC)
                            if byte == 0x1b {
                                // Read next byte (should be '[')
                                let mut second_byte = [0u8; 1];
                                if tty_reader.read_exact(&mut second_byte).is_ok() && second_byte[0] == 0x5b {
                                    // Read third byte to determine which arrow key
                                    let mut third_byte = [0u8; 1];
                                    if tty_reader.read_exact(&mut third_byte).is_ok() {
                                        match third_byte[0] {
                                            0x41 => { // Up arrow [A
                                                let _ = tx.send(Event::Key(KeyEvent {
                                                    code: KeyCode::Up,
                                                    modifiers: KeyModifiers::empty(),
                                                    kind: KeyEventKind::Press,
                                                    state: crossterm::event::KeyEventState::empty(),
                                                }));
                                            }
                                            0x42 => { // Down arrow [B
                                                let _ = tx.send(Event::Key(KeyEvent {
                                                    code: KeyCode::Down,
                                                    modifiers: KeyModifiers::empty(),
                                                    kind: KeyEventKind::Press,
                                                    state: crossterm::event::KeyEventState::empty(),
                                                }));
                                            }
                                            0x35 => { // PageUp starts with [5, need one more byte
                                                let mut fourth_byte = [0u8; 1];
                                                if tty_reader.read_exact(&mut fourth_byte).is_ok() && fourth_byte[0] == 0x7e {
                                                    let _ = tx.send(Event::Key(KeyEvent {
                                                        code: KeyCode::PageUp,
                                                        modifiers: KeyModifiers::empty(),
                                                        kind: KeyEventKind::Press,
                                                        state: crossterm::event::KeyEventState::empty(),
                                                    }));
                                                }
                                            }
                                            0x36 => { // PageDown starts with [6, need one more byte
                                                let mut fourth_byte = [0u8; 1];
                                                if tty_reader.read_exact(&mut fourth_byte).is_ok() && fourth_byte[0] == 0x7e {
                                                    let _ = tx.send(Event::Key(KeyEvent {
                                                        code: KeyCode::PageDown,
                                                        modifiers: KeyModifiers::empty(),
                                                        kind: KeyEventKind::Press,
                                                        state: crossterm::event::KeyEventState::empty(),
                                                    }));
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            } else {
                                // Parse simple keypresses
                                match byte {
                                    b'q' | b'Q' => {
                                        let _ = tx.send(Event::Key(KeyEvent {
                                            code: KeyCode::Char('q'),
                                            modifiers: KeyModifiers::empty(),
                                            kind: KeyEventKind::Press,
                                            state: crossterm::event::KeyEventState::empty(),
                                        }));
                                    }
                                    b'a' | b'A' => {
                                        let _ = tx.send(Event::Key(KeyEvent {
                                            code: KeyCode::Char('a'),
                                            modifiers: KeyModifiers::empty(),
                                            kind: KeyEventKind::Press,
                                            state: crossterm::event::KeyEventState::empty(),
                                        }));
                                    }
                                    b'f' | b'F' => {
                                        let _ = tx.send(Event::Key(KeyEvent {
                                            code: KeyCode::Char('f'),
                                            modifiers: KeyModifiers::empty(),
                                            kind: KeyEventKind::Press,
                                            state: crossterm::event::KeyEventState::empty(),
                                        }));
                                    }
                                    b'c' | b'C' => {
                                        let _ = tx.send(Event::Key(KeyEvent {
                                            code: KeyCode::Char('c'),
                                            modifiers: KeyModifiers::empty(),
                                            kind: KeyEventKind::Press,
                                            state: crossterm::event::KeyEventState::empty(),
                                        }));
                                    }
                                    27 => { // ESC
                                        let _ = tx.send(Event::Key(KeyEvent {
                                            code: KeyCode::Esc,
                                            modifiers: KeyModifiers::empty(),
                                            kind: KeyEventKind::Press,
                                            state: crossterm::event::KeyEventState::empty(),
                                        }));
                                    }
                                    3 => { // Ctrl+C (ETX)
                                        let _ = tx.send(Event::Key(KeyEvent {
                                            code: KeyCode::Char('c'),
                                            modifiers: KeyModifiers::CONTROL,
                                            kind: KeyEventKind::Press,
                                            state: crossterm::event::KeyEventState::empty(),
                                        }));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                
                // Restore terminal
                let _ = tcsetattr(fd, TCSANOW, &original_termios);
            }
        }
    });
    
    Ok(handle)
}

