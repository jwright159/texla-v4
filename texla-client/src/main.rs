use std::io::{stdout, Write};
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use crossterm::event::{poll, read, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::{Clear, ClearType};
use itertools::Itertools;
use texla_client::{run, Output};
use unicode_segmentation::UnicodeSegmentation;

fn main() {
    crossterm::terminal::enable_raw_mode().unwrap();

    let (ev_in_tx, ev_in_rx) = std::sync::mpsc::channel();
    let (ev_out_tx, ev_out_rx) = std::sync::mpsc::channel();
    let (stopped_tx, stopped_rx) = std::sync::mpsc::channel();

    thread::spawn(move || {
        run(ev_in_rx, ev_out_tx);
        stopped_tx.send(()).unwrap();
    });

    'main: loop {
        let mut input = String::new();

        loop {
            loop {
                match ev_out_rx.try_recv() {
                    Ok(msg) => match msg {
                        Output::Text(msg) => {
                            execute!(stdout(), Clear(ClearType::CurrentLine)).unwrap();
                            println!("\r{}", format_output(msg));
                        }
                        Output::Warning(msg) => {
                            execute!(stdout(), Clear(ClearType::CurrentLine)).unwrap();
                            println!("\r{}", format_output(msg).yellow());
                        }
                    },
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        break 'main;
                    }
                }
            }

            if stopped_rx.try_recv().is_ok() {
                break 'main;
            }

            execute!(stdout(), Clear(ClearType::CurrentLine)).unwrap();
            print!("\r> {}", input.clone().dark_yellow());
            stdout().flush().unwrap();

            if poll(Duration::from_millis(50)).unwrap() {
                if let Event::Key(event) = read().unwrap() {
                    match event.code {
                        KeyCode::Char('\n') | KeyCode::Enter => {
                            println!();
                            break;
                        }
                        KeyCode::Char(c) => input.push(c),
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Esc => {
                            break 'main;
                        }
                        _ => {}
                    }
                }
            }
        }

        input = input.trim().to_owned();
        ev_in_tx.send(input.clone()).expect("Can't send message");
        input.clear();
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    println!();
}

fn format_output(msg: String) -> String {
    let width = crossterm::terminal::size().unwrap().0 as usize;
    let mut parts = msg.split("\n").flat_map(|part| {
        let mut part = part.to_owned();
        let mut parts = Vec::new();
        while part.graphemes(true).count() > width {
            let mut index = width;
            while index > 0
                && !char::is_whitespace(
                    part.graphemes(true)
                        .nth(index)
                        .unwrap()
                        .chars()
                        .next()
                        .unwrap(),
                )
            {
                index -= 1;
            }
            if index == 0 {
                index = width;
            }
            parts.push(
                part.graphemes(true)
                    .take(index)
                    .join("")
                    .trim_end()
                    .to_owned(),
            );
            part = part
                .graphemes(true)
                .skip(index)
                .join("")
                .trim_start()
                .to_owned();
        }
        if !part.is_empty() {
            parts.push(part.to_owned());
        }
        parts
    });
    parts.join("\r\n")
}
