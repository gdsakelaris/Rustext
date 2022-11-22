use crossterm::event::*;
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, queue, terminal};
use std::cmp::Ordering;
use std::io::stdout;
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::{cmp, env, fs};

const TAB_STOP: usize = 8;

struct RawFix;
impl Drop for RawFix {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode");
        // function to clear the screen when our program exits, either successfully or not
        Output::clear_screen().expect("Error");
    }
}

#[derive(Default)] // implements default method for Row struct
struct Row {
    row_content: String,
    render: String,
}

impl Row {
    fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
        }
    }

    // Inserts a character into the line at a given position
    fn insert_char(&mut self, at: usize, ch: char) {
        self.row_content.insert(at, ch);
        EditorRows::render_row(self)
    }

    // Deletes a character in row_content
    fn delete_char(&mut self, at: usize) {
        self.row_content.remove(at);
        EditorRows::render_row(self)
    }
}

struct EditorRows {
    row_contents: Vec<Row>,
    // bb
    filename: Option<PathBuf>,
}
impl EditorRows {
    fn new() -> Self {
        let mut arg = env::args();

        match arg.nth(1) {
            None => Self {
                row_contents: Vec::new(),
                // bb
                filename: None,
            },
            // Some(file) => Self::from_file(file.as_ref()),
            // bb ^
            Some(file) => Self::from_file(file.into()),
            //, syntax_highlight),
        }
    }

    // // fn from_file(file: &Path) -> Self {
    // fn from_file(file: PathBuf) -> Self {
    //     let file_contents = fs::read_to_string(file).expect("Cannot read file");
    //     Self {
    //         filename: Some(file.to_path_buf()),
    //         // bb ^
    //         row_contents: file_contents
    //             .lines()
    //             .map(|it| {
    //                 let mut row = Row::new(it.into(), String::new());
    //                 Self::render_row(&mut row);
    //                 row
    //             })
    //             .collect(),
    //     }
    // }
    // bb ^
    fn from_file(file: PathBuf) -> Self {
        // , syntax_highlight: &mut Option<Box<dyn SyntaxHighlight>> ^2nd param here
        let file_contents = fs::read_to_string(&file).expect("Unable to read file");
        let mut row_contents = Vec::new();
        // file.extension()
        //     .and_then(|ext| ext.to_str())
        //     .map(|ext| Output::select_syntax(ext).map(|syntax| syntax_highlight.insert(syntax)));
        file_contents.lines().enumerate().for_each(|(i, line)| {
            let mut row = Row::new(line.into(), String::new());
            Self::render_row(&mut row);
            row_contents.push(row);
            // if let Some(it) = syntax_highlight {
            //     it.update_syntax(i, &mut row_contents)
            // }
        });
        Self {
            filename: Some(file),
            row_contents,
        }
    }

    fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }

    fn get_editor_row(&self, at: usize) -> &Row {
        &self.row_contents[at]
    }

    fn number_of_rows(&self) -> usize {
        self.row_contents.len()
    }

    fn get_row(&self, at: usize) -> &str {
        &self.row_contents[at].render
    }
    // fn get_row(&self, at: usize) -> &Row {
    //     &self.row_contents[at]
    // }

    // if user presses Backspace at the beginning of a line, append contents of current line to the end of the previous line and delete the current line
    fn join_adjacent_rows(&mut self, at: usize) {
        let current_row = self.row_contents.remove(at);
        let previous_row = self.get_editor_row_mut(at - 1);
        previous_row.row_content.push_str(&current_row.row_content);
        Self::render_row(previous_row);
    }

    // function which adds a new empty row to the file where characters can be inserted
    fn insert_row(&mut self, at: usize, contents: String) {
        let mut new_row = Row::new(contents, String::new());
        EditorRows::render_row(&mut new_row);
        self.row_contents.insert(at, new_row);
    }
    // fn insert_row(&mut self) {
    //     self.row_contents.push(Row::default());
    // }

    // returns a mutable reference to Row
    fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
        &mut self.row_contents[at]
    }

    fn render_row(row: &mut Row) {
        let mut index = 0;
        let capacity = row
            .row_content
            .chars()
            .fold(0, |acc, next| acc + if next == '\t' { TAB_STOP } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            index += 1;
            if c == '\t' {
                row.render.push(' ');
                while index % TAB_STOP != 0 {
                    row.render.push(' ');
                    index += 1
                }
            } else {
                row.render.push(c);
            }
        });
    }

    // Handles file-saving
    fn save(&self) -> io::Result<()> {
        match &self.filename {
            None => Err(io::Error::new(ErrorKind::Other, "no file name specified")),
            // convert row_contents from Vec to string
            Some(name) => {
                let mut file = fs::OpenOptions::new().write(true).create(true).open(name)?;
                let contents: String = self
                    .row_contents
                    .iter()
                    .map(|it| it.row_content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n");
                file.set_len(contents.len() as u64)?;
                file.write_all(contents.as_bytes())
            }
        }
    }
}

// CursorController: STORES POSITION OF CURSOR
struct CursorController {
    cursor_x: usize,
    cursor_y: usize,
    screen_columns: usize,
    screen_lines: usize,
    // keeps track of what line cursor is currently at:
    row_offset: usize,
    column_offset: usize,
    render_x: usize,
}
// INITIALIZES CURSOR POSITION
impl CursorController {
    fn new(window_size: (usize, usize)) -> CursorController {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            screen_columns: window_size.0,
            screen_lines: window_size.1,
            row_offset: 0,
            column_offset: 0,
            render_x: 0,
        }
    }

    fn get_render_x(&self, row: &Row) -> usize {
        row.row_content[..self.cursor_x]
            .chars()
            .fold(0, |render_x, c| {
                if c == '\t' {
                    render_x + (TAB_STOP - 1) - (render_x % TAB_STOP) + 1
                } else {
                    render_x + 1
                }
            })
    }

    fn scroll(&mut self, editor_rows: &EditorRows) {
        self.render_x = 0;
        if self.cursor_y < editor_rows.number_of_rows() {
            self.render_x = self.get_render_x(editor_rows.get_editor_row(self.cursor_y))
        }
        self.row_offset = cmp::min(self.row_offset, self.cursor_y);
        if self.cursor_y >= self.row_offset + self.screen_lines {
            self.row_offset = self.cursor_y - self.screen_lines + 1;
        }
        self.column_offset = cmp::min(self.column_offset, self.render_x);
        if self.render_x >= self.column_offset + self.screen_columns {
            self.column_offset = self.render_x - self.screen_columns + 1;
        }
    }

    fn move_cursor(&mut self, direction: KeyCode, editor_rows: &EditorRows) {
        let number_of_rows = editor_rows.number_of_rows();
        match direction {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Left => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = editor_rows.get_row(self.cursor_y).len();
                }
            }
            KeyCode::Down => {
                if self.cursor_y < number_of_rows {
                    self.cursor_y += 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_y < number_of_rows {
                    match self.cursor_x.cmp(&editor_rows.get_row(self.cursor_y).len()) {
                        Ordering::Less => self.cursor_x += 1,
                        Ordering::Equal => {
                            self.cursor_y += 1;
                            self.cursor_x = 0
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::End => {
                if self.cursor_y < number_of_rows {
                    self.cursor_x = editor_rows.get_row(self.cursor_y).len();
                }
            }
            // self.cursor_x = self.screen_columns - 1,
            KeyCode::Home => self.cursor_x = 0,
            _ => unimplemented!(),
        }
        let row_len = if self.cursor_y < number_of_rows {
            editor_rows.get_row(self.cursor_y).len()
        } else {
            0
        };
        self.cursor_x = cmp::min(self.cursor_x, row_len);
    }
}
// Struct for handling text output
struct Output {
    window_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
    editor_rows: EditorRows,
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
            editor_rows: EditorRows::new(),
        }
    }
    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller
            .move_cursor(direction, &self.editor_rows);
    }
    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        // Position cursor to top left of window:
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    // 1.
    fn insert_char(&mut self, ch: char) {
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
            self.editor_rows
                .insert_row(self.editor_rows.number_of_rows(), String::new());
        }
        self.editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y)
            .insert_char(self.cursor_controller.cursor_x, ch);
        self.cursor_controller.cursor_x += 1;
    }

    // Performs various checks based on function of the same name implemented by Row struct
    fn delete_char(&mut self) {
        // if cursor is past EOF, there is nothing to delete, so returns immediately
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
            return;
        }
        // otherwise, find the line the cursor is on
        let row = self
            .editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y);
        // if there is a character to the left of the cursor, it gets deleted and cursor moves one space to the left
        if self.cursor_controller.cursor_x > 0 {
            row.delete_char(self.cursor_controller.cursor_x - 1);
            self.cursor_controller.cursor_x -= 1;
        } else {
            let previous_row_content = self
                .editor_rows
                .get_row(self.cursor_controller.cursor_y - 1);
            self.cursor_controller.cursor_x = previous_row_content.len();
            self.editor_rows
                .join_adjacent_rows(self.cursor_controller.cursor_y);
            self.cursor_controller.cursor_y -= 1;
        }
        // self.dirty += 1;
    }

    // 1... (mapped to "Enter" key)
    fn insert_newline(&mut self) {
        if self.cursor_controller.cursor_x == 0 {
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y, String::new())
        } else {
            let current_row = self
                .editor_rows
                .get_editor_row_mut(self.cursor_controller.cursor_y);
            let new_row_content = current_row.row_content[self.cursor_controller.cursor_x..].into();
            current_row
                .row_content
                .truncate(self.cursor_controller.cursor_x);
            EditorRows::render_row(current_row);
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y + 1, new_row_content);
        }
        self.cursor_controller.cursor_x = 0;
        self.cursor_controller.cursor_y += 1;
        // self.dirty += 1;
    }

    // DRAW LINES () #r
    // Adds line numbers to the beginning of each line
    fn draw_lines(&mut self) {
        let screen_lines = self.window_size.1;
        let screen_columns = self.window_size.0;
        for r in 1..screen_lines + 1 {
            let mut i = r - 1;
            let mut istr = format!("{}", i);
            ///////////////////////////////////////////////
            let file_row = i + self.cursor_controller.row_offset;
            if file_row >= self.editor_rows.number_of_rows() {
                //////    ///////////////////////////////////////////////
                if self.editor_rows.number_of_rows() == 0 && r == 1 {
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
            /////////////////////////////////////////////////////////
            } else {
                let row = self.editor_rows.get_render(file_row);
                let column_offset = self.cursor_controller.column_offset;
                // let len = cmp::min(self.editor_rows.get_row(file_row).len(), screen_columns);
                let len = cmp::min(row.len().saturating_sub(column_offset), screen_columns);
                let start = if len == 0 { 0 } else { column_offset };
                // self.editor_contents
                //     .push_str(&self.editor_rows.get_row(file_row)[..len])
                self.editor_contents.push_str(&row[start..start + len])
            }
            /////////////////////////////////////////////////////////
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
        self.cursor_controller.scroll(&self.editor_rows);
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))?;
        self.draw_lines();
        let cursor_x = self.cursor_controller.render_x - self.cursor_controller.column_offset;
        let cursor_y = self.cursor_controller.cursor_y - self.cursor_controller.row_offset;
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
            } => {
                if matches!(val, KeyCode::PageUp) {
                    self.output.cursor_controller.cursor_y =
                        self.output.cursor_controller.row_offset
                } else {
                    self.output.cursor_controller.cursor_y = cmp::min(
                        self.output.window_size.1 + self.output.cursor_controller.row_offset - 1,
                        self.output.editor_rows.number_of_rows(),
                    );
                }
                (2..self.output.window_size.1).for_each(|_| {
                    self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    });
                    // }),
                })
            }
            // Use CTRL + S to save file
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
            } => self.output.editor_rows.save()?,
            // maps "Backspace" and "Delete" keys to delete_char() function:
            KeyEvent {
                code: key @ (KeyCode::Backspace | KeyCode::Delete),
                modifiers: KeyModifiers::NONE,
            } => {
                if matches!(key, KeyCode::Delete) {
                    self.output.move_cursor(KeyCode::Right)
                }
                self.output.delete_char()
            }
            // Maps "Enter" key to insert_newline() function:
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            } => self.output.insert_newline(),
            // 1.
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            } => self.output.insert_char(match code {
                KeyCode::Tab => '\t',
                KeyCode::Char(ch) => ch,
                _ => unreachable!(),
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
