use std::marker::PhantomData;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_ws_server::{ReceiveError, WsConnection, WsListener, WsPlugin};
use tungstenite::Message;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), WsPlugin))
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            (
                receive_message.before(PreprocessCommandsSet),
                (
                    preprocess_commands::<LoginCommand>,
                    preprocess_commands::<EchoCommand>,
                    preprocess_commands::<LookCommand>,
                )
                    .in_set(PreprocessCommandsSet),
                (handle_login, handle_echo, handle_look).in_set(HandleCommandsSet),
                clean_up_unhandled_commands.after(HandleCommandsSet),
            ),
        )
        .configure_sets(Update, PreprocessCommandsSet.before(HandleCommandsSet))
        .run();
}

fn startup(mut commands: Commands, listener: Res<WsListener>) {
    listener.listen("127.0.0.1:8080");

    commands.spawn((
        CommandHandler::<LoginCommand>::new("login"),
        RequiresNoLogin,
    ));
    commands.spawn((CommandHandler::<EchoCommand>::new("echo"),));
    commands.spawn((CommandHandler::<LookCommand>::new("look"), RequiresLogin));
}

fn receive_message(mut commands: Commands, conns: Query<(Entity, &WsConnection)>) {
    for (entity, conn) in conns.iter() {
        loop {
            match conn.receive() {
                Ok(Message::Text(message)) => {
                    debug!("{} <| {}", conn.id(), message);
                    commands.spawn(Command::new(message, entity));
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

fn clean_up_unhandled_commands(
    mut commands: Commands,
    comms: Query<(Entity, &Command)>,
    conns: Query<&WsConnection>,
) {
    for (entity, command) in comms.iter() {
        let conn = conns.get(command.conn).unwrap();
        match &command.state {
            CommandState::NotHandled => {
                conn.send(Message::Text(format!(
                    "Unknown command: {}",
                    command.inner.command
                )));
            }
            CommandState::Handled => {}
        }
        commands.entity(entity).despawn();
    }
}

fn preprocess_commands<T: Component + Default>(
    mut commands: Commands,
    handlers: Query<(
        &CommandHandler<T>,
        Option<&RequiresLogin>,
        Option<&RequiresNoLogin>,
    )>,
    mut comms: Query<(Entity, &mut Command)>,
    conns: Query<(&WsConnection, Option<&PlayerConnection>)>,
) {
    for (entity, mut command) in comms.iter_mut() {
        let Some((_, req_login, req_no_login)) = handlers
            .iter()
            .find(|(h, _, _)| h.command == command.inner.command)
        else {
            continue;
        };
        let (conn, logged_in) = conns.get(command.conn).unwrap();

        command.state = CommandState::Handled;

        if req_login.is_some() && logged_in.is_none() {
            conn.send(Message::Text("You must be logged in to do that".to_owned()));
            continue;
        }

        if req_no_login.is_some() && logged_in.is_some() {
            conn.send(Message::Text(
                "You must not be logged in to do that".to_owned(),
            ));
            continue;
        }

        commands.entity(entity).insert(T::default());
    }
}

fn handle_login(
    mut commands: Commands,
    comms: Query<&Command, With<LoginCommand>>,
    conns: Query<&WsConnection>,
) {
    for command in comms.iter() {
        commands.entity(command.conn).insert(PlayerConnection {
            object: command.conn,
        });
        let conn = conns.get(command.conn).unwrap();
        conn.send(Message::Text("Successfully logged in".to_owned()));
    }
}

fn handle_echo(comms: Query<&Command, With<EchoCommand>>, conns: Query<&WsConnection>) {
    for command in comms.iter() {
        let conn = conns.get(command.conn).unwrap();
        conn.send(Message::Text(command.inner.args.join(" ")));
    }
}

fn handle_look(comms: Query<&Command, With<LookCommand>>, conns: Query<&WsConnection>) {
    for command in comms.iter() {
        let conn = conns.get(command.conn).unwrap();
        conn.send(Message::Text(
            "You look around you... but nobody came.".to_owned(),
        ));
    }
}

#[derive(Component, Debug)]
pub struct PlayerConnection {
    pub object: Entity,
}

#[derive(Component, Debug, Default)]
pub struct Object;

#[derive(Component, Debug)]
pub struct Command {
    inner: CommandInner,
    state: CommandState,
    conn: Entity,
}

impl Command {
    pub fn new(str: String, conn: Entity) -> Self {
        Self {
            inner: CommandInner::from(str),
            state: CommandState::NotHandled,
            conn,
        }
    }
}

#[derive(Debug)]
pub enum CommandState {
    NotHandled,
    Handled,
}

#[derive(Debug)]
pub struct CommandInner {
    pub command: String,
    pub args: Vec<String>,
}

impl From<String> for CommandInner {
    fn from(str: String) -> Self {
        let (command, args) = str.split_once(" ").unwrap_or((&str, ""));
        let args = args.split("|").map(|s| s.trim().to_owned()).collect();
        Self {
            command: command.trim().to_owned(),
            args,
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct RequiresLogin;

#[derive(Component, Debug, Default)]
pub struct RequiresNoLogin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreprocessCommandsSet;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HandleCommandsSet;

#[derive(Component, Debug)]
pub struct CommandHandler<T> {
    command: String,
    _phantom: PhantomData<T>,
}

impl<T> CommandHandler<T> {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_owned(),
            _phantom: PhantomData,
        }
    }
}

#[derive(Event, Debug)]
pub struct CommandTrigger<T>(PhantomData<T>);

impl<T> Default for CommandTrigger<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Component, Default)]
pub struct LoginCommand;

#[derive(Component, Default)]
pub struct EchoCommand;

#[derive(Component, Default)]
pub struct LookCommand;
