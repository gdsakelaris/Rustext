# Rustext - Text Editor

## Project Description

For my project, I have created a painfully basic, keyboard-oriented, command-line text editor using the Rust programming language (I named it "Rustext" because I had no extra time to be creative).

It allows you to:

- create new files
- open existing files
- save changes to all files
- manipulate the cursor with specific controls to easily navigate files

## To run Rustext:

- From the command-line, navigate to the "rustext" directory.

  - type and enter: "cargo run" to open a new, unsaved file.\
     OR
  - type and enter: "cargo run <filename.ext>" to open the existing file.

## Program Notes

- All unique controls are displayed at the bottom of the window when relevant.

- Here are the controls with "standard" functionality:

  - Enter
  - Backspace
  - Delete
  - Tab (8 spaces)
  - Arrow Keys (Up/Down/Left/Right)
  - Shift
  - Generic character keys (EX: a, b, c, ., ,, /, etc.)
  - Generic character keys, modified by Shift key (EX: A, B, C, >, <, ?, etc.)

- Saved files are saved within the "rustext" directory.
