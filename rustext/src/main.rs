use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::{event, terminal};
use std::time::Duration;
// obtain user input
use std::io;
use std::io::Read;

struct RawFix;

impl Drop for RawFix {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

fn main() -> crossterm::Result<()> {
    let _raw_fix = RawFix;
    terminal::enable_raw_mode()?;
    loop {
        if event::poll(Duration::from_millis(1000))? {
            if let Event::Key(event) = event::read()? {
                match event {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: event::KeyModifiers::NONE,
                    } => break,
                    _ => {
                        //todo
                    }
                }
                println!("{:?}\r", event);
            };
        } else {
            println!("No input received\r");
        }
    }
    Ok(())
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
}
