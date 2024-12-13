use std::io::{stdout, Write};
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{poll, read, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::EnterAlternateScreen;
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

        self.draw_borders();

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

            let mut resized = false;
            if poll(Duration::from_millis(50)).unwrap() {
                match read().unwrap() {
                    Event::Key(event) => match event.code {
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
                    },
                    Event::Resize(_, _) => {
                        resized = true;
                    }
                    _ => {}
                }
            }

            if resized {
                self.draw_borders();
            }
            self.draw();
        }

        crossterm::terminal::disable_raw_mode().unwrap();
        execute!(stdout(), crossterm::terminal::LeaveAlternateScreen).unwrap();
    }

    fn draw_borders(&self) {
        let (width, height) = crossterm::terminal::size().unwrap();
        let mut stdout = stdout();

        for x in 1..width - 1 {
            execute!(stdout, MoveTo(x, 0)).unwrap();
            write!(stdout, "═").unwrap();
            execute!(stdout, MoveTo(x, height - 3)).unwrap();
            write!(stdout, "─").unwrap();
            execute!(stdout, MoveTo(x, height - 1)).unwrap();
            write!(stdout, "═").unwrap();
        }

        for y in 1..height - 1 {
            if y == height - 3 {
                continue;
            }

            execute!(stdout, MoveTo(0, y)).unwrap();
            write!(stdout, "║ ").unwrap();
            execute!(stdout, MoveTo(width - 2, y)).unwrap();
            write!(stdout, " ║").unwrap();
        }

        execute!(stdout, MoveTo(0, 0)).unwrap();
        write!(stdout, "╔").unwrap();
        execute!(stdout, MoveTo(width - 1, 0)).unwrap();
        write!(stdout, "╗").unwrap();
        execute!(stdout, MoveTo(0, height - 3)).unwrap();
        write!(stdout, "╟").unwrap();
        execute!(stdout, MoveTo(width - 1, height - 3)).unwrap();
        write!(stdout, "╢").unwrap();
        execute!(stdout, MoveTo(0, height - 1)).unwrap();
        write!(stdout, "╚").unwrap();
        execute!(stdout, MoveTo(width - 1, height - 1)).unwrap();
        write!(stdout, "╝").unwrap();
    }

    fn draw(&self) {
        let (width, height) = crossterm::terminal::size().unwrap();
        let mut stdout = stdout();

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
                parts.into_iter().map(|part| {
                    let mut part = part.to_owned();
                    while part.graphemes(true).count() < width {
                        part.push(' ');
                    }
                    part
                })
            })
            .collect::<Vec<_>>()
    }
}
