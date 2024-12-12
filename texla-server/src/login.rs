use bevy::prelude::*;
use bevy_ws_server::WsConnection;
use tungstenite::Message;

use crate::{
    preprocess_commands, Command, CommandHandler, HandleCommandsSet, Player, PlayerConnection,
    PreprocessCommandsSet,
};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                (
                    preprocess_commands::<LoginCommand>,
                    preprocess_commands::<RegisterCommand>,
                    preprocess_commands::<LogoutCommand>,
                )
                    .in_set(PreprocessCommandsSet),
                (handle_login, handle_register, handle_logout).in_set(HandleCommandsSet),
            ),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        CommandHandler::<LoginCommand>::new("login"),
        RequiresNoLogin,
    ));
    commands.spawn((
        CommandHandler::<RegisterCommand>::new("register"),
        RequiresNoLogin,
    ));
    commands.spawn((
        CommandHandler::<LogoutCommand>::new("logout"),
        RequiresLogin,
    ));
}

#[derive(Component, Default)]
struct LoginCommand;

#[derive(Component, Default)]
struct RegisterCommand;

#[derive(Component, Default)]
struct LogoutCommand;

fn handle_login(
    mut commands: Commands,
    comms: Query<&Command, With<LoginCommand>>,
    conns: Query<&WsConnection>,
    players: Query<(Entity, &Player)>,
) {
    for command in comms.iter() {
        let conn = conns.get(command.conn).unwrap();

        if command.inner.args.len() < 2 {
            conn.send(Message::Text(
                "Usage: login <username> | <password>".to_owned(),
            ));
            continue;
        }

        let username = command.inner.args[0].clone();
        let password = command.inner.args[1].clone();

        let Some((player_entity, _player)) = players
            .iter()
            .find(|(_, player)| player.username == username && player.password == password)
        else {
            conn.send(Message::Text("Invalid username or password".to_owned()));
            continue;
        };

        commands.entity(command.conn).insert(PlayerConnection {
            object: player_entity,
        });

        conn.send(Message::Text("Successfully logged in".to_owned()));
    }
}

fn handle_register(
    mut commands: Commands,
    comms: Query<&Command, With<RegisterCommand>>,
    conns: Query<&WsConnection>,
    players: Query<(Entity, &Player)>,
) {
    for command in comms.iter() {
        let conn = conns.get(command.conn).unwrap();

        if command.inner.args.len() < 2 {
            conn.send(Message::Text(
                "Usage: register <username> | <password>".to_owned(),
            ));
            continue;
        }

        let username = command.inner.args[0].clone();
        let password = command.inner.args[1].clone();

        if players
            .iter()
            .any(|(_, player)| player.username == username)
        {
            conn.send(Message::Text("Username already taken".to_owned()));
            continue;
        }

        let player_entity = commands
            .entity(command.conn)
            .insert(Player {
                username: username.clone(),
                password: password.clone(),
            })
            .id();
        commands.entity(command.conn).insert(PlayerConnection {
            object: player_entity,
        });

        conn.send(Message::Text(
            "Successfully registered and logged in".to_owned(),
        ));
    }
}

fn handle_logout(
    mut commands: Commands,
    comms: Query<&Command, With<LogoutCommand>>,
    conns: Query<&WsConnection>,
) {
    for command in comms.iter() {
        let conn = conns.get(command.conn).unwrap();
        commands.entity(command.conn).remove::<PlayerConnection>();
        conn.send(Message::Text("Successfully logged out".to_owned()));
    }
}

#[derive(Component, Debug, Default)]
pub struct RequiresLogin;

#[derive(Component, Debug, Default)]
pub struct RequiresNoLogin;
