// "crossterm" crate/library contains several functions and interfaces that allow this program to interact with the computer's terminal [dependency added into "Cargo.toml" file]

use crossterm::event::*;
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, queue, style, terminal};
use std::cmp::Ordering;
use std::io::{stdout, ErrorKind, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{cmp, env, fs, io};

struct Reader;

impl Reader {
    fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            // [read] fn returns if it does not receive any input for 500 seconds
            if event::poll(Duration::from_millis(500))? {
                if let Event::Key(event) = event::read()? {
                    return Ok(event);
                }
            }
        }
    }
}

struct Editor {
    reader: Reader,
    output: Output,
}

impl Editor {
    fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
        }
    }
    // ***
    // receives user button presses and passes corresponding data along to [Output], etc.
    fn button_handler(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
        // *** Each KeyEvent corresponds to a button mapping
            // CTRL-q: quit (exit) program:
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                return Ok(false);
            }
            // CTRL-a/d: go to beginning/end of current line:
            KeyEvent {
                code:
                    direction
                    @
                    ( KeyCode::Char('a')
                    | KeyCode::Char('d')),
                modifiers: KeyModifiers::CONTROL,
            } => self.output.move_cursor(direction),
            // Arrow Keys (Up/Down/Left/Right): move the cursor 1 position in arrow direction (Standard):
            KeyEvent {
                code:
                    direction
                    @
                    (KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                ),
                modifiers: KeyModifiers::NONE,
            } => self.output.move_cursor(direction),
            // CTRL-Up/Down: go to previous/next page of file: 
            KeyEvent {
                code: val @ (KeyCode::Up | KeyCode::Down),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if matches!(val, KeyCode::Up) {
                    self.output.cursor_controller.cursor_y =
                        self.output.cursor_controller.row_offset
                } else {
                    self.output.cursor_controller.cursor_y = cmp::min(
                        self.output.win_size.1 + self.output.cursor_controller.row_offset - 1,
                        self.output.editor_rows.number_of_rows(),
                    );
                }
                (0..self.output.win_size.1).for_each(|_| {
                    self.output.move_cursor(if matches!(val, KeyCode::Up) {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    });
                })
            }
            // CTRL-s: save file:
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                // Check is the prompt is None:
                if matches!(self.output.editor_rows.filename, None) {
                    // prompts the user for a file name before saving if [filename] is None:
                    let prompt = prompt!(&mut self.output, "(Enter File Name : {}   | Press: ENTER to save / ESC to cancel save)")
                        .map(|it| it.into());
                    // if prompt is None, display "File Save Aborted":
                    if let None = prompt {
                        self.output
                            .status_message
                            .set_message("File Save Aborted".into());
                        return Ok(true);
                    }
                    self.output.editor_rows.filename = prompt
                }
                self.output.editor_rows.save().map(|len| {
                    self.output
                        .status_message
                        .set_message(format!("{:?} File Saved", self.output.editor_rows.filename));
                })?;
            }
            // [Backspace] and [DELETE] keys: Standard functionality:
            KeyEvent {
                code: key @ (KeyCode::Backspace | KeyCode::Delete),
                modifiers: KeyModifiers::NONE,
            } => {
                if matches!(key, KeyCode::Delete) {
                    self.output.move_cursor(KeyCode::Right)
                }
                self.output.delete_char()
            }
            // [ENTER]: Standard functionality:
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            // maps Enter key to [insert_newline] function - defined in [Output] implementation
            } => self.output.insert_newline(),
            // [TAB]: Standard functionality:
            // any regular character (EX: a, b, c, 1, 2, 3, ., ,, !, @, etc.) is mapped as is: 
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                // [SHIFT] button can be used as modifier:
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

    fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.button_handler()
    }
}

struct Cursor {
    cursor_x: usize,
    cursor_y: usize,
    screen_rows: usize,
    screen_columns: usize,
    row_offset: usize,
    column_offset: usize,
    render_x: usize,
}

impl Cursor {
    fn new(win_size: (usize, usize)) -> Cursor {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            screen_columns: win_size.0,
            screen_rows: win_size.1,
            row_offset: 0,
            column_offset: 0,
            render_x: 0,
        }
    }

    fn get_render_x(&self, row: &Line) -> usize {
        row.row_content[..self.cursor_x]
            .chars()
            .fold(0, |render_x, c| {
                if c == '\t' {
                    render_x + (7) - (render_x % 8) + 1
                } else {
                    render_x + 1
                }
            })
    }

    fn scroll(&mut self, editor_rows: &EditorRows) {
        self.render_x = 0;
        if self.cursor_y < editor_rows.number_of_rows() {
            self.render_x = self.get_render_x(editor_rows.get_editor_row(self.cursor_y));
        }
        self.row_offset = cmp::min(self.row_offset, self.cursor_y);
        if self.cursor_y >= self.row_offset + self.screen_rows {
            self.row_offset = self.cursor_y - self.screen_rows + 1;
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
            KeyCode::Char('d') => {
                if self.cursor_y < number_of_rows {
                    self.cursor_x = editor_rows.get_row(self.cursor_y).len();
                }
            }
            KeyCode::Char('a') => self.cursor_x = 0,
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

// Empty struct (see [Drop] implementation below)
struct Reset;

// See [main]
// allows screen clearing on crashes, panics, etc:
// [drop] fn is called: 
    // when the instance of the [Reset] struct (interpreted as [_reset] variable within [main] fn) goes out of scope when [main] returns
    // there is a [panic] while the instance is still in scope
impl Drop for Reset {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Unable to disable raw mode");
        Output::clear_screen().expect("Error");
    }
}

#[derive(Default)]
// ^ macro implements a [Default] method for [Line] struct
    // the default value creates a new instance of [Line] with [row_content] and [render] being empty strings:
struct Line {
    // Strings = mutability:
    row_content: String,
    render: String,
}

impl Line {
    fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
        }
    }

    // inserts a single character into a line, at position specified by [at] argument:
    fn insert_char(&mut self, at: usize, ch: char) {
        // [String::insert] inserts the new character:
        self.row_content.insert(at, ch);
        // [render_row] updates [render]
        EditorRows::render_row(self)
    }

    fn delete_char(&mut self, at: usize) {
        self.row_content.remove(at);
        EditorRows::render_row(self)
    }
}

// ***
// struct holding the contents of each row (line):
struct EditorRows {
    // Each line is represented as an element in [row_contents] variable
    // stored as [Vec] because contents are mutable
    row_contents: Vec<Line>,
    filename: Option<PathBuf>,
}

impl EditorRows {
    fn new() -> Self {
        match env::args().nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None,
            },
            Some(file) => Self::from_file(file.into()),
        }
    }

    fn from_file(file: PathBuf) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Could not read file");
        Self {
            filename: Some(file),
            row_contents: file_contents
                .lines()
                .map(|it| {
                    let mut row = Line::new(it.into(), String::new());
                    Self::render_row(&mut row);
                    row
                })
                .collect(),
        }
    }

    // returns the number of lines in the file:
    fn number_of_rows(&self) -> usize {
        self.row_contents.len()
    }

    fn get_row(&self, at: usize) -> &str {
        &self.row_contents[at].row_content
    }

    fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }

    fn get_editor_row(&self, at: usize) -> &Line {
        &self.row_contents[at]
    }

    fn get_editor_row_mut(&mut self, at: usize) -> &mut Line {
        &mut self.row_contents[at]
    }

    fn render_row(row: &mut Line) {
        let mut index = 0;
        let capacity = row
            .row_content
            .chars()
            .fold(0, |acc, next| acc + if next == '\t' { 8 } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            index += 1;
            if c == '\t' {
                row.render.push(' ');
                while index % 8 != 0 {
                    row.render.push(' ');
                    index += 1
                }
            } else {
                row.render.push(c);
            }
        });
    }

    // insert a row at the index specified by the [at] argument:
    fn insert_row(&mut self, at: usize, contents: String) {
        let mut new_row = Line::new(contents, String::new());
        EditorRows::render_row(&mut new_row);
        self.row_contents.insert(at, new_row);
    }

    fn save(&mut self) -> io::Result<usize> {
        match &self.filename {
            None => Err(io::Error::new(ErrorKind::Other, "File Name Not Specified")),
            Some(name) => {
                let mut file = fs::OpenOptions::new().write(true).create(true).open(name)?;
                let contents: String = self
                    .row_contents
                    .iter()
                    .map(|it| it.row_content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n");
                file.set_len(contents.len() as u64)?;
                file.write_all(contents.as_bytes())?;
                Ok(contents.as_bytes().len())
            }
        }
    }

    fn join_adjacent_rows(&mut self, at: usize) {
        let current_row = self.row_contents.remove(at);
        let previous_row = self.get_editor_row_mut(at - 1);
        previous_row.row_content.push_str(&current_row.row_content);
        Self::render_row(previous_row);
    }
}

struct EditorContents {
    content: String,
}

impl EditorContents {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}

impl io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
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

// ***
// prompts user to enter a file name when saving a new file
// uses [macros] to:
    // accept "Save as: {}"
    // fill "{}" with user input
#[macro_export]
macro_rules! prompt {
    // [prompt!()] takes 2 arguments
        // 1. an [Output] type expression
        // 2. [args]
            // is a [token tree]/[tt] type - enables macro to take format arguments
    ($output:expr,$($args:tt)*) => {{
        // 1st argument restriction:
            // only instances of Output can be passed into the macro 
        let output:&mut Output = $output;
        // user input is stored in a String:
        let mut input = String::with_capacity(32);
        // Infinite Loop:
            // i. repeatedly sets help message
            // ii. refreshes screen
            // iii. waits for buttons to handle
        loop {
            // i:
            output.status_message.set_message(format!($($args)*, input));
            // ^ [*] operator means the tokens can repeat any amount of times
            // ii:
            output.refresh_screen()?;
            // iii:
            match Reader.read_key()? {
                // if user presses Enter:
                KeyEvent {
                    code:KeyCode::Enter,
                    modifiers:KeyModifiers::NONE
                } => {
                    // if input is not empty, the help message is cleared and the input is returned
                    if !input.is_empty() {
                        output.status_message.set_message(String::new());
                        break;
                    }
                }
                // allows user to press [ESC] to cancel input prompt:
                KeyEvent {
                    code: KeyCode::Esc, ..
                } => {
                    // When the prompt is cancelled, we clear input and return None:
                    output.status_message.set_message(String::new());
                    input.clear();
                    break;
                }
                // allows user to press [backspace] and [DELETE] in the input prompt:
                KeyEvent {
                    code: KeyCode::Backspace | KeyCode::Delete,
                    modifiers: KeyModifiers::NONE,
                // ".pop()" used to remove the last character char of input
                } => { input.pop(); }
                // if a character [Char('<_>')] is input:
                KeyEvent {
                    code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                } => input.push(match code {
                        KeyCode::Tab => '\t',
                        // append the input character to [input]:
                        KeyCode::Char(ch) => ch,
                        _ => unreachable!(),
                    }),
                _=> {}
            }
        }
        // return None if there was no input 
        // or 
        // return Some(input) is there was input
        if input.is_empty() { None } else { Some (input) }
    }};
}

// Help displays useful information at the bottom of the text editor:
    // file name
    // file length (in lines)
    // current line
    // control guide
    // status updates
    // etc.
struct Help {
    message: Option<String>,
    set_time: Option<Instant>,
}

impl Help {
    fn new(initial_message: String) -> Self {
        Self {
            message: Some(initial_message),
            set_time: Some(Instant::now()),
        }
    }

    fn set_message(&mut self, message: String) {
        self.message = Some(message);
        self.set_time = Some(Instant::now())
    }

    fn message(&mut self) -> Option<&String> {
        self.set_time.and_then(|time| {
            if time.elapsed() > Duration::from_secs(300) {
                self.message = None;
                self.set_time = None;
                None
            } else {
                Some(self.message.as_ref().unwrap())
            }
        })
    }
}

struct Output {
    win_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: Cursor,
    editor_rows: EditorRows,
    status_message: Help,
}

impl Output {
    fn new() -> Self {
        let win_size = terminal::size()
            // decrement y (which represents screen_rows) so [draw_rows] fn does not attempt to draw a line of user input at the bottom of the screen:
                // makes room for two additional lines at the bottom of the screen (for Help messages)
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: Cursor::new(win_size),
            editor_rows: EditorRows::new(),
            status_message: Help::new("HELP: CTRL - [q: Quit | s: Save | a/d: Go to Beginning/End of line | Up/Down (Arrows): Page Up/Page Down]".into()),
        }
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    fn draw_message_bar(&mut self) {
        queue!(
            self.editor_contents,
            terminal::Clear(ClearType::UntilNewLine)
        )
        .unwrap();
        if let Some(msg) = self.status_message.message() {
            self.editor_contents
                .push_str(&msg[..cmp::min(self.win_size.0, msg.len())]);
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
            return;
        }
        if self.cursor_controller.cursor_y == 0 && self.cursor_controller.cursor_x == 0 {
            return;
        }
        let row = self
            .editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y);
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
    }

    // mapped to Enter key in [button_handler] struct
    fn insert_newline(&mut self) {
        // if at the beginning of a line: 
        if self.cursor_controller.cursor_x == 0 {
            self.editor_rows
                // insert a new blank row before the line the cursor is currently on:
                .insert_row(self.cursor_controller.cursor_y, String::new())
        // if not at the beginning of a line, split the current line into two rows:
        } else {
            let current_row = self
                .editor_rows
                .get_editor_row_mut(self.cursor_controller.cursor_y);
            let new_row_content = current_row.row_content[self.cursor_controller.cursor_x..].into();
            current_row
                .row_content
                // truncate the current line the cursor is on to a size equal to cursor_x:
                .truncate(self.cursor_controller.cursor_x);
            // call [render_row] to update the contents of [render]
            EditorRows::render_row(current_row);
            // insert a new row with contents of the previous line from cursor_x and on:
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y + 1, new_row_content);
        }
        // after adding a new line: 
        // set cursor_x as 0 (cursor moves to the start of the line): 
        self.cursor_controller.cursor_x = 0;
        // increase cursor_y (cursor moves down one line):
        self.cursor_controller.cursor_y += 1;
    }

    // insert a char at the cursor position
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

    fn draw_status_bar(&mut self) {
        self.editor_contents
            .push_str(&style::Attribute::Reverse.to_string());
        let info = format!(
            "{} [{} lines]",
            self.editor_rows
                .filename
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("File Not Saved"),
            self.editor_rows.number_of_rows()
        );
        let info_len = cmp::min(info.len(), self.win_size.0);
        let line_info = format!(
            "{}/{}",
            self.cursor_controller.cursor_y + 1,
            self.editor_rows.number_of_rows()
        );
        self.editor_contents.push_str(&info[..info_len]);
        for i in info_len..self.win_size.0 {
            if self.win_size.0 - i == line_info.len() {
                self.editor_contents.push_str(&line_info);
                break;
            } else {
                self.editor_contents.push(' ')
            }
        }
        self.editor_contents
            .push_str(&style::Attribute::Reset.to_string());
        self.editor_contents.push_str("\r\n");
    }

    fn draw_rows(&mut self) {
        let screen_rows = self.win_size.1;
        let screen_columns = self.win_size.0;
        for i in 0..screen_rows {
            // i_line var represents the line number of the editor (i+1):
                // its type is manipulated with format!() in order to be pushed into the row/line
            let mut i_line = format!("{}", i+1);
            let file_row = i + self.cursor_controller.row_offset;
            if file_row >= self.editor_rows.number_of_rows() {
                if self.editor_rows.number_of_rows() == 0 && i == 0 {
                    let mut welcome = format!(" ");
                    if welcome.len() > screen_columns {
                        welcome.truncate(screen_columns)
                    }
                    let mut padding = (screen_columns - welcome.len()) / 2;
                    if padding != 0 {
                        // line number added to beginning of line 1 only
                        self.editor_contents.push('1');
                        padding -= 1
                    }
                    (0..padding).for_each(|_| self.editor_contents.push(' '));
                    self.editor_contents.push_str(&welcome);
                } else {
                    // line number added to beginning of all lines after line 1
                    self.editor_contents.push_str(&i_line);
                }
            } else {
                let row = self.editor_rows.get_render(file_row);
                let column_offset = self.cursor_controller.column_offset;
                let len = cmp::min(row.len().saturating_sub(column_offset), screen_columns);
                let start = if len == 0 { 0 } else { column_offset };
                self.editor_contents.push_str(&row[start..start + len])
            }
            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            // prints a newline after the last row it draws, since [Help] struct is the final thing being drawn
            self.editor_contents.push_str("\r\n");
        }
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller
            .move_cursor(direction, &self.editor_rows);
    }

    fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.cursor_controller.scroll(&self.editor_rows);
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))?;
        self.draw_rows();
        self.draw_status_bar();
        self.draw_message_bar();
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

// returns a [Result}:
fn main() -> crossterm::Result<()> {
    // create instance of the [Reset] struct:
    let _reset = Reset;
    terminal::enable_raw_mode()?;
    // ^ [?] operator unwraps valid values or returns erroneous values and passes them to the calling function
    let mut editor = Editor::new();
    while editor.run()? {}
    Ok(())
}