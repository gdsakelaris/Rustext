use crossterm::event::*;
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, queue, terminal};
use std::io::stdout;
use std::io::{self, Write};
use std::time::Duration;

struct RawFix;

impl Drop for RawFix {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode");
        // function to clear the screen when our program exits either successfully or not
        Output::clear_screen().expect("Error");
    }
}
// Cursor Controller (stores cursor position)
struct CursorController {
    cursor_x: usize,
    cursor_y: usize,
    screen_columns: usize,
    screen_lines: usize,
}
impl CursorController {
    fn new(window_size: (usize, usize)) -> CursorController {
        Self {
            cursor_x: 4,
            cursor_y: 1,
            screen_columns: window_size.0,
            screen_lines: window_size.1,
        }
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        match direction {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Left => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_y != self.screen_lines - 1 {
                    self.cursor_y += 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_x != self.screen_columns - 1 {
                    self.cursor_x += 1;
                }
            }
            KeyCode::End => self.cursor_x = self.screen_columns - 1,
            KeyCode::Home => self.cursor_x = 4,
            _ => unimplemented!(),
        }
        //
    }
}

// OUTPUT
// Struct for handling text output
struct Output {
    window_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
}

impl Output {
    fn new() -> Self {
        // window_size = size of terminal window
        let window_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap();
        Self {
            window_size,
            editor_contents: EditorContents::new(),
            //
            // cursor_controller: CursorController::new(),
            //
            cursor_controller: CursorController::new(window_size),
        }
    }
    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller.move_cursor(direction);
    }
    // clear_screen:
    fn clear_screen() -> crossterm::Result<()> {
        // clear_screen function writes escape sequences to the terminal
        // x1b is the escape character (similar to pressing ESC on the keyboard)
        // it is followed by [
        // The J command (Erase In Display) to clear the screen.
        // Escape sequence command takes arguments, which come before the command.
        // Argument = 2: clear the entire screen
        // <esc>[1J: clear the screen up to where the cursor is
        // <esc>[0J: clear the screen from the cursor to the end of the screen
        // 0 is the default argument for J
        // <esc>[J: ALSO clear the screen from the cursor to the end
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        // Position cursor to top left of window:
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    // DRAW LINES () #r
    // Adds line numbers to the beginning of each line
    fn draw_lines(&mut self) {
        let screen_lines = self.window_size.1;
        let screen_columns = self.window_size.0;
        for r in 1..screen_lines + 1 {
            let mut i = r - 1;
            let mut istr = format!("{}", i);
            if r == 1 {
                let mut welcome = format!("Rustext - Version 363");
                if welcome.len() > screen_columns {
                    welcome.truncate(screen_columns);
                }
                let mut padding = (screen_columns - welcome.len()) / 2;
                if padding != 0 {
                    if i != 0 {
                        self.editor_contents.push_str(&istr);
                    }
                    padding -= 1
                }
                (0..padding).for_each(|_| self.editor_contents.push(' '));
                self.editor_contents.push_str(&welcome);
            } else {
                if i != 0 {
                    self.editor_contents.push_str(&istr);
                }
            }
            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            // exception for last line in window:
            if r < screen_lines {
                self.editor_contents.push_str("\r\n");
            }
        }
    }

    // REFRESH SCREEN ()
    fn refresh_screen(&mut self) -> crossterm::Result<()> {
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))?;
        self.draw_lines();
        let cursor_x = self.cursor_controller.cursor_x;
        let cursor_y = self.cursor_controller.cursor_y;
        queue!(
            self.editor_contents,
            cursor::MoveTo(cursor_x as u16, cursor_y as u16),
            cursor::Show
        )?;
        self.editor_contents.flush()
    }
}

// STRUCT: EditorContents
// Output is written to this struct instead of to stdout
struct EditorContents {
    content: String,
}
impl EditorContents {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }
    // POTENTIAL ERROR HERE
    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}
// Implementation of std::io::Write for EditorContent:
impl io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // convert the bytes passed into the write function to str so they can be added to the content
        match std::str::from_utf8(buf) {
            // return the length of the string if the bytes can be converted to str
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            // return error otherwise
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}

// Struct that reads keypresses:
struct KeypressReader;
// Method that reads key events:
impl KeypressReader {
    fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if event::poll(Duration::from_millis(500))? {
                if let Event::Key(event) = event::read()? {
                    return Ok(event);
                }
            }
        }
    }
}

// Main stuct that runs program:
struct RustextEditor {
    reader: KeypressReader,
    output: Output,
}
impl RustextEditor {
    // new method creates new instance of RustextEditor
    fn new() -> Self {
        Self {
            reader: KeypressReader,
            output: Output::new(),
        }
    }
    // Processes the events returned by KeypressReader:
    fn process_keypress(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => return Ok(false),
            KeyEvent {
                code:
                    direction @ (KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End),
                modifiers: KeyModifiers::NONE,
            } => self.output.move_cursor(direction),
            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: KeyModifiers::NONE,
            } => (2..self.output.window_size.1).for_each(|_| {
                self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
                    KeyCode::Up
                } else {
                    KeyCode::Down
                });
            }),
            _ => {}
        }
        Ok(true)
    }
    // run function:
    fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_keypress()
    }
}

fn main() -> crossterm::Result<()> {
    let _raw_fix = RawFix;
    terminal::enable_raw_mode()?;
    let mut editor = RustextEditor::new();
    while editor.run()? {}
    Ok(())
}
