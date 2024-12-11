use std::sync::mpsc;
use std::thread;
use tungstenite::{connect, Message};

fn main() {
    let (mut socket, response) = connect("ws://localhost:8080/socket").expect("Can't connect");
    match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => {
            stream.set_nonblocking(true).unwrap();
        }
        _ => println!("Can't set nonblocking mode for TLS stream"),
    }

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (header, _value) in response.headers() {
        println!("* {header}");
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || loop {
        loop {
            match rx.try_recv() {
                Ok(msg @ Message::Text(_)) => {
                    println!("Sent: {msg}");
                    socket.send(msg).expect("Can't send message");
                }
                Ok(Message::Close(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                    socket.close(None).expect("Can't close");
                }
                Ok(msg) => {
                    println!("Sent unsupported message type: {msg}");
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }

        loop {
            match socket.read() {
                Ok(Message::Text(msg)) => {
                    println!("Received: {msg}");
                }
                Ok(msg) => {
                    println!("Received unsupported message type: {msg}");
                }
                Err(tungstenite::Error::ConnectionClosed) => {
                    println!("Connection closed");
                    return;
                }
                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    println!("Error: {e}");
                    socket.close(None).expect("Can't close");
                }
            }
        }
    });

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        println!("Sending: {input}");
        tx.send(Message::Text(input)).expect("Can't send message");
    }
}
