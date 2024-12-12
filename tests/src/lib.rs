#![cfg(test)]

use std::thread;

#[test]
fn connect_and_register() {
    thread::spawn(|| {
        texla_server::app().run();
    });

    let (ev_in_tx, ev_in_rx) = std::sync::mpsc::channel();
    let (ev_out_tx, ev_out_rx) = std::sync::mpsc::channel();

    thread::spawn(move || {
        texla_client::run(ev_in_rx, ev_out_tx);
    });

    // Give the server time to start and the client to connect
    thread::sleep(std::time::Duration::from_millis(1000));

    ev_in_tx.send("register foo | bar".to_owned()).unwrap();
    thread::sleep(std::time::Duration::from_millis(100));

    assert!(ev_out_rx
        .try_recv()
        .is_ok_and(|msg| { matches!(msg, texla_client::Output::Text(_)) }));
}
