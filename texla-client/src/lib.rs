use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;
use tungstenite::{connect, Message};

pub fn run(ev_in: Receiver<String>, ev_out: Sender<Output>) {
    let mut socket = loop {
        if let Ok((socket, _response)) = connect("ws://localhost:8080/socket") {
            break socket;
        }
        thread::sleep(Duration::from_secs(1));
    };
    match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => {
            stream.set_nonblocking(true).unwrap();
        }
        _ => ev_out
            .send(Output::Warning(
                "Can't set nonblocking mode for TLS stream".to_owned(),
            ))
            .unwrap(),
    }

    let (tx, rx) = mpsc::channel();
    let (other_tx, other_rx) = mpsc::channel();

    thread::spawn(move || loop {
        loop {
            match rx.try_recv() {
                Ok(msg @ Message::Text(_)) => {
                    socket.send(msg).expect("Can't send message");
                }
                Ok(Message::Close(_)) | Err(TryRecvError::Disconnected) => {
                    socket.close(None).expect("Can't close");
                }
                Ok(msg) => {
                    ev_out
                        .send(Output::Warning(format!(
                            "Sent unsupported message type: {msg}"
                        )))
                        .unwrap();
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        loop {
            use tungstenite::{error::ProtocolError, Error};

            match socket.read() {
                Ok(Message::Text(msg)) => {
                    ev_out.send(Output::Text(msg)).unwrap();
                }
                Ok(msg) => {
                    ev_out
                        .send(Output::Warning(format!(
                            "Received unsupported message type: {msg}"
                        )))
                        .unwrap();
                }
                Err(Error::ConnectionClosed)
                | Err(Error::AlreadyClosed)
                | Err(Error::Protocol(ProtocolError::ResetWithoutClosingHandshake)) => {
                    println!("Connection closed");
                    other_tx
                        .send(OtherMessage::Close)
                        .expect("Can't send close message");
                    return;
                }
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    ev_out.send(Output::Warning(format!("Error: {e}"))).unwrap();
                    socket.close(None).expect("Can't close");
                }
            }
        }
    });

    loop {
        match other_rx.try_recv() {
            Ok(OtherMessage::Close) | Err(TryRecvError::Disconnected) => {
                return;
            }
            Err(TryRecvError::Empty) => {}
        }

        while let Ok(msg) = ev_in.try_recv() {
            tx.send(Message::Text(msg)).expect("Can't send message");
        }
    }
}

enum OtherMessage {
    Close,
}

pub enum Output {
    Text(String),
    Warning(String),
}
