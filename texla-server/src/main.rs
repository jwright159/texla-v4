use std::marker::PhantomData;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use login::{RequiresLogin, RequiresNoLogin};

mod login;
mod ws;

fn main() {
    minimal_app()
        .add_plugins((MinimalPlugins, LogPlugin::default()))
        .add_plugins((login::LoginPlugin, ws::WsPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    preprocess_commands::<EchoCommand>,
                    preprocess_commands::<LookCommand>,
                )
                    .in_set(PreprocessCommandsSet),
                (handle_echo, handle_look).in_set(HandleCommandsSet),
            ),
        )
        .run();
}

fn minimal_app() -> App {
    let mut app = App::new();
    app.add_systems(
        Update,
        (clean_up_unhandled_commands.after(HandleCommandsSet),),
    )
    .configure_sets(Update, PreprocessCommandsSet.before(HandleCommandsSet));
    app
}

#[cfg(test)]
fn test_app<const NUM_EXTRA_CONNS: usize>() -> (
    App,
    Entity,
    std::sync::mpsc::Receiver<ConnectionMessageEvent>,
    [Entity; NUM_EXTRA_CONNS],
) {
    let mut app = minimal_app();
    let world = app.world_mut();
    let (tx, rx) = std::sync::mpsc::channel();
    let conn = world
        .spawn((Connection,))
        .observe(move |trigger: Trigger<ConnectionMessageEvent>| {
            tx.send(trigger.event().clone()).unwrap()
        })
        .id();
    let conns = [0; NUM_EXTRA_CONNS].map(|_| world.spawn((Connection,)).id());

    (app, conn, rx, conns)
}

fn setup(mut commands: Commands) {
    commands.spawn((CommandHandler::<EchoCommand>::new("echo"),));
    commands.spawn((CommandHandler::<LookCommand>::new("look"), RequiresLogin));
}

pub fn send(commands: &mut Commands, conn: Entity, message: Result<String, String>) {
    commands
        .entity(conn)
        .trigger(ConnectionMessageEvent(message));
}

fn clean_up_unhandled_commands(mut commands: Commands, comms: Query<(Entity, &Command)>) {
    for (entity, command) in comms.iter() {
        match &command.state {
            CommandState::NotHandled => {
                send(
                    &mut commands,
                    command.conn,
                    Err(format!("Unknown command: {}", command.inner.command)),
                );
            }
            CommandState::Handled => {}
        }
        commands.entity(entity).despawn();
    }
}

pub fn preprocess_commands<T: Component + Default>(
    mut commands: Commands,
    handlers: Query<(
        &CommandHandler<T>,
        Option<&RequiresLogin>,
        Option<&RequiresNoLogin>,
    )>,
    mut comms: Query<(Entity, &mut Command)>,
    conns: Query<Option<&PlayerConnection>, With<Connection>>,
) {
    for (entity, mut command) in comms.iter_mut() {
        let Some((_, req_login, req_no_login)) = handlers
            .iter()
            .find(|(h, _, _)| h.command == command.inner.command)
        else {
            continue;
        };
        let logged_in = conns.get(command.conn).unwrap();

        command.state = CommandState::Handled;

        if req_login.is_some() && logged_in.is_none() {
            send(
                &mut commands,
                command.conn,
                Err("You must be logged in to do that.".to_owned()),
            );
            continue;
        }

        if req_no_login.is_some() && logged_in.is_some() {
            send(
                &mut commands,
                command.conn,
                Err("You must not be logged in to do that.".to_owned()),
            );
            continue;
        }

        commands.entity(entity).insert(T::default());
    }
}

fn handle_echo(mut commands: Commands, comms: Query<&Command, With<EchoCommand>>) {
    for command in comms.iter() {
        send(
            &mut commands,
            command.conn,
            Ok(command.inner.args.join("\n")),
        );
    }
}

fn handle_look(mut commands: Commands, comms: Query<&Command, With<LookCommand>>) {
    for command in comms.iter() {
        send(
            &mut commands,
            command.conn,
            Err("You look around you... but nobody came.".to_owned()),
        );
    }
}

#[derive(Component, Debug, Default)]
pub struct Connection;

#[derive(Event, Debug, Clone)]
pub struct ConnectionMessageEvent(pub Result<String, String>);

#[derive(Component, Debug)]
pub struct PlayerConnection {
    pub object: Entity,
}

#[derive(Component, Debug)]
pub struct Player {
    pub username: String,
    pub password: String,
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
    pub fn new(command: &str, args: Vec<&str>, conn: Entity) -> Self {
        Self {
            inner: CommandInner {
                command: command.to_owned(),
                args: args.into_iter().map(|s| s.to_owned()).collect(),
            },
            state: CommandState::NotHandled,
            conn,
        }
    }

    pub fn from_str(str: String, conn: Entity) -> Self {
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
struct EchoCommand;

#[derive(Component, Default)]
struct LookCommand;
