use bevy::prelude::*;

use crate::prelude::*;

pub struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                (preprocess_commands::<EchoCommand>,).in_set(PreprocessCommandsSet),
                (handle_echo,).in_set(HandleCommandsSet),
            ),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((CommandHandler::<EchoCommand>::new("echo"),));
}

#[derive(Component, Default)]
struct EchoCommand;

fn handle_echo(mut commands: Commands, comms: Query<&PlayerCommand, With<EchoCommand>>) {
    for command in comms.iter() {
        send(
            &mut commands,
            command.conn,
            Ok(command.inner.args.join("\n")),
        );
    }
}
