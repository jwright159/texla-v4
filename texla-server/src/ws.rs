use bevy::prelude::*;
use bevy_ws_server::{ReceiveError, WsConnection, WsListener};
use tungstenite::Message;

use crate::{Command, Connection, ConnectionMessageEvent, PreprocessCommandsSet};

pub struct WsPlugin;

impl Plugin for WsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_ws_server::WsPlugin)
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (observe_connections, receive_message)
                    .chain()
                    .after(bevy_ws_server::accept_ws_from_queue)
                    .before(PreprocessCommandsSet),
            );
    }
}

fn setup(listener: Res<WsListener>) {
    listener.listen("127.0.0.1:8080");
}

fn observe_connections(mut commands: Commands, listener: Query<Entity, Added<WsConnection>>) {
    for entity in listener.iter() {
        commands
            .entity(entity)
            .insert(Connection)
            .observe(send_message);
    }
}

fn receive_message(mut commands: Commands, conns: Query<(Entity, &WsConnection)>) {
    for (entity, conn) in conns.iter() {
        loop {
            match conn.receive() {
                Ok(Message::Text(message)) => {
                    debug!("{} <| {}", conn.id(), message);
                    commands.spawn(Command::from_str(message, entity));
                }
                Ok(_) => {}
                Err(ReceiveError::Empty) => break,
                Err(ReceiveError::Closed) => {
                    commands.entity(entity).despawn();
                    break;
                }
            }
        }
    }
}

fn send_message(trigger: Trigger<ConnectionMessageEvent>, conns: Query<&WsConnection>) {
    let conn = conns.get(trigger.entity()).unwrap();
    match &trigger.event().0 {
        Ok(message) | Err(message) => {
            conn.send(Message::Text(message.clone()));
        }
    }
}
