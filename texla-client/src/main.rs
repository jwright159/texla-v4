use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use std::io::{self, stdout, Write};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;
use tracing::{debug, warn};
use tracing_subscriber::EnvFilter;
use tungstenite::{connect, Message};

fn main() {
    crossterm::terminal::enable_raw_mode().unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let (mut socket, response) = connect("ws://localhost:8080/socket").expect("Can't connect");
    match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => {
            stream.set_nonblocking(true).unwrap();
        }
        _ => warn!("Can't set nonblocking mode for TLS stream"),
    }

    debug!("Connected to the server");
    debug!("Response HTTP code: {}", response.status());
    debug!("Response contains the following headers:");
    for (header, _value) in response.headers() {
        debug!("* {header}");
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || 'thread: loop {
        loop {
            match rx.try_recv() {
                Ok(msg @ Message::Text(_)) => {
                    socket.send(msg).expect("Can't send message");
                }
                Ok(Message::Close(_)) | Err(TryRecvError::Disconnected) => {
                    socket.close(None).expect("Can't close");
                }
                Ok(msg) => {
                    warn!("Sent unsupported message type: {msg}");
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        loop {
            match socket.read() {
                Ok(Message::Text(msg)) => {
                    execute!(io::stdout(), Clear(ClearType::CurrentLine)).unwrap();
                    println!("\r{msg}");
                }
                Ok(msg) => {
                    warn!("Received unsupported message type: {msg}");
                }
                Err(tungstenite::Error::ConnectionClosed) => {
                    warn!("Connection closed");
                    break 'thread;
                }
                Err(tungstenite::Error::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    warn!("Error: {e}");
                    socket.close(None).expect("Can't close");
                }
            }
        }
    });

    loop {
        use crossterm::event::{poll, read, Event, KeyCode};
        let mut input = String::new();

        loop {
            execute!(io::stdout(), Clear(ClearType::CurrentLine)).unwrap();
            print!("\r> {input}");
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
                            crossterm::terminal::disable_raw_mode().unwrap();
                            println!();
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }

        input = input.trim().to_owned();
        tx.send(Message::Text(input.clone()))
            .expect("Can't send message");
        input.clear();
    }
}
