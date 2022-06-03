mod camera;

use camera::SimpleOrthoProjection;
use crystalorb_bevy_networking_turbulence::{
    bevy_networking_turbulence::{
        MessageChannelMode, MessageChannelSettings, NetworkResource, ReliableChannelSettings,
    },
    crystalorb::client::{stage::Stage as ClientStage, stage::StageMut as ClientStageMut, Client},
    CommandChannelSettings, CrystalOrbClientPlugin, WrappedNetworkResource,
};
use platformer_shared::{
    bevy,
    bevy::{
        prelude::*,
        render::camera::{Camera, VisibleEntities},
        utils::HashSet,
    },
    crystalorb_bevy_networking_turbulence::{self, bevy_networking_turbulence, crystalorb},
    game::{GameCommand, GameWorld, PlayerCommand, PlayerId, PlayerInput, PowerPadStatus},
    Position, BOTTOM_POWER_PAD_POSITIONS, BOTTOM_START_POSITION, LAVA_RECTS, PLATFORMS,
    POWER_PAD_SIZE, PROJECTILE_SIZE, STARTING_BOTTOM_POWER_PAD_POSITION,
    STARTING_TOP_POWER_PAD_POSITION, TOP_POWER_PAD_POSITIONS, TOP_START_POSITION,
};
use std::{net::SocketAddr, time::Duration};

fn player_input(
    mut state: Local<PlayerInput>,
    input: Res<Input<KeyCode>>,
    mut client: ResMut<Client<GameWorld>>,
    mut net: ResMut<NetworkResource>,
) {
    if let ClientStageMut::Ready(mut ready_client) = client.stage_mut() {
        // can a client impersonate another with this?
        let player_id = match ready_client.client_id() as u8 {
            0 => Some(PlayerId::Player1),
            1 => Some(PlayerId::Player2),
            _ => None,
        };

        if let Some(player_id) = player_id {
            let player_input = &PlayerInput {
                action: input.just_pressed(KeyCode::Space),
                left: input.pressed(KeyCode::Left),
                right: input.pressed(KeyCode::Right),
            };

            if player_input.action != state.action {
                ready_client.issue_command(
                    GameCommand::Input(player_id, PlayerCommand::Action, player_input.action),
                    &mut WrappedNetworkResource(&mut *net),
                );
            }
            if player_input.left != state.left {
                ready_client.issue_command(
                    GameCommand::Input(player_id, PlayerCommand::Left, player_input.left),
                    &mut WrappedNetworkResource(&mut *net),
                );
            }
            if player_input.right != state.right {
                ready_client.issue_command(
                    GameCommand::Input(player_id, PlayerCommand::Right, player_input.right),
                    &mut WrappedNetworkResource(&mut *net),
                );
            }
            *state = *player_input;
        }
    }
}

fn main() {
    use bevy::render::camera::camera_system;

    App::build()
        // You can optionally override some message channel settings
        // There is `CommandChannelSettings`, `SnapshotChannelSettings`, and `ClockSyncChannelSettings`
        // Make sure you apply the same settings for both client and server.
        .insert_resource(CommandChannelSettings(MessageChannelSettings {
            channel: 0,
            channel_mode: MessageChannelMode::Compressed {
                reliability_settings: ReliableChannelSettings {
                    bandwidth: 4096,
                    recv_window_size: 1024,
                    send_window_size: 1024,
                    burst_bandwidth: 1024,
                    init_send: 512,
                    wakeup_time: Duration::from_millis(100),
                    initial_rtt: Duration::from_millis(200),
                    max_rtt: Duration::from_secs(2),
                    rtt_update_factor: 0.1,
                    rtt_resend_factor: 1.5,
                },
                max_chunk_len: 1024,
            },
            message_buffer_size: 64,
            packet_buffer_size: 64,
        }))
        .insert_resource(WindowDescriptor {
            height: 1000.0,
            width: 1000.0,
            resizable: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_system_to_stage(
            CoreStage::PostUpdate,
            camera_system::<SimpleOrthoProjection>.system(),
        )
        .add_startup_system(setup_scene.system())
        .add_plugin(CrystalOrbClientPlugin::<GameWorld>::new(
            platformer_shared::crystal_orb_config(),
        ))
        .add_startup_system(setup_network.system())
        .add_system(player_input.system())
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_system(show_state.system())
        .add_system(projectile_view_lifecycle.system())
        .add_system(view.system())
        .run();
}

struct GameContext {
    player1: Entity,
    player2: Entity,
    cannon: Entity,
    bottom_power_pad: Entity,
    top_power_pad: Entity,
}

struct Projectile(u16);

fn setup_scene(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    // camera
    let projection = SimpleOrthoProjection::new(1000.0, 1000.0);
    let cam_name = bevy::render::render_graph::base::camera::CAMERA_2D;
    let camera = Camera {
        name: Some(cam_name.to_string()),
        ..Default::default()
    };

    commands.spawn_bundle((
        Transform::from_xyz(0.0, 0.0, projection.far - 0.1),
        GlobalTransform::default(),
        VisibleEntities::default(),
        camera,
        projection,
    ));

    // player 1
    let start_position = BOTTOM_START_POSITION;
    let size = Vec2::new(20.0, 20.0);
    let player1 = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::BLUE.into()),
            sprite: Sprite::new(size),
            transform: Transform::from_xyz(start_position.x, start_position.y, 0.0),
            ..Default::default()
        })
        .id();

    // player 2
    let start_position = TOP_START_POSITION;
    let size = Vec2::new(20.0, 20.0);
    let player2 = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::RED.into()),
            sprite: Sprite::new(size),
            transform: Transform::from_xyz(start_position.x, start_position.y, 0.0),
            ..Default::default()
        })
        .id();

    let size = Vec2::new(40.0, 40.0);
    let cannon = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::ORANGE_RED.into()),
            sprite: Sprite::new(size),
            transform: Transform::from_xyz(500.0, 500.0, 0.0),
            ..Default::default()
        })
        .id();

    for platform in PLATFORMS.iter() {
        for (x, y) in [
            (platform.x, platform.y),
            (1000.0 - platform.x, 1000.0 - platform.y),
        ]
        .iter()
        {
            let size = Vec2::new(platform.w, platform.h);
            commands.spawn_bundle(SpriteBundle {
                transform: Transform::from_xyz(*x, *y, 0.0),
                material: materials.add(Color::WHITE.into()),
                sprite: Sprite::new(size),
                ..Default::default()
            });
        }
    }

    for lava_rect in LAVA_RECTS.iter() {
        for (x, y) in [
            (lava_rect.x, lava_rect.y),
            (1000.0 - lava_rect.x, 1000.0 - lava_rect.y),
        ]
        .iter()
        {
            let size = Vec2::new(lava_rect.w, lava_rect.h);
            commands.spawn_bundle(SpriteBundle {
                transform: Transform::from_xyz(*x, *y, 1.0),
                material: materials.add(Color::ORANGE.into()),
                sprite: Sprite::new(size),
                ..Default::default()
            });
        }
    }

    let size = Vec2::new(POWER_PAD_SIZE.w, POWER_PAD_SIZE.h);
    let position = STARTING_BOTTOM_POWER_PAD_POSITION;
    let bottom_power_pad = commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(position.x, position.y, 2.0),
            material: materials.add(Color::CRIMSON.into()),
            sprite: Sprite::new(size),
            ..Default::default()
        })
        .id();
    let position = STARTING_TOP_POWER_PAD_POSITION;
    let top_power_pad = commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(position.x, position.y, 2.0),
            material: materials.add(Color::CRIMSON.into()),
            sprite: Sprite::new(size),
            ..Default::default()
        })
        .id();

    commands.insert_resource(GameContext {
        player1,
        player2,
        cannon,
        bottom_power_pad,
        top_power_pad,
    });
}

fn setup_network(mut net: ResMut<NetworkResource>) {
    let ip_address =
        bevy_networking_turbulence::find_my_ip_address().expect("can't find ip address");
    let socket_address = SocketAddr::new(ip_address, platformer_shared::SERVER_PORT);
    info!("Connecting to {}", socket_address);
    net.connect(socket_address);
}

fn show_state(mut previous: Local<String>, client: ResMut<Client<GameWorld>>) {
    use crystalorb::client::stage::Stage;
    let text = match client.stage() {
        Stage::SyncingClock(c) => {
            format!("SyncingClock {}/{}", c.sample_count(), c.samples_needed())
        }
        Stage::SyncingInitialState(_) => "SyncingInitialState".to_string(),
        Stage::Ready(_) => "Ready".to_string(),
    };
    if *previous != text {
        info!("State: {}", text);
        *previous = text;
    }
}

fn projectile_view_lifecycle(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    client: Res<Client<GameWorld>>,
    projectiles: Query<(Entity, &Projectile)>,
) {
    if let ClientStage::Ready(client) = client.stage() {
        // world is flipped for player 2
        let world_flipped = client.client_id() as u8 == 1;

        let display_state = client.display_state();

        let current_views = projectiles
            .iter()
            .map(|(_, p)| p.0)
            .collect::<HashSet<u16>>();
        let projectile_ids = display_state
            .projectile_positions
            .keys()
            .copied()
            .collect::<HashSet<u16>>();

        // remove destroyed ones
        let for_removal = current_views
            .difference(&projectile_ids)
            .copied()
            .collect::<HashSet<u16>>();
        projectiles.for_each(|(e, p)| {
            if for_removal.contains(&p.0) {
                commands.entity(e).despawn_recursive();
            }
        });

        // add newly spawned ones
        for projectile_id in projectile_ids.difference(&current_views) {
            let position = display_state
                .projectile_positions
                .get(&projectile_id)
                .unwrap()
                .translation
                .vector;

            let transform = if world_flipped {
                Transform::from_xyz(1000.0 - position.x, 1000.0 - position.y, 2.0)
            } else {
                Transform::from_xyz(position.x, position.y, 2.0)
            };

            commands
                .spawn_bundle(SpriteBundle {
                    transform,
                    material: materials.add(Color::ORANGE.into()),
                    sprite: Sprite::new(Vec2::new(PROJECTILE_SIZE.w, PROJECTILE_SIZE.h)),
                    ..Default::default()
                })
                .insert(Projectile(*projectile_id));
        }
    }
}

// helper
fn update_transform(transform: &mut Transform, x: f32, y: f32, world_flipped: bool) {
    if world_flipped {
        transform.translation = Vec3::new(1000.0 - x, 1000.0 - y, 0.0);
    } else {
        transform.translation = Vec3::new(x, y, 0.0);
    }
}

fn view(
    client: Res<Client<GameWorld>>,
    game_context: Res<GameContext>,
    mut q: QuerySet<(Query<&mut Transform>, Query<(&Projectile, &mut Transform)>)>,
) {
    if let ClientStage::Ready(client) = client.stage() {
        // world is flipped for player 2
        let world_flipped = client.client_id() as u8 == 1;

        let display_state = client.display_state();
        for (entity, pos) in [
            (game_context.player1, display_state.player1_position),
            (game_context.player2, display_state.player2_position),
        ]
        .iter()
        {
            let mut transform = q.q0_mut().get_mut(*entity).unwrap();

            update_transform(
                &mut transform,
                pos.translation.vector.x,
                pos.translation.vector.y,
                world_flipped,
            );
        }

        for (entity, status, positions) in [
            (
                game_context.bottom_power_pad,
                display_state.bottom_power_pad_status,
                BOTTOM_POWER_PAD_POSITIONS,
            ),
            (
                game_context.top_power_pad,
                display_state.top_power_pad_status,
                TOP_POWER_PAD_POSITIONS,
            ),
        ]
        .iter()
        {
            let mut transform = q.q0_mut().get_mut(*entity).unwrap();

            let Position { x, y } = match status {
                PowerPadStatus::Left => positions.left,
                PowerPadStatus::Right => positions.right,
            };

            update_transform(&mut transform, x, y, world_flipped);
        }

        let mut transform = q.q0_mut().get_mut(game_context.cannon).unwrap();
        let y = transform.translation.y;
        update_transform(
            &mut transform,
            display_state.cannon_x_position,
            y,
            world_flipped,
        );

        for (projectile, mut transform) in q.q1_mut().iter_mut() {
            // there might not be a projectile in the game state on the server anymore, even though there's still one in the ECS
            // these projectiles should get cleaned up in the next frame
            if let Some(position) = display_state.projectile_positions.get(&projectile.0) {
                let pos = position.translation.vector;
                update_transform(
                    &mut transform,
                    // TODO: correlation
                    pos.x,
                    pos.y,
                    world_flipped,
                );
            }
        }
    }
}
