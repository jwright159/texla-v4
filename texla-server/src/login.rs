use bevy::prelude::*;

use crate::interact::{look, LookBundle};
use crate::prelude::*;
use crate::SpawnRoom;

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
    comms: Query<&PlayerCommand, With<LoginCommand>>,
    players: Query<(Entity, &Player)>,
) {
    for command in comms.iter() {
        if command.inner.args.len() < 2 {
            send(
                &mut commands,
                command.conn,
                Err("Usage: login <username> | <password>".to_owned()),
            );
            continue;
        }

        let username = command.inner.args[0].clone();
        let password = command.inner.args[1].clone();

        let Some((player_entity, _player)) = players
            .iter()
            .find(|(_, player)| player.username == username && player.password == password)
        else {
            send(
                &mut commands,
                command.conn,
                Err("Invalid username or password.".to_owned()),
            );
            continue;
        };

        commands.entity(command.conn).insert(PlayerConnection {
            object: player_entity,
        });

        send(
            &mut commands,
            command.conn,
            Ok("Successfully logged in.".to_owned()),
        );
    }
}

fn handle_register(
    mut commands: Commands,
    comms: Query<&PlayerCommand, With<RegisterCommand>>,
    players: Query<(Entity, &Player)>,
    spawn_room: Res<SpawnRoom>,
    looks: Query<LookBundle>,
) {
    for command in comms.iter() {
        if command.inner.args.len() < 2 {
            send(
                &mut commands,
                command.conn,
                Err("Usage: register <username> | <password>".to_owned()),
            );
            continue;
        }

        let username = command.inner.args[0].clone();
        let password = command.inner.args[1].clone();

        if players
            .iter()
            .any(|(_, player)| player.username == username)
        {
            send(
                &mut commands,
                command.conn,
                Err("Username already taken.".to_owned()),
            );
            continue;
        }

        let player_entity = commands
            .spawn((
                Object::default(),
                Player {
                    username: username.clone(),
                    password: password.clone(),
                },
            ))
            .set_parent(spawn_room.0)
            .id();
        commands.entity(command.conn).insert(PlayerConnection {
            object: player_entity,
        });

        let spawn_room_look = looks.get(spawn_room.0).unwrap();

        send(&mut commands, command.conn, Ok(look(spawn_room_look)));
    }
}

fn handle_logout(mut commands: Commands, comms: Query<&PlayerCommand, With<LogoutCommand>>) {
    for command in comms.iter() {
        commands.entity(command.conn).remove::<PlayerConnection>();
        send(
            &mut commands,
            command.conn,
            Ok("Successfully logged out.".to_owned()),
        );
    }
}

#[derive(Component, Debug, Default)]
pub struct RequiresLogin;

#[derive(Component, Debug, Default)]
pub struct RequiresNoLogin;

#[cfg(test)]
mod tests {
    use crate::login::LoginPlugin;
    use crate::PlayerCommand;

    #[test]
    fn register_works() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_ok()));
    }

    #[test]
    fn registering_with_existing_username_fails() {
        let (mut app, conn, rx, conns) = crate::test_app::<1>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conns[0],
        ));
        app.update();
        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }

    #[test]
    fn registering_with_no_username_or_password_fails() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut()
            .spawn((PlayerCommand::new("register", vec![], conn),));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }

    #[test]
    fn registering_when_logged_in_fails() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }

    #[test]
    fn logout_works() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("logout", vec![], conn));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_ok()));
    }

    #[test]
    fn logging_out_when_not_logged_in_fails() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut()
            .spawn(PlayerCommand::new("logout", vec![], conn));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }

    #[test]
    fn login_works() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("logout", vec![], conn));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("login", vec!["test", "password"], conn));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_ok()));
    }

    #[test]
    fn logging_in_with_no_username_or_password_fails() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("logout", vec![], conn));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("login", vec![], conn));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }

    #[test]
    fn logging_in_with_invalid_username_or_password_fails() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("logout", vec![], conn));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("login", vec!["test", "passwor"], conn));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }

    #[test]
    fn logging_in_when_already_logged_in_fails() {
        let (mut app, conn, rx, _conns) = crate::test_app::<0>();
        app.add_plugins(LoginPlugin);

        app.world_mut().spawn(PlayerCommand::new(
            "register",
            vec!["test", "password"],
            conn,
        ));
        app.update();
        rx.try_recv().unwrap();

        app.world_mut()
            .spawn(PlayerCommand::new("login", vec!["test", "password"], conn));
        app.update();

        assert!(rx.try_recv().is_ok_and(|msg| msg.0.is_err()));
    }
}
