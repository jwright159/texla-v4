use std::io::{stdout, Write};
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{poll, read, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen};
use itertools::Itertools;
use texla_client::{run, Output};
use unicode_segmentation::UnicodeSegmentation;

fn main() {
    Client::default().run();
}

#[derive(Debug, Default)]
struct Client {
    input: String,
    history: Vec<String>,
}

impl Client {
    fn run(&mut self) {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        crossterm::terminal::enable_raw_mode().unwrap();

        let (ev_in_tx, ev_in_rx) = std::sync::mpsc::channel();
        let (ev_out_tx, ev_out_rx) = std::sync::mpsc::channel();
        let (stopped_tx, stopped_rx) = std::sync::mpsc::channel();

        thread::spawn(move || {
            run(ev_in_rx, ev_out_tx);
            stopped_tx.send(()).unwrap();
        });

        'main: loop {
            loop {
                match ev_out_rx.try_recv() {
                    Ok(msg) => match msg {
                        Output::Text(msg) => {
                            self.history.extend(msg.split("\n").map(|s| s.to_owned()));
                        }
                        Output::Warning(msg) => {
                            self.history
                                .extend(msg.split("\n").map(|s| s.yellow().to_string()));
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

            if poll(Duration::from_millis(50)).unwrap() {
                if let Event::Key(event) = read().unwrap() {
                    match event.code {
                        KeyCode::Char('\n') | KeyCode::Enter => {
                            self.input = self.input.trim().to_owned();
                            self.history
                                .push(format!("> {}", self.input.clone().dark_yellow()));
                            ev_in_tx
                                .send(self.input.clone())
                                .expect("Can't send message");
                            self.input.clear();
                        }
                        KeyCode::Char(c) => self.input.push(c),
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Esc => {
                            break 'main;
                        }
                        _ => {}
                    }
                }
            }

            self.draw();
        }

        crossterm::terminal::disable_raw_mode().unwrap();
        execute!(stdout(), crossterm::terminal::LeaveAlternateScreen).unwrap();
    }

    fn draw(&self) {
        let size = crossterm::terminal::size().unwrap();
        let width = size.0;
        let height = size.1;

        let output_width = width - 4;
        let output_height = height - 4;

        let input = format!("> {}", self.input.clone().dark_yellow());
        let output = self
            .history
            .iter()
            .rev()
            .take(output_height as usize)
            .flat_map(|msg| {
                Self::wrap_lines(msg.clone(), output_width as usize)
                    .into_iter()
                    .rev()
            })
            .take(output_height as usize)
            .collect::<Vec<_>>();

        let mut stdout = stdout();
        execute!(stdout, Clear(ClearType::All)).unwrap();

        for x in 1..width - 1 {
            execute!(stdout, MoveTo(x, 0)).unwrap();
            write!(stdout, "─").unwrap();
            execute!(stdout, MoveTo(x, height - 3)).unwrap();
            write!(stdout, "─").unwrap();
            execute!(stdout, MoveTo(x, height - 1)).unwrap();
            write!(stdout, "─").unwrap();
        }

        for y in 1..height - 1 {
            execute!(stdout, MoveTo(0, y)).unwrap();
            write!(stdout, "│").unwrap();
            execute!(stdout, MoveTo(width - 1, y)).unwrap();
            write!(stdout, "│").unwrap();
        }

        execute!(stdout, MoveTo(0, 0)).unwrap();
        write!(stdout, "┌").unwrap();
        execute!(stdout, MoveTo(width - 1, 0)).unwrap();
        write!(stdout, "┐").unwrap();
        execute!(stdout, MoveTo(0, height - 3)).unwrap();
        write!(stdout, "├").unwrap();
        execute!(stdout, MoveTo(width - 1, height - 3)).unwrap();
        write!(stdout, "┤").unwrap();
        execute!(stdout, MoveTo(0, height - 1)).unwrap();
        write!(stdout, "└").unwrap();
        execute!(stdout, MoveTo(width - 1, height - 1)).unwrap();
        write!(stdout, "┘").unwrap();

        for (y, line) in output.into_iter().enumerate() {
            execute!(stdout, MoveTo(2, height - 4 - y as u16)).unwrap();
            write!(stdout, "{}", line).unwrap();
        }

        execute!(stdout, MoveTo(2, height - 2)).unwrap();
        write!(stdout, "{}", input).unwrap();

        stdout.flush().unwrap();
    }

    fn wrap_lines(msg: String, width: usize) -> Vec<String> {
        msg.split("\n")
            .flat_map(|part| {
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
            })
            .collect::<Vec<_>>()
    }
}
