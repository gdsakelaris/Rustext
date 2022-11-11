use crossterm::event::{Event, KeyCode, KeyEvent}; //::*;
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, terminal};
use std::time::Duration;
// obtains user input
use std::io;
use std::io::stdout;
use std::io::Read;

struct RawFix;

impl Drop for RawFix {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

// Struct for handling text output:
struct Output;

impl Output {
    fn new() -> Self {
        Self
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
        // execute!(stdout(), terminal::Clear(ClearType::All))
    }

    fn refresh_screen(&self) -> crossterm::Result<()> {
        Self::clear_screen()
    }
}

// Struct that reads keypresses:
struct KeypressReader;
// Method that reads key events:
impl KeypressReader {
    // read_key function:
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
    fn process_keypress(&self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => return Ok(false),
            _ => {}
        }
        Ok(true)
    }
    // run function:
    fn run(&self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_keypress()
    }
}

fn main() -> crossterm::Result<()> {
    let _raw_fix = RawFix;
    terminal::enable_raw_mode()?;
    let editor = RustextEditor::new();
    while editor.run()? {}
    Ok(())
}
// loop {
//     if event::poll(Duration::from_millis(1000))? {
//         if let Event::Key(event) = event::read()? {
//             match event {
//                 KeyEvent {
//                     code: KeyCode::Char('q'),
//                     modifiers: event::KeyModifiers::CONTROL,
//                 } => break,
//                 _ => {
//                     //todo
//                 }
//             }
//             println!("{:?}\r", event);
//         };
//     } else {
//         println!("No input received\r");
//     }
// }
// // read 1 byte at a time
// let mut buf = [0; 1];
// while io::stdin().read(&mut buf).expect("Could not read line") == 1 && buf != [b'q'] {
//     let character = buf[0] as char;
//     // tests whether a character is a control character
//     if character.is_control() {
//         println!("{}\r", character as u8)
//         // println!("{}", character as u8)
//     } else {
//         println!("{}\r", character)
//         // println!("{}", character)
//     }
// }
// // panic!();
// // terminal::disable_raw_mode().expect("Could not turn off raw mode");
