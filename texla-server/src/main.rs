use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_ws_server::{ReceiveError, WsConnection, WsListener, WsPlugin};

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), WsPlugin))
        .add_systems(Startup, startup)
        .add_systems(Update, receive_message)
        .run();
}

fn startup(listener: Res<WsListener>) {
    debug!("Hello, world!");

    listener.listen("127.0.0.1:8080");
}

fn receive_message(mut commands: Commands, connections: Query<(Entity, &WsConnection)>) {
    for (entity, conn) in connections.iter() {
        loop {
            match conn.receive() {
                Ok(message) => {
                    debug!("> {message}");
                    conn.send(message.clone());
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    conn.send(message);
                }
                Err(ReceiveError::Empty) => break,
                Err(ReceiveError::Closed) => {
                    commands.entity(entity).despawn();
                    break;
                }
            }
        }
    }
}
