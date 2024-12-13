use bevy::prelude::*;

use crate::prelude::*;

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                (preprocess_commands::<LookCommand>,).in_set(PreprocessCommandsSet),
                (handle_look,).in_set(HandleCommandsSet),
            ),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((CommandHandler::<LookCommand>::new("look"), RequiresLogin));
}

#[derive(Component, Default)]
struct LookCommand;

fn handle_look(
    mut commands: Commands,
    comms: Query<&PlayerCommand, With<LookCommand>>,
    conns: Query<&PlayerConnection>,
    player_parents: Query<&Parent, With<Player>>,
    looks: Query<LookBundle>,
) {
    for command in comms.iter() {
        let conn = conns.get(command.conn).unwrap();
        let player_parent = player_parents.get(conn.object).unwrap();
        let parent_look = looks.get(player_parent.get()).unwrap();
        send(&mut commands, command.conn, Ok(look(parent_look)));
    }
}

pub type LookBundle<'a> = (Entity, &'a Object, Option<&'a Name>);

pub fn look((entity, obj, name): LookBundle) -> String {
    let name = name
        .map(|n| n.to_string())
        .unwrap_or(format!("{:?}", entity));
    let description = obj.properties.get("description");

    if let Some(description) = description {
        format!("{}\n{}", name, description)
    } else {
        name
    }
}
